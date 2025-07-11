/*****************************************************************************
 *   Ledger App Boilerplate Rust.
 *   (c) 2023 Ledger SAS.
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 *****************************************************************************/
use crate::app_ui::sign::{show_signing_spinner, ui_display_tx};
use crate::handlers::sign_message::schnorr_sign;
use crate::utils::{Bip32Path, CoinType};
use crate::{AppSW, DataContext, P1SignTx};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use ledger_device_sdk::ecc::{Secp256k1, SeedDerive};
use ledger_device_sdk::hash::{blake2::Blake2b_512, HashInit};
use ledger_device_sdk::io::Comm;
use ledger_device_sdk::nbgl::NbglSpinner;

use ledger_secure_sdk_sys::*;

use ml_common::{
    AccountCommand, AccountSpending, Amount, OrderAccountCommand, OutputValue,
    SighashInputCommitment, TxInput, TxOutput, H256,
};
use parity_scale_codec::{Compact, Decode, DecodeAll, Encode};

const MAX_TRANSACTION_LEN: usize = 510;

#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub enum CoinOrTokenId {
    Coin,
    TokenId(H256),
}

fn into_coin_or_token_id_and_amount(value: &OutputValue) -> Result<(CoinOrTokenId, Amount), AppSW> {
    match value {
        OutputValue::Coin(amount) => Ok((CoinOrTokenId::Coin, *amount)),
        OutputValue::TokenV0 => Err(AppSW::TxInvalidTokenV0),
        OutputValue::TokenV1(token_id, amount) => Ok((CoinOrTokenId::TokenId(*token_id), *amount)),
    }
}

#[derive(Decode)]
pub struct Input {
    pub path: Option<Bip32Path>,
}

pub enum InputData {
    Utxo,
    DelegationWithdrawl,
    MintTokens,
    UnmintTokens,
    FreezeToken,
    UnfreezeToken,
    LockTokenSupply,
    ChangeTokenAuthority,
    ChangeTokenMetadataUri,
    FillOrderV1(Amount),
    FreezeOrderV1,
    ConcludeOrderV1,
}

pub struct TxContext {
    raw_buf: Vec<u8>,
    pub coin: CoinType,
    num_inputs: u32,
    num_outputs: u32,
    review_finished: bool,

    tx_hasher: Blake2b_512,

    pub total_inputs: BTreeMap<CoinOrTokenId, Amount>,
    pub total_outputs: BTreeMap<CoinOrTokenId, Amount>,
    inputs: Vec<Input>,
    inputs_data: Vec<InputData>,
    commitments: Vec<SighashInputCommitment>,
    pub outputs: Vec<TxOutput>,

    num_signed_inputs: usize,
    spinner: Option<NbglSpinner>,
}

enum TxStatus {
    Incomplete,
    CompleteNotApproved,
    ApprovedNotFinishedSigning,
    Finished,
}

// Implement constructor for TxInfo with default values
impl TxContext {
    // Constructor
    pub fn new(coin: CoinType, version: u8, num_inputs: u32, num_outputs: u32) -> Result<TxContext, AppSW> {
        let mut tx_hasher = Blake2b_512::new();
        // mode
        tx_hasher.update(b"\x01").map_err(|_| AppSW::TxHashFail)?;
        // version
        tx_hasher
            .update(&[version])
            .map_err(|_| AppSW::TxHashFail)?;
        // flags
        tx_hasher.update(&[0; 16]).map_err(|_| AppSW::TxHashFail)?;

        Ok(TxContext {
            coin,
            raw_buf: Vec::new(),
            num_inputs,
            num_outputs,
            review_finished: false,
            tx_hasher,

            total_inputs: Default::default(),
            total_outputs: Default::default(),
            inputs: Default::default(),
            inputs_data: Default::default(),
            commitments: Default::default(),
            outputs: Default::default(),
            num_signed_inputs: 0,
            spinner: None,
        })
    }

    fn update_hash(&mut self) -> Result<(), AppSW> {
        self.tx_hasher
            .update(self.raw_buf.as_slice())
            .map_err(|_| AppSW::TxHashFail)?;

        self.raw_buf.clear();
        Ok(())
    }

    fn status(&self) -> TxStatus {
        let completed = self.num_inputs as usize == self.inputs.len()
            && self.num_inputs as usize == self.commitments.len()
            && self.num_outputs as usize == self.outputs.len();

        if !completed {
            return TxStatus::Incomplete;
        }

        if !self.approved() {
            return TxStatus::CompleteNotApproved;
        }

        if !self.completed_all_signatures() {
            return TxStatus::ApprovedNotFinishedSigning;
        }

        TxStatus::Finished
    }

    fn completed_all_signatures(&self) -> bool {
        self.num_signed_inputs == self.inputs.len()
    }

    // The transaction was approved if we already have returned a signature back
    fn approved(&self) -> bool {
        self.num_signed_inputs > 0
    }

    // Get review status
    #[allow(dead_code)]
    pub fn finished(&self) -> bool {
        self.review_finished
    }
}

pub fn handler_sign_tx(
    comm: &mut Comm,
    p1: P1SignTx,
    data_type: u8,
    ctx: &mut DataContext,
) -> Result<(), AppSW> {
    // Try to get data from comm
    let data = comm.get_data().map_err(|_| AppSW::WrongApduLength)?;
    // First chunk, try to parse the path
    if p1 == P1SignTx::Metadata {
        // Reset transaction context
        if data.len() != 10 {
            return Err(AppSW::WrongApduLength);
        }

        let coin = data[0].try_into()?;
        let version = u8::from_be_bytes(data[1..2].try_into().unwrap());
        let num_inputs = u32::from_be_bytes(data[2..6].try_into().unwrap());
        let num_outputs = u32::from_be_bytes(data[6..10].try_into().unwrap());

        let tx_ctx = TxContext::new(coin, version, num_inputs, num_outputs)?;
        *ctx = DataContext::TxContext(tx_ctx);
        Ok(())
    // Next chunks, append data to raw_tx and return or parse
    // the transaction if it is the last chunk.
    } else {
        let ctx = match ctx {
            DataContext::TxContext(ctx) => ctx,
            _ => return Err(AppSW::WrongContext),
        };

        if ctx.raw_buf.len() + data.len() > MAX_TRANSACTION_LEN {
            return Err(AppSW::TxWrongLength);
        }

        if data_type == 0 {
            // get path
            let inp = Input::decode_all(&mut &data[..]).map_err(|_| AppSW::TxDeserializeFail)?;

            ctx.inputs.push(inp);
            return Ok(());
        }

        // Append data to raw_tx
        ctx.raw_buf.extend(data);

        // If we expect more chunks, return
        if data_type == 1 {
            ctx.review_finished = false;
            return Ok(());
        }

        match p1 {
            P1SignTx::Input => {
                process_input(ctx)?;
                ctx.update_hash()?;
            }
            P1SignTx::InputCommitement => {
                process_input_commitement(ctx)?;
                ctx.update_hash()?;
            }
            P1SignTx::Output => {
                process_output(ctx)?;
                ctx.update_hash()?;
            }
            P1SignTx::NextSignature => {
                // continue
            }
            _ => return Err(AppSW::WrongContext),
        };

        match ctx.status() {
            TxStatus::Incomplete => {
                ctx.review_finished = false;
                Ok(())
            }
            TxStatus::CompleteNotApproved => {
                // Display transaction. If user approves
                // the transaction, sign it. Otherwise,
                // return a "deny" status word.
                if ui_display_tx(ctx)? {
                    compute_signature_and_append(comm, ctx)?;
                    if ctx.completed_all_signatures() {
                        ctx.review_finished = true;
                    } else {
                        ctx.review_finished = false;
                        show_signing_spinner(ctx.spinner.get_or_insert_with(NbglSpinner::new));
                    }

                    Ok(())
                } else {
                    ctx.review_finished = true;
                    Err(AppSW::Deny)
                }
            }
            TxStatus::ApprovedNotFinishedSigning => {
                // Allready approved sign and return the next signature
                compute_signature_and_append(comm, ctx)?;
                if ctx.completed_all_signatures() {
                    ctx.review_finished = true;
                }

                Ok(())
            }
            TxStatus::Finished => Err(AppSW::WrongContext),
        }
    }
}

fn process_output(ctx: &mut TxContext) -> Result<(), AppSW> {
    let out = ml_common::TxOutput::decode_all(&mut ctx.raw_buf.as_slice())
        .map_err(|_| AppSW::TxDeserializeFail)?;
    match &out {
        TxOutput::Transfer(value, _)
        | TxOutput::LockThenTransfer(value, _, _)
        | TxOutput::Burn(value)
        | TxOutput::Htlc(value, _) => {
            let (coin_or_token_id, amount) = into_coin_or_token_id_and_amount(value)?;
            increase_output_totals(&mut ctx.total_outputs, coin_or_token_id, amount)?;
        }
        TxOutput::CreateStakePool(_, data) => {
            increase_output_totals(&mut ctx.total_outputs, CoinOrTokenId::Coin, data.pledge)?;
        }
        TxOutput::ProduceBlockFromStake(_, _) => {}
        TxOutput::DelegateStaking(amount, _) => {
            increase_output_totals(&mut ctx.total_outputs, CoinOrTokenId::Coin, *amount)?;
        }
        TxOutput::CreateDelegationId(_, _)
        | TxOutput::IssueFungibleToken(_)
        | TxOutput::DataDeposit(_)
        | TxOutput::IssueNft(_, _, _)
        | TxOutput::CreateOrder(_) => {}
    }
    ctx.outputs.push(out);
    if ctx.commitments.len() == 1 {
        ctx.tx_hasher
            .update(&Compact::<u32>::encode(&ctx.num_outputs.into()))
            .map_err(|_| AppSW::TxHashFail)?;
    }
    Ok(())
}

fn process_input_commitement(ctx: &mut TxContext) -> Result<(), AppSW> {
    let inp_data = ctx
        .inputs_data
        .get(ctx.commitments.len())
        .ok_or(AppSW::WrongContext)?;
    let commitment = SighashInputCommitment::decode_all(&mut ctx.raw_buf.as_slice())
        .map_err(|_| AppSW::TxDeserializeFail)?;
    match &commitment {
        SighashInputCommitment::None => {}
        SighashInputCommitment::Utxo(utxo) => match &utxo {
            TxOutput::Transfer(value, _)
            | TxOutput::LockThenTransfer(value, _, _)
            | TxOutput::Htlc(value, _) => {
                let (coin_or_token_id, amount) = into_coin_or_token_id_and_amount(value)?;
                increase_input_totals(&mut ctx.total_inputs, coin_or_token_id, amount)?;
            }
            TxOutput::Burn(_)
            | TxOutput::ProduceBlockFromStake(_, _)
            | TxOutput::CreateDelegationId(_, _)
            | TxOutput::IssueFungibleToken(_)
            | TxOutput::DataDeposit(_)
            | TxOutput::DelegateStaking(_, _)
            | TxOutput::CreateOrder(_) => return Err(AppSW::TxInvalidInputUtxo),
            TxOutput::CreateStakePool(_, data) => {
                increase_input_totals(&mut ctx.total_inputs, CoinOrTokenId::Coin, data.pledge)?;
            }
            TxOutput::IssueNft(nft_id, _, _) => {
                increase_input_totals(
                    &mut ctx.total_inputs,
                    CoinOrTokenId::TokenId(*nft_id),
                    Amount::from_atoms(1),
                )?;
            }
        },
        SighashInputCommitment::ProduceBlockFromStakeUtxo {
            utxo: _,
            staker_balance,
        } => {
            increase_input_totals(&mut ctx.total_inputs, CoinOrTokenId::Coin, *staker_balance)?;
        }
        SighashInputCommitment::FillOrderAccountCommand {
            initially_asked,
            initially_given,
        } => match &inp_data {
            InputData::FillOrderV1(fill_amount) => {
                let (fill_coin_or_token_id, asked_amount) =
                    into_coin_or_token_id_and_amount(initially_asked)?;
                let (given_coin_or_token_id, given_amount) =
                    into_coin_or_token_id_and_amount(initially_given)?;

                increase_output_totals(
                    &mut ctx.total_outputs,
                    fill_coin_or_token_id,
                    *fill_amount,
                )?;

                let atoms = given_amount
                    .into_atoms()
                    .checked_mul(fill_amount.into_atoms())
                    .ok_or(AppSW::TxNumericOperationFail)?
                    .checked_div(asked_amount.into_atoms())
                    .ok_or(AppSW::TxNumericOperationFail)?;
                let amount = Amount::from_atoms(atoms);
                increase_input_totals(&mut ctx.total_inputs, given_coin_or_token_id, amount)?;
            }
            _ => return Err(AppSW::WrongContext),
        },
        SighashInputCommitment::ConcludeOrderAccountCommand {
            initially_asked,
            initially_given,
            ask_balance,
            give_balance,
        } => {
            let (coin_or_token_id, _) = into_coin_or_token_id_and_amount(initially_asked)?;
            increase_input_totals(&mut ctx.total_inputs, coin_or_token_id, *ask_balance)?;

            let (coin_or_token_id, _) = into_coin_or_token_id_and_amount(initially_given)?;
            increase_input_totals(&mut ctx.total_inputs, coin_or_token_id, *give_balance)?;
        }
    }
    ctx.commitments.push(commitment);
    if ctx.commitments.len() == 1 {
        ctx.tx_hasher
            .update(&ctx.num_inputs.to_le_bytes())
            .map_err(|_| AppSW::TxHashFail)?;
    }

    Ok(())
}

fn process_input(ctx: &mut TxContext) -> Result<(), AppSW> {
    let inp =
        TxInput::decode_all(&mut ctx.raw_buf.as_slice()).map_err(|_| AppSW::TxDeserializeFail)?;
    let input_data = match inp {
        TxInput::Utxo(_) => InputData::Utxo,
        TxInput::Account(acc) => match acc.account {
            AccountSpending::DelegationBalance(_, amount) => {
                increase_input_totals(&mut ctx.total_inputs, CoinOrTokenId::Coin, amount)?;
                InputData::DelegationWithdrawl
            }
        },
        TxInput::AccountCommand(_, cmd) => match cmd {
            AccountCommand::MintTokens(token_id, amount) => {
                increase_input_totals(
                    &mut ctx.total_inputs,
                    CoinOrTokenId::TokenId(token_id),
                    amount,
                )?;
                InputData::MintTokens
            }
            AccountCommand::ConcludeOrder(_) => return Err(AppSW::TxUnsupportedInput),
            AccountCommand::FillOrder(_, _, _) => return Err(AppSW::TxUnsupportedInput),
            AccountCommand::UnmintTokens(_) => InputData::UnmintTokens,
            AccountCommand::LockTokenSupply(_) => InputData::LockTokenSupply,
            AccountCommand::FreezeToken(_, _) => InputData::FreezeToken,
            AccountCommand::UnfreezeToken(_) => InputData::UnfreezeToken,
            AccountCommand::ChangeTokenAuthority(_, _) => InputData::ChangeTokenAuthority,
            AccountCommand::ChangeTokenMetadataUri(_, _) => InputData::ChangeTokenMetadataUri,
        },
        TxInput::OrderAccountCommand(cmd) => match cmd {
            OrderAccountCommand::FillOrder(_, fill_amount, _) => {
                InputData::FillOrderV1(fill_amount)
            }
            OrderAccountCommand::ConcludeOrder(_) => InputData::ConcludeOrderV1,
            OrderAccountCommand::FreezeOrder(_) => InputData::FreezeOrderV1,
        },
    };
    ctx.inputs_data.push(input_data);
    if ctx.inputs.len() == 1 {
        ctx.tx_hasher
            .update(&ctx.num_inputs.to_le_bytes())
            .map_err(|_| AppSW::TxHashFail)?;
    }

    Ok(())
}

fn increase_input_totals(
    total_inputs: &mut BTreeMap<CoinOrTokenId, Amount>,
    key: CoinOrTokenId,
    amount: Amount,
) -> Result<(), AppSW> {
    let total = total_inputs.entry(key).or_insert(Amount::from_atoms(0));
    let new_total = total
        .into_atoms()
        .checked_add(amount.into_atoms())
        .ok_or(AppSW::TxNumericOperationFail)?;
    *total = Amount::from_atoms(new_total);
    Ok(())
}

fn increase_output_totals(
    total_outputs: &mut BTreeMap<CoinOrTokenId, Amount>,
    key: CoinOrTokenId,
    amount: Amount,
) -> Result<(), AppSW> {
    let total = total_outputs.entry(key).or_insert(Amount::from_atoms(0));
    let new_total = total
        .into_atoms()
        .checked_add(amount.into_atoms())
        .ok_or(AppSW::TxNumericOperationFail)?;
    *total = Amount::from_atoms(new_total);
    Ok(())
}

fn compute_signature_and_append(comm: &mut Comm, ctx: &mut TxContext) -> Result<(), AppSW> {
    let mut message_hash: [u8; 64] = [0u8; 64];
    ctx.tx_hasher
        .finalize(&mut message_hash)
        .map_err(|_| AppSW::TxHashFail)?;

    let mut blake2b256 = Blake2b_512::new();
    let mut message_hash2: [u8; 64] = [0u8; 64];
    blake2b256
        .hash(&message_hash[0..32], &mut message_hash2)
        .map_err(|_| AppSW::TxHashFail)?;

    let hash_algorithm_id = CX_SHA256;
    let signing_mode = CX_ECSCHNORR_BIP0340 | CX_RND_TRNG | CX_LAST;

    if let Some(path) = ctx.inputs[ctx.num_signed_inputs].path.as_ref() {
        let private_key = Secp256k1::derive_from_path(path.as_ref());
        let (sig, siglen) = schnorr_sign(
            &private_key,
            &message_hash2[0..32],
            hash_algorithm_id,
            signing_mode,
        )?;

        comm.append(&[siglen as u8]);
        comm.append(&sig[..siglen as usize]);
        ctx.num_signed_inputs += 1;

        Ok(())
    } else {
        comm.append(&[0]);
        ctx.num_signed_inputs += 1;

        Ok(())
    }
}
