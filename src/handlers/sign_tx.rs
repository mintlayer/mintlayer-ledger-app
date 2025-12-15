/*****************************************************************************
 *   Mintlayer Ledger App.
 *   (c) 2025 RBB S.r.l.
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

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::{
    app_ui::sign::{
        approve_streaming_review, new_streaming_review, start_streaming_review,
        streaming_review_show_output, ui_display_tx,
    },
    handlers::sign_message::schnorr_sign,
    DataContext, P1SignTx, StatusWord, P2_DONE, P2_MORE,
};
use messages::{
    encode, encode_as_compact, encode_to, AccountCommand, AccountSpending, Amount, Encode,
    InputAdditionalInfo, InputAddressPath, OrderAccountCommand, OutputValue, PCoinType,
    SighashInputCommitment, SignTxReq, Signature, TxInput, TxInputReq, TxMetadataReq, TxOutput,
    TxOutputReq, H256,
};

use ledger_device_sdk::{
    ecc::{Secp256k1, SeedDerive},
    hash::{blake2::Blake2b_512, HashInit},
    io::Comm,
    nbgl::{NbglSpinner, NbglStreamingReview},
};
use ledger_secure_sdk_sys::*;

const BIP44: u32 = 44 + (1 << 31);

// BIP44/COIN/ACCOUNT/PURPOSE/INDEX
const DERIVATION_PATH_LEN: usize = 5;
// DERIVATION_PATH_LEN without the BIP44 and COIN as they are the same for all
const COMPRESSED_DERIVATION_PATH_LEN: usize = 3;

#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub enum CoinOrTokenId {
    Coin,
    TokenId(H256),
}

fn into_coin_or_token_id_and_amount(
    value: &OutputValue,
) -> Result<(CoinOrTokenId, Amount), StatusWord> {
    match value {
        OutputValue::Coin(amount) => Ok((CoinOrTokenId::Coin, *amount)),
        OutputValue::TokenV1(token_id, amount) => {
            Ok((CoinOrTokenId::TokenId(*token_id.hash()), *amount))
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TxType {
    Transfer,
    Burn,
    Htlc,
    CreateDelegation,
    DelegationStake,
    DelegationWithdrawl,
    CreateStakePool,
    DecommissionStakePool,
    CreateNft,
    CreateToken,
    MintTokens,
    UnmintTokens,
    FreezeToken,
    UnfreezeToken,
    LockTokenSupply,
    ChangeTokenAuthority,
    ChangeTokenMetadataUri,
    FillOrder,
    FreezeOrder,
    CreateOrder,
    ConcludeOrder,
    ComplexTransaction,
    DataDeposit,
}

fn merge_tx_type(tx_type: Option<TxType>, new_type: TxType) -> Option<TxType> {
    match tx_type {
        None => Some(new_type),
        // Transfers are a lower priority (as they can be change outputs) so keep the previous type
        Some(_) if new_type == TxType::Transfer => tx_type,
        Some(_) => Some(TxType::ComplexTransaction),
    }
}

pub struct InputCompressed {
    pub addresses: Vec<InputAddressPathCompressed>,
}

pub struct InputAddressPathCompressed {
    pub path: [u32; COMPRESSED_DERIVATION_PATH_LEN],
    pub multisig_idx: Option<u32>,
}

impl InputAddressPathCompressed {
    fn new(addr: InputAddressPath, coin: PCoinType) -> Result<Self, StatusWord> {
        let path = addr.path.as_ref();
        if path.len() != DERIVATION_PATH_LEN {
            return Err(StatusWord::TxInvalidInputPath);
        }

        if path[0] != BIP44 {
            return Err(StatusWord::TxInvalidInputPath);
        }

        if path[1] != coin.bip44_coin_type() {
            return Err(StatusWord::TxInvalidInputPath);
        }

        Ok(Self {
            path: path[2..]
                .try_into()
                .map_err(|_| StatusWord::TxInvalidInputPath)?,
            multisig_idx: addr.multisig_idx,
        })
    }
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
    FillOrderV0,
    FillOrderV1,
    FreezeOrderV1,
    ConcludeOrder,
}

#[derive(PartialEq, Eq)]
enum TxParsingState {
    Input(usize),
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

#[derive(PartialEq, Eq)]
pub enum NextTxOutputParsingState {
    Output(usize),
    CompleteNotApproved {
        inp_idx: usize,
        sig_idx: usize,
        sighash: [u8; 32],
    },
}

pub struct TxContext {
    raw_buf: Vec<u8>,
    coin: PCoinType,
    num_inputs: u32,
    num_outputs: u32,
    review_finished: bool,
    state: TxParsingState,
    tx_type: Option<TxType>,

    tx_hasher: Blake2b_512,

    total_inputs: BTreeMap<CoinOrTokenId, Amount>,
    total_outputs: BTreeMap<CoinOrTokenId, Amount>,
    inputs: Vec<InputCompressed>,
    input_commitments: Vec<SighashInputCommitment>,

    spinner: NbglSpinner,
}

pub enum Review {
    Review(Vec<TxOutput>),
    StreamingReview(NbglStreamingReview),
}

pub enum SigningState<'a> {
    StreamingReviewStart(&'a NbglStreamingReview),
    StreamingReviewOutput(&'a NbglStreamingReview, TxOutput),
    StreamingReviewApprove {
        review: &'a NbglStreamingReview,
        output: TxOutput,
        inp_idx: usize,
        sig_idx: usize,
        sighash: [u8; 32],
    },
    TxParsingNotComplete,
    CompleteNotApproved {
        inp_idx: usize,
        sig_idx: usize,
        sighash: [u8; 32],
        outputs: &'a [TxOutput],
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
        TxMetadataReq {
            coin,
            version,
            num_inputs,
            num_outputs,
        }: TxMetadataReq,
    ) -> Result<TxContext, StatusWord> {
        let mut tx_hasher = Blake2b_512::new();
        // mode
        tx_hasher
            .update(b"\x01")
            .map_err(|_| StatusWord::TxHashFail)?;
        // version
        tx_hasher
            .update(&[version])
            .map_err(|_| StatusWord::TxHashFail)?;
        // flags
        tx_hasher
            .update(&[0; 16])
            .map_err(|_| StatusWord::TxHashFail)?;

        Ok(TxContext {
            coin: coin.into(),
            raw_buf: Vec::with_capacity(251),
            num_inputs,
            num_outputs,
            review_finished: false,
            tx_hasher,
            state: TxParsingState::Input(0),
            tx_type: None,

            total_inputs: Default::default(),
            total_outputs: Default::default(),
            inputs: Vec::with_capacity(20),
            input_commitments: Vec::with_capacity(20),
            spinner: NbglSpinner::new(),
        })
    }

    pub fn coin(&self) -> PCoinType {
        self.coin
    }

    pub fn tx_type(&self) -> Option<TxType> {
        self.tx_type
    }

    pub fn total_inputs(&self) -> &BTreeMap<CoinOrTokenId, Amount> {
        &self.total_inputs
    }

    pub fn total_outputs(&self) -> &BTreeMap<CoinOrTokenId, Amount> {
        &self.total_outputs
    }

    fn state(&self) -> &TxParsingState {
        &self.state
    }

    fn update_hash<T: Encode>(&mut self, data: &T) -> Result<(), StatusWord> {
        self.raw_buf.clear();
        encode_to(data, &mut self.raw_buf);
        self.tx_hasher
            .update(self.raw_buf.as_slice())
            .map_err(|_| StatusWord::TxHashFail)?;
        self.raw_buf.clear();
        Ok(())
    }

    fn completed_all_signatures(&self) -> bool {
        self.state == TxParsingState::Finished
    }

    // Get review status
    #[allow(dead_code)]
    pub fn finished(&self) -> bool {
        self.review_finished
    }

    fn advance_next_input_additional_info_step<'a>(
        &mut self,
        current_input_step: usize,
        review: &'a Review,
    ) -> Result<SigningState<'a>, StatusWord> {
        let finished_with_inputs = current_input_step >= (self.num_inputs - 1) as usize;

        let signing_state = if finished_with_inputs {
            // Update hash for input commitments and proceed with outputs
            self.tx_hasher
                .update(&self.num_inputs.to_le_bytes())
                .map_err(|_| StatusWord::TxHashFail)?;

            for commitment in core::mem::take(&mut self.input_commitments) {
                self.update_hash(&commitment)?;
            }

            self.state = TxParsingState::Output(0);
            match review {
                Review::Review(_) => SigningState::TxParsingNotComplete,
                Review::StreamingReview(review) => SigningState::StreamingReviewStart(review),
            }
        } else {
            self.state = TxParsingState::Input(current_input_step + 1);
            SigningState::TxParsingNotComplete
        };

        Ok(signing_state)
    }

    // After processing an output advance the internal state
    fn advance_next_output_state(
        &mut self,
        n: usize,
    ) -> Result<NextTxOutputParsingState, StatusWord> {
        let next_state = if n < (self.num_outputs - 1) as usize {
            NextTxOutputParsingState::Output(n + 1)
        } else {
            let next = self.next_input_idx_to_sign(0, None);
            if let Some((inp_idx, sig_idx)) = next {
                // Finalize the tx hash for signing
                let mut message_hash: [u8; 64] = [0u8; 64];
                self.tx_hasher
                    .finalize(&mut message_hash)
                    .map_err(|_| StatusWord::TxHashFail)?;

                let mut blake2b256 = Blake2b_512::new();
                let mut message_hash2: [u8; 64] = [0u8; 64];
                blake2b256
                    .hash(&message_hash[0..32], &mut message_hash2)
                    .map_err(|_| StatusWord::TxHashFail)?;

                NextTxOutputParsingState::CompleteNotApproved {
                    inp_idx,
                    sig_idx,
                    sighash: message_hash2[..32]
                        .try_into()
                        .map_err(|_| StatusWord::TxHashFail)?,
                }
            } else {
                return Err(StatusWord::NothingToSign);
            }
        };

        self.state = match next_state {
            NextTxOutputParsingState::Output(out) => TxParsingState::Output(out),
            NextTxOutputParsingState::CompleteNotApproved {
                inp_idx,
                sig_idx,
                sighash,
            } => TxParsingState::CompleteNotApproved {
                inp_idx,
                sig_idx,
                sighash,
            },
        };

        Ok(next_state)
    }

    // After processing a signature advance the internal state
    fn advance_next_signing_step(&mut self, inp_idx: usize, sig_idx: usize, sighash: &[u8; 32]) {
        let next = self.next_input_idx_to_sign(inp_idx, Some(sig_idx));
        self.state = if let Some((inp_idx, sig_idx)) = next {
            TxParsingState::ApprovedNotFinishedSigning {
                inp_idx,
                sig_idx,
                sighash: *sighash,
            }
        } else {
            TxParsingState::Finished
        };
    }

    // As some inputs don't need signing and some multisig inputs can be signed multiple times
    // find the next input index to sign.
    //
    // Returns the Tx input index and the index of the path/destination
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

    // Check the state corresponds to the incoming request
    pub fn check_state(&self, p1: P1SignTx) -> Result<(), StatusWord> {
        match (p1, &self.state) {
            (P1SignTx::Input, TxParsingState::Input(_))
            | (P1SignTx::Output, TxParsingState::Output(_))
            | (
                P1SignTx::NextSignature,
                TxParsingState::ApprovedNotFinishedSigning {
                    inp_idx: _,
                    sig_idx: _,
                    sighash: _,
                },
            ) => Ok(()),
            (_, _) => Err(StatusWord::WrongP1P2),
        }
    }

    // show a spinner for bigger transactions
    pub fn show_spinner(&mut self) {
        let is_transaction_big = self.num_inputs * 2 + self.num_outputs > 10;
        let returning_signatures = match self.state {
            TxParsingState::ApprovedNotFinishedSigning {
                inp_idx: _,
                sig_idx: _,
                sighash: _,
            }
            | TxParsingState::CompleteNotApproved {
                inp_idx: _,
                sig_idx: _,
                sighash: _,
            } => true,
            TxParsingState::Input(_) | TxParsingState::Finished | TxParsingState::Output(_) => {
                false
            }
        };

        if returning_signatures && self.num_inputs > 1 {
            self.spinner.show("Signing...");
        } else if is_transaction_big {
            self.spinner.show("Parsing transaction...");
        }
    }
}

pub fn setup_sign_tx(req: TxMetadataReq, ctx: &mut DataContext) -> Result<(), StatusWord> {
    let mut tx_ctx = TxContext::new(req)?;

    tx_ctx.show_spinner();

    // if has many outputs use a streaming review
    if tx_ctx.num_outputs > 10 {
        *ctx = DataContext::TxContext(tx_ctx, Review::StreamingReview(new_streaming_review()));
    } else {
        *ctx = DataContext::TxContext(tx_ctx, Review::Review(Vec::new()));
    }

    Ok(())
}

fn handle_input_req<'a>(
    req: TxInputReq,
    input_step: usize,
    ctx: &mut TxContext,
    review: &'a mut Review,
) -> Result<SigningState<'a>, StatusWord> {
    let addresses = req
        .addresses
        .into_iter()
        .map(|a| InputAddressPathCompressed::new(a, ctx.coin))
        .collect::<Result<Vec<_>, StatusWord>>()?;

    ctx.inputs.push(InputCompressed { addresses });

    let input_data = process_input(ctx, &req.inp, &req.additional_info)?;

    let commitment = into_input_commitment(input_data, req.additional_info)?;
    ctx.input_commitments.push(commitment);

    ctx.update_hash(&req.inp)?;
    ctx.advance_next_input_additional_info_step(input_step, review)
}

fn handle_output_req<'a>(
    req: TxOutputReq,
    output_step: usize,
    ctx: &mut TxContext,
    review: &'a mut Review,
) -> Result<SigningState<'a>, StatusWord> {
    process_output(ctx, &req.out)?;
    // on the first output add the number of outputs to the hash
    if output_step == 0 {
        ctx.tx_hasher
            .update(&encode_as_compact(ctx.num_outputs))
            .map_err(|_| StatusWord::TxHashFail)?;
    }
    ctx.update_hash(&req.out)?;
    let next_step = ctx.advance_next_output_state(output_step)?;
    let signin_state = match review {
        Review::Review(outputs) => {
            outputs.push(req.out);
            match next_step {
                NextTxOutputParsingState::Output(_) => SigningState::TxParsingNotComplete,
                NextTxOutputParsingState::CompleteNotApproved {
                    inp_idx,
                    sig_idx,
                    sighash,
                } => SigningState::CompleteNotApproved {
                    inp_idx,
                    sig_idx,
                    sighash,
                    outputs,
                },
            }
        }
        Review::StreamingReview(review) => {
            // on last output show it and ask for approval
            match next_step {
                NextTxOutputParsingState::Output(_) => {
                    SigningState::StreamingReviewOutput(review, req.out)
                }
                NextTxOutputParsingState::CompleteNotApproved {
                    inp_idx,
                    sig_idx,
                    sighash,
                } => SigningState::StreamingReviewApprove {
                    review,
                    output: req.out,
                    inp_idx,
                    sig_idx,
                    sighash,
                },
            }
        }
    };

    Ok(signin_state)
}

pub fn handle_sign_tx(
    comm: &mut Comm,
    req: SignTxReq,
    ctx: &mut TxContext,
    review: &mut Review,
) -> Result<(), StatusWord> {
    let signing_state = match (req, ctx.state()) {
        (SignTxReq::Input(req), TxParsingState::Input(n)) => {
            handle_input_req(req, *n, ctx, review)?
        }
        (SignTxReq::Output(req), TxParsingState::Output(n)) => {
            handle_output_req(req, *n, ctx, review)?
        }
        (
            SignTxReq::NextSignature,
            TxParsingState::ApprovedNotFinishedSigning {
                inp_idx,
                sig_idx,
                sighash,
            },
        ) => SigningState::ApprovedNotFinishedSigning {
            inp_idx: *inp_idx,
            sig_idx: *sig_idx,
            sighash: *sighash,
        },
        (
            SignTxReq::NextSignature,
            TxParsingState::CompleteNotApproved {
                inp_idx: _,
                sig_idx: _,
                sighash: _,
            },
        ) => return Err(StatusWord::Deny),
        (SignTxReq::NextSignature, TxParsingState::Finished) => {
            return Err(StatusWord::TxAlreadyFinished)
        }
        (_, _) => return Err(StatusWord::WrongP1P2),
    };

    match signing_state {
        SigningState::TxParsingNotComplete => Ok(()),
        SigningState::StreamingReviewStart(review) => {
            if start_streaming_review(review) {
                Ok(())
            } else {
                ctx.review_finished = true;
                Err(StatusWord::Deny)
            }
        }
        SigningState::StreamingReviewOutput(review, output) => {
            if streaming_review_show_output(review, &output, ctx.coin)? {
                Ok(())
            } else {
                ctx.review_finished = true;
                Err(StatusWord::Deny)
            }
        }
        SigningState::StreamingReviewApprove {
            review,
            output,
            inp_idx,
            sig_idx,
            sighash,
        } => {
            if approve_streaming_review(review, &output, ctx)? {
                compute_signature_and_append(comm, ctx, inp_idx, sig_idx, &sighash)?;
                if ctx.completed_all_signatures() {
                    ctx.review_finished = true;
                } else {
                    ctx.show_spinner();
                }
                Ok(())
            } else {
                ctx.review_finished = true;
                Err(StatusWord::Deny)
            }
        }
        SigningState::CompleteNotApproved {
            inp_idx,
            sig_idx,
            sighash,
            outputs,
        } => {
            // Display transaction. If user approves the transaction, sign it.
            // Otherwise, return a "deny" status word.
            if ui_display_tx(ctx, outputs)? {
                compute_signature_and_append(comm, ctx, inp_idx, sig_idx, &sighash)?;
                if ctx.completed_all_signatures() {
                    ctx.review_finished = true;
                } else {
                    ctx.show_spinner();
                }

                Ok(())
            } else {
                ctx.review_finished = true;
                Err(StatusWord::Deny)
            }
        }
        SigningState::ApprovedNotFinishedSigning {
            inp_idx,
            sig_idx,
            sighash,
        } => {
            // Already approved sign and return the next signature
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

fn process_output(ctx: &mut TxContext, out: &TxOutput) -> Result<(), StatusWord> {
    match &out {
        TxOutput::Transfer(value, _) | TxOutput::LockThenTransfer(value, _, _) => {
            ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::Transfer);

            let (coin_or_token_id, amount) = into_coin_or_token_id_and_amount(value)?;
            increase_output_totals(&mut ctx.total_outputs, coin_or_token_id, amount)?;
        }
        TxOutput::Burn(value) => {
            ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::Burn);

            let (coin_or_token_id, amount) = into_coin_or_token_id_and_amount(value)?;
            increase_output_totals(&mut ctx.total_outputs, coin_or_token_id, amount)?;
        }
        TxOutput::Htlc(value, _) => {
            ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::Htlc);

            let (coin_or_token_id, amount) = into_coin_or_token_id_and_amount(value)?;
            increase_output_totals(&mut ctx.total_outputs, coin_or_token_id, amount)?;
        }
        TxOutput::CreateStakePool(_, data) => {
            ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::CreateStakePool);

            increase_output_totals(&mut ctx.total_outputs, CoinOrTokenId::Coin, data.pledge)?;
        }
        TxOutput::ProduceBlockFromStake(_, _) => {}
        TxOutput::DelegateStaking(amount, _) => {
            ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::DelegationStake);
            increase_output_totals(&mut ctx.total_outputs, CoinOrTokenId::Coin, *amount)?;
        }
        TxOutput::CreateDelegationId(_, _) => {
            ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::CreateDelegation);
        }
        TxOutput::IssueFungibleToken(_) => {
            ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::CreateToken);
        }
        TxOutput::DataDeposit(_) => {
            ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::DataDeposit);
        }
        TxOutput::IssueNft(_, _, _) => {
            ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::CreateNft);
        }
        TxOutput::CreateOrder(_) => {
            ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::CreateOrder);
        }
    }

    Ok(())
}

fn into_input_commitment(
    inp_data: InputData,
    additional_info: InputAdditionalInfo,
) -> Result<SighashInputCommitment, StatusWord> {
    let commitment = match additional_info {
        InputAdditionalInfo::None => SighashInputCommitment::None,
        InputAdditionalInfo::Utxo { utxo } => SighashInputCommitment::Utxo(utxo),
        InputAdditionalInfo::PoolInfo {
            utxo,
            staker_balance,
        } => SighashInputCommitment::ProduceBlockFromStakeUtxo {
            utxo,
            staker_balance,
        },
        InputAdditionalInfo::OrderInfo {
            initially_asked,
            initially_given,
            ask_balance,
            give_balance,
        } => match &inp_data {
            InputData::FillOrderV0 => SighashInputCommitment::FillOrderAccountCommand {
                initially_asked,
                initially_given,
            },
            InputData::FillOrderV1 => SighashInputCommitment::FillOrderAccountCommand {
                initially_asked,
                initially_given,
            },
            InputData::ConcludeOrder => SighashInputCommitment::ConcludeOrderAccountCommand {
                initially_asked,
                initially_given,
                ask_balance,
                give_balance,
            },
            _ => return Err(StatusWord::WrongContext),
        },
    };

    Ok(commitment)
}

fn process_input(
    ctx: &mut TxContext,
    inp: &TxInput,
    additional_info: &InputAdditionalInfo,
) -> Result<InputData, StatusWord> {
    let input_data = match (inp, additional_info) {
        (
            TxInput::Utxo(_),
            InputAdditionalInfo::PoolInfo {
                utxo: _,
                staker_balance,
            },
        ) => {
            increase_input_totals(&mut ctx.total_inputs, CoinOrTokenId::Coin, *staker_balance)?;
            InputData::Utxo
        }
        (TxInput::Utxo(_), InputAdditionalInfo::Utxo { utxo }) => {
            match &utxo {
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
                | TxOutput::CreateOrder(_) => return Err(StatusWord::TxInvalidInputUtxo),
                TxOutput::CreateStakePool(_, data) => {
                    increase_input_totals(&mut ctx.total_inputs, CoinOrTokenId::Coin, data.pledge)?;
                }
                TxOutput::IssueNft(nft_id, _, _) => {
                    increase_input_totals(
                        &mut ctx.total_inputs,
                        CoinOrTokenId::TokenId(*nft_id.hash()),
                        Amount::from_atoms(1),
                    )?;
                }
            };
            InputData::Utxo
        }
        (TxInput::Utxo(_), _) => {
            return Err(StatusWord::WrongContext);
        }
        (TxInput::Account(acc), InputAdditionalInfo::None) => match acc.spending {
            AccountSpending::DelegationBalance(_, amount) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::DelegationWithdrawl);
                increase_input_totals(&mut ctx.total_inputs, CoinOrTokenId::Coin, amount)?;
                InputData::DelegationWithdrawl
            }
        },
        (TxInput::Account(_), _) => {
            return Err(StatusWord::WrongContext);
        }
        (TxInput::AccountCommand(_, cmd), additional_info) => match (cmd, additional_info) {
            (AccountCommand::MintTokens(token_id, amount), InputAdditionalInfo::None) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::MintTokens);
                increase_input_totals(
                    &mut ctx.total_inputs,
                    CoinOrTokenId::TokenId(*token_id.hash()),
                    *amount,
                )?;
                InputData::MintTokens
            }
            (
                AccountCommand::ConcludeOrder(_),
                InputAdditionalInfo::OrderInfo {
                    initially_asked,
                    initially_given,
                    ask_balance,
                    give_balance,
                },
            ) => {
                let (coin_or_token_id, _) = into_coin_or_token_id_and_amount(&initially_asked)?;
                increase_input_totals(&mut ctx.total_inputs, coin_or_token_id, *ask_balance)?;

                let (coin_or_token_id, _) = into_coin_or_token_id_and_amount(&initially_given)?;
                increase_input_totals(&mut ctx.total_inputs, coin_or_token_id, *give_balance)?;

                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::ConcludeOrder);
                InputData::ConcludeOrder
            }
            (
                AccountCommand::FillOrder(_, fill_amount, _),
                InputAdditionalInfo::OrderInfo {
                    initially_asked,
                    initially_given,
                    ask_balance,
                    give_balance,
                },
            ) => {
                let (fill_coin_or_token_id, asked_amount) = into_coin_or_token_id_and_amount(
                    &output_value_with_amount(&initially_asked, *ask_balance)?,
                )?;
                let (given_coin_or_token_id, given_amount) = into_coin_or_token_id_and_amount(
                    &output_value_with_amount(&initially_given, *give_balance)?,
                )?;

                increase_output_totals(
                    &mut ctx.total_outputs,
                    fill_coin_or_token_id,
                    *fill_amount,
                )?;

                let atoms = given_amount
                    .into_atoms()
                    .checked_mul(fill_amount.into_atoms())
                    .ok_or(StatusWord::TxNumericOperationFail)?
                    .checked_div(asked_amount.into_atoms())
                    .ok_or(StatusWord::TxNumericOperationFail)?;
                let amount = Amount::from_atoms(atoms);
                increase_input_totals(&mut ctx.total_inputs, given_coin_or_token_id, amount)?;

                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::FillOrder);
                InputData::FillOrderV0
            }
            (AccountCommand::UnmintTokens(_), InputAdditionalInfo::None) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::UnmintTokens);
                InputData::UnmintTokens
            }
            (AccountCommand::LockTokenSupply(_), InputAdditionalInfo::None) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::LockTokenSupply);
                InputData::LockTokenSupply
            }
            (AccountCommand::FreezeToken(_, _), InputAdditionalInfo::None) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::FreezeToken);
                InputData::FreezeToken
            }
            (AccountCommand::UnfreezeToken(_), InputAdditionalInfo::None) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::UnfreezeToken);
                InputData::UnfreezeToken
            }
            (AccountCommand::ChangeTokenAuthority(_, _), InputAdditionalInfo::None) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::ChangeTokenAuthority);
                InputData::ChangeTokenAuthority
            }
            (AccountCommand::ChangeTokenMetadataUri(_, _), InputAdditionalInfo::None) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::ChangeTokenMetadataUri);
                InputData::ChangeTokenMetadataUri
            }
            _ => return Err(StatusWord::WrongContext),
        },
        (
            TxInput::OrderAccountCommand(OrderAccountCommand::FillOrder(_, fill_amount)),
            InputAdditionalInfo::OrderInfo {
                initially_asked,
                initially_given,
                ask_balance: _,
                give_balance: _,
            },
        ) => {
            let (fill_coin_or_token_id, asked_amount) =
                into_coin_or_token_id_and_amount(&initially_asked)?;
            let (given_coin_or_token_id, given_amount) =
                into_coin_or_token_id_and_amount(&initially_given)?;

            increase_output_totals(&mut ctx.total_outputs, fill_coin_or_token_id, *fill_amount)?;

            let atoms = given_amount
                .into_atoms()
                .checked_mul(fill_amount.into_atoms())
                .ok_or(StatusWord::TxNumericOperationFail)?
                .checked_div(asked_amount.into_atoms())
                .ok_or(StatusWord::TxNumericOperationFail)?;
            let amount = Amount::from_atoms(atoms);
            increase_input_totals(&mut ctx.total_inputs, given_coin_or_token_id, amount)?;

            ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::FillOrder);
            InputData::FillOrderV1
        }
        (
            TxInput::OrderAccountCommand(OrderAccountCommand::ConcludeOrder(_)),
            InputAdditionalInfo::OrderInfo {
                initially_asked,
                initially_given,
                ask_balance,
                give_balance,
            },
        ) => {
            let (coin_or_token_id, _) = into_coin_or_token_id_and_amount(&initially_asked)?;
            increase_input_totals(&mut ctx.total_inputs, coin_or_token_id, *ask_balance)?;

            let (coin_or_token_id, _) = into_coin_or_token_id_and_amount(&initially_given)?;
            increase_input_totals(&mut ctx.total_inputs, coin_or_token_id, *give_balance)?;

            ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::ConcludeOrder);
            InputData::ConcludeOrder
        }
        (
            TxInput::OrderAccountCommand(OrderAccountCommand::FreezeOrder(_)),
            InputAdditionalInfo::None,
        ) => {
            ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::FreezeOrder);
            InputData::FreezeOrderV1
        }

        _ => return Err(StatusWord::WrongContext),
    };

    if ctx.inputs.len() == 1 {
        ctx.tx_hasher
            .update(&ctx.num_inputs.to_le_bytes())
            .map_err(|_| StatusWord::TxHashFail)?;
    }

    Ok(input_data)
}

fn increase_input_totals(
    total_inputs: &mut BTreeMap<CoinOrTokenId, Amount>,
    key: CoinOrTokenId,
    amount: Amount,
) -> Result<(), StatusWord> {
    let total = total_inputs.entry(key).or_insert(Amount::from_atoms(0));
    let new_total = total
        .into_atoms()
        .checked_add(amount.into_atoms())
        .ok_or(StatusWord::TxNumericOperationFail)?;
    *total = Amount::from_atoms(new_total);
    Ok(())
}

fn increase_output_totals(
    total_outputs: &mut BTreeMap<CoinOrTokenId, Amount>,
    key: CoinOrTokenId,
    amount: Amount,
) -> Result<(), StatusWord> {
    let total = total_outputs.entry(key).or_insert(Amount::from_atoms(0));
    let new_total = total
        .into_atoms()
        .checked_add(amount.into_atoms())
        .ok_or(StatusWord::TxNumericOperationFail)?;
    *total = Amount::from_atoms(new_total);
    Ok(())
}

fn compute_signature_and_append(
    comm: &mut Comm,
    ctx: &mut TxContext,
    inp_idx: usize,
    sig_idx: usize,
    sighash: &[u8; 32],
) -> Result<(), StatusWord> {
    let hash_algorithm_id = CX_SHA256;
    let signing_mode = CX_ECSCHNORR_BIP0340 | CX_RND_PROVIDED | CX_LAST;

    let address = ctx
        .inputs
        .get(inp_idx)
        .ok_or(StatusWord::WrongContext)?
        .addresses
        .get(sig_idx)
        .ok_or(StatusWord::WrongContext)?;

    let [p1, p2, p3] = address.path;
    let addr = [BIP44, ctx.coin.bip44_coin_type(), p1, p2, p3];

    let private_key = Secp256k1::derive_from_path(&addr);
    let sig = schnorr_sign(&private_key, sighash, hash_algorithm_id, signing_mode)?;

    let sig = Signature {
        signature: sig,
        multisig_idx: address.multisig_idx,
    };

    ctx.advance_next_signing_step(inp_idx, sig_idx, sighash);

    comm.append(&[inp_idx as u8]);
    comm.append(&encode(sig));
    if ctx.state == TxParsingState::Finished {
        comm.append(&[P2_DONE])
    } else {
        comm.append(&[P2_MORE])
    }

    Ok(())
}

fn output_value_with_amount(
    value: &OutputValue,
    new_amount: Amount,
) -> Result<OutputValue, StatusWord> {
    match value {
        OutputValue::Coin(_) => Ok(OutputValue::Coin(new_amount)),
        OutputValue::TokenV1(token_id, _) => Ok(OutputValue::TokenV1(*token_id, new_amount)),
    }
}
