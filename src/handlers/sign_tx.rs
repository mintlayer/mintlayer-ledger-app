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
use crate::app_ui::sign::ui_display_tx;
use crate::handlers::sign_message::schnorr_sign;
use crate::utils::{Bip32Path, CoinType};
use crate::{AppSW, DataContext, P1SignTx, P2_SIGN_TX_LAST, P2_SIGN_TX_MORE};
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

#[derive(Encode)]
pub struct Signature {
    pub signature: [u8; 64],
    pub multisig_idx: Option<u32>,
}

#[derive(Decode)]
pub struct Input {
    pub addresses: Vec<InputAddressPath>,
}

#[derive(Decode)]
pub struct InputAddressPath {
    pub path: Bip32Path,
    pub multisig_idx: Option<u32>,
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
    FillOrderV0(Amount),
    ConcludeOrderV0,
    FillOrderV1(Amount),
    FreezeOrderV1,
    ConcludeOrderV1,
}

#[derive(PartialEq, Eq)]
pub enum TxParsingState {
    Input(usize),
    InputCommitement(usize),
    Output(usize),
    CompleteNotApproved {
        inp_idx: usize,
        sig_idx: usize,
        sighash: [u8; 32],
    },
    ApprovedNotFinishedSigning {
        inp_idx: usize,
        sig_idx: usize,
        sighash: [u8; 32],
    },
    Finished,
}

pub struct TxContext {
    raw_buf: Vec<u8>,
    pub coin: CoinType,
    num_inputs: u32,
    num_outputs: u32,
    review_finished: bool,
    state: TxParsingState,

    tx_hasher: Blake2b_512,

    pub total_inputs: BTreeMap<CoinOrTokenId, Amount>,
    pub total_outputs: BTreeMap<CoinOrTokenId, Amount>,
    inputs: Vec<Input>,
    inputs_data: Vec<InputData>,
    num_prcessed_input_commitments: u32,
    pub outputs: Vec<TxOutput>,

    spinner: NbglSpinner,
}

enum SigningState {
    TxParsingNotComplete,
    CompleteNotApproved {
        inp_idx: usize,
        sig_idx: usize,
        sighash: [u8; 32],
    },
    ApprovedNotFinishedSigning {
        inp_idx: usize,
        sig_idx: usize,
        sighash: [u8; 32],
    },
}

// Implement constructor for TxInfo with default values
impl TxContext {
    // Constructor
    pub fn new(
        coin: CoinType,
        version: u8,
        num_inputs: u32,
        num_outputs: u32,
    ) -> Result<TxContext, AppSW> {
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
            raw_buf: Vec::with_capacity(251),
            num_inputs,
            num_outputs,
            review_finished: false,
            tx_hasher,
            state: TxParsingState::Input(0),

            total_inputs: Default::default(),
            total_outputs: Default::default(),
            inputs: Vec::with_capacity(25),
            inputs_data: Vec::with_capacity(25),
            num_prcessed_input_commitments: 0,
            outputs: Default::default(),
            spinner: NbglSpinner::new(),
        })
    }

    fn update_hash(&mut self) -> Result<(), AppSW> {
        self.tx_hasher
            .update(self.raw_buf.as_slice())
            .map_err(|_| AppSW::TxHashFail)?;

        self.raw_buf.clear();
        Ok(())
    }

    fn signing_state(&self) -> Result<SigningState, AppSW> {
        let state = match self.state {
            TxParsingState::Input(_)
            | TxParsingState::InputCommitement(_)
            | TxParsingState::Output(_) => SigningState::TxParsingNotComplete,
            TxParsingState::CompleteNotApproved {
                inp_idx,
                sig_idx,
                sighash,
            } => SigningState::CompleteNotApproved {
                inp_idx,
                sig_idx,
                sighash,
            },
            TxParsingState::ApprovedNotFinishedSigning {
                inp_idx,
                sig_idx,
                sighash,
            } => SigningState::ApprovedNotFinishedSigning {
                inp_idx,
                sig_idx,
                sighash,
            },
            TxParsingState::Finished => return Err(AppSW::TxFinished),
        };
        Ok(state)
    }

    fn completed_all_signatures(&self) -> bool {
        self.state == TxParsingState::Finished
    }

    // Get review status
    #[allow(dead_code)]
    pub fn finished(&self) -> bool {
        self.review_finished
    }

    // Move to the next transaction processing state, and return the signing state
    fn next_step(&mut self) -> Result<(), AppSW> {
        self.state = match self.state {
            TxParsingState::Input(n) => {
                if n < (self.num_inputs - 1) as usize {
                    TxParsingState::Input(n + 1)
                } else {
                    TxParsingState::InputCommitement(0)
                }
            }
            TxParsingState::InputCommitement(n) => {
                if n < (self.num_inputs - 1) as usize {
                    TxParsingState::InputCommitement(n + 1)
                } else {
                    TxParsingState::Output(0)
                }
            }
            TxParsingState::Output(n) => {
                if n < (self.num_outputs - 1) as usize {
                    TxParsingState::Output(n + 1)
                } else {
                    let next = self.next_input_idx_to_sign(0, None);
                    if let Some((inp_idx, sig_idx)) = next {
                        // Finalize the tx hash for signing
                        let mut message_hash: [u8; 64] = [0u8; 64];
                        self.tx_hasher
                            .finalize(&mut message_hash)
                            .map_err(|_| AppSW::TxHashFail)?;

                        let mut blake2b256 = Blake2b_512::new();
                        let mut message_hash2: [u8; 64] = [0u8; 64];
                        blake2b256
                            .hash(&message_hash[0..32], &mut message_hash2)
                            .map_err(|_| AppSW::TxHashFail)?;

                        TxParsingState::CompleteNotApproved {
                            inp_idx,
                            sig_idx,
                            sighash: message_hash2[..32]
                                .try_into()
                                .map_err(|_| AppSW::TxHashFail)?,
                        }
                    } else {
                        return Err(AppSW::NothingToSign);
                    }
                }
            }
            TxParsingState::CompleteNotApproved {
                inp_idx,
                sig_idx,
                sighash,
            } => {
                let next = self.next_input_idx_to_sign(inp_idx, Some(sig_idx));
                if let Some((inp_idx, sig_idx)) = next {
                    TxParsingState::ApprovedNotFinishedSigning {
                        inp_idx,
                        sig_idx,
                        sighash,
                    }
                } else {
                    TxParsingState::Finished
                }
            }
            TxParsingState::ApprovedNotFinishedSigning {
                inp_idx,
                sig_idx,
                sighash,
            } => {
                let next = self.next_input_idx_to_sign(inp_idx, Some(sig_idx));
                if let Some((inp_idx, sig_idx)) = next {
                    TxParsingState::ApprovedNotFinishedSigning {
                        inp_idx,
                        sig_idx,
                        sighash,
                    }
                } else {
                    TxParsingState::Finished
                }
            }
            TxParsingState::Finished => return Err(AppSW::TxFinished),
        };

        Ok(())
    }

    fn next_input_idx_to_sign(
        &mut self,
        current_inp_idx: usize,
        current_sig_idx: Option<usize>,
    ) -> Option<(usize, usize)> {
        let next = self
            .inputs
            .iter()
            .enumerate()
            .flat_map(|(inp_idx, inp)| {
                inp.addresses
                    .iter()
                    .enumerate()
                    .map(move |(sig_idx, _)| (inp_idx, sig_idx))
            })
            .find_map(|(inp_idx, sig_idx)| {
                let is_next_input = inp_idx > current_inp_idx;
                let is_next_sig_for_same_input =
                    inp_idx == current_inp_idx && current_sig_idx.is_none_or(|idx| sig_idx > idx);

                if is_next_input || is_next_sig_for_same_input {
                    Some((inp_idx, sig_idx))
                } else {
                    None
                }
            });
        next
    }

    fn check_state(&self, p1: P1SignTx) -> Result<(), AppSW> {
        match (p1, &self.state) {
            (P1SignTx::Input, TxParsingState::Input(_))
            | (P1SignTx::InputCommitement, TxParsingState::InputCommitement(_))
            | (P1SignTx::Output, TxParsingState::Output(_))
            | (
                P1SignTx::NextSignature,
                TxParsingState::ApprovedNotFinishedSigning {
                    inp_idx: _,
                    sig_idx: _,
                    sighash: _,
                },
            ) => Ok(()),
            (_, _) => Err(AppSW::WrongP1P2),
        }
    }

    // show a spinnger for bigger transactions
    fn show_spinner(&mut self) {
        let is_transaction_big = self.num_inputs * 2 + self.num_outputs > 10;
        let returning_signatures = match self.state {
            TxParsingState::ApprovedNotFinishedSigning { inp_idx: _, sig_idx: _, sighash: _ }
            | TxParsingState::CompleteNotApproved { inp_idx: _, sig_idx: _, sighash: _ } => true,
            TxParsingState::Input(_)
            | TxParsingState::Finished
            | TxParsingState::InputCommitement(_)
            | TxParsingState::Output(_) => false
        };

        if returning_signatures && self.num_inputs > 1 {
            self.spinner.show("Signing...");
        } else if is_transaction_big {
            self.spinner.show("Parsing transaction...");
        }
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

        let mut tx_ctx = TxContext::new(coin, version, num_inputs, num_outputs)?;

        tx_ctx.show_spinner();

        *ctx = DataContext::TxContext(tx_ctx);
        Ok(())
    // Next chunks, append data to raw_tx and return or parse
    // the transaction if it is the last chunk.
    } else {
        let ctx = match ctx {
            DataContext::TxContext(ctx) => ctx,
            _ => return Err(AppSW::WrongContext),
        };

        ctx.show_spinner();

        if ctx.raw_buf.len() + data.len() > MAX_TRANSACTION_LEN {
            return Err(AppSW::TxWrongLength);
        }

        // Append data to raw_tx
        ctx.raw_buf.extend(data);

        // If we expect more chunks, return
        if data_type == P2_SIGN_TX_MORE {
            return Ok(());
        }

        ctx.check_state(p1)?;

        let signing_state = match ctx.state {
            TxParsingState::Input(n) => {
                if ctx.inputs.len() == n {
                    let inp = Input::decode_all(&mut ctx.raw_buf.as_slice())
                        .map_err(|_| AppSW::DeserializeFail)?;

                    ctx.inputs.push(inp);
                    ctx.raw_buf.clear();
                    return Ok(());
                } else {
                    process_input(ctx)?;
                    ctx.update_hash()?;
                    ctx.next_step()?;
                    ctx.signing_state()?
                }
            }
            TxParsingState::InputCommitement(_) => {
                process_input_commitement(ctx)?;
                ctx.update_hash()?;
                ctx.next_step()?;
                ctx.signing_state()?
            }
            TxParsingState::Output(_) => {
                process_output(ctx)?;
                ctx.update_hash()?;
                ctx.next_step()?;
                ctx.signing_state()?
            }
            TxParsingState::CompleteNotApproved {
                inp_idx,
                sig_idx,
                sighash,
            } => SigningState::CompleteNotApproved {
                inp_idx,
                sig_idx,
                sighash,
            },
            TxParsingState::ApprovedNotFinishedSigning {
                inp_idx,
                sig_idx,
                sighash,
            } => SigningState::ApprovedNotFinishedSigning {
                inp_idx,
                sig_idx,
                sighash,
            },
            TxParsingState::Finished => return Err(AppSW::TxFinished),
        };

        match signing_state {
            SigningState::TxParsingNotComplete => Ok(()),
            SigningState::CompleteNotApproved {
                inp_idx,
                sig_idx,
                sighash,
            } => {
                // Display transaction. If user approves
                // the transaction, sign it. Otherwise,
                // return a "deny" status word.
                if ui_display_tx(ctx)? {
                    compute_signature_and_append(comm, ctx, inp_idx, sig_idx, &sighash)?;
                    if ctx.completed_all_signatures() {
                        ctx.review_finished = true;
                    } else {
                        ctx.show_spinner();
                    }

                    Ok(())
                } else {
                    ctx.review_finished = true;
                    Err(AppSW::Deny)
                }
            }
            SigningState::ApprovedNotFinishedSigning {
                inp_idx,
                sig_idx,
                sighash,
            } => {
                // Allready approved sign and return the next signature
                compute_signature_and_append(comm, ctx, inp_idx, sig_idx, &sighash)?;
                if ctx.completed_all_signatures() {
                    ctx.review_finished = true;
                } else {
                    ctx.show_spinner();
                }

                Ok(())
            }
        }
    }
}

fn process_output(ctx: &mut TxContext) -> Result<(), AppSW> {
    let out = ml_common::TxOutput::decode_all(&mut ctx.raw_buf.as_slice())
        .map_err(|_| AppSW::DeserializeFail)?;
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
    if ctx.outputs.len() == 0 {
        ctx.tx_hasher
            .update(&Compact::<u32>::encode(&ctx.num_outputs.into()))
            .map_err(|_| AppSW::TxHashFail)?;
    }

    ctx.outputs.push(out); // FIXME: out of memory for large TXs
    Ok(())
}

fn process_input_commitement(ctx: &mut TxContext) -> Result<(), AppSW> {
    let inp_data = ctx
        .inputs_data
        .get(ctx.num_prcessed_input_commitments as usize)
        .ok_or(AppSW::WrongContext)?;
    let commitment = SighashInputCommitment::decode_all(&mut ctx.raw_buf.as_slice())
        .map_err(|_| AppSW::DeserializeFail)?;
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
            InputData::FillOrderV0(fill_amount) => {
                // FIXME:
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

    // On the first input commitment update the tx_hasher with the number of commitments
    if ctx.num_prcessed_input_commitments == 0 {
        ctx.tx_hasher
            .update(&ctx.num_inputs.to_le_bytes())
            .map_err(|_| AppSW::TxHashFail)?;
    }

    ctx.num_prcessed_input_commitments += 1;

    Ok(())
}

fn process_input(ctx: &mut TxContext) -> Result<(), AppSW> {
    let inp =
        TxInput::decode_all(&mut ctx.raw_buf.as_slice()).map_err(|_| AppSW::DeserializeFail)?;
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
            AccountCommand::ConcludeOrder(_) => InputData::ConcludeOrderV0,
            AccountCommand::FillOrder(_, fill_amount, _) => InputData::FillOrderV0(fill_amount),
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

fn compute_signature_and_append(
    comm: &mut Comm,
    ctx: &mut TxContext,
    inp_idx: usize,
    sig_idx: usize,
    sighash: &[u8; 32],
) -> Result<(), AppSW> {
    let hash_algorithm_id = CX_SHA256;
    let signing_mode = CX_ECSCHNORR_BIP0340 | CX_RND_PROVIDED | CX_LAST;

    let address = ctx
        .inputs
        .get(inp_idx)
        .ok_or(AppSW::WrongContext)?
        .addresses
        .get(sig_idx)
        .ok_or(AppSW::WrongContext)?;

    let private_key = Secp256k1::derive_from_path(address.path.as_ref());
    let sig = schnorr_sign(&private_key, sighash, hash_algorithm_id, signing_mode)?;

    let sig = Signature {
        signature: sig,
        multisig_idx: address.multisig_idx,
    };

    ctx.next_step()?;

    comm.append(&[inp_idx as u8]);
    comm.append(&sig.encode());
    if ctx.state == TxParsingState::Finished {
        comm.append(&[P2_SIGN_TX_LAST])
    } else {
        comm.append(&[P2_SIGN_TX_MORE])
    }

    Ok(())
}
