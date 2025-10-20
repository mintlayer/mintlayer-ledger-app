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
    AppSW, DataContext, P1SignTx, P2_DONE, P2_SIGN_MORE,
};
use messages::{
    CoinType, InputAdditionalInfoReq, InputAddressPath, SignTxReq, TxInputReq, TxMetadataReq,
    TxOutputReq,
};

use ledger_device_sdk::{
    ecc::{Secp256k1, SeedDerive},
    hash::{blake2::Blake2b_512, HashInit},
    io::Comm,
    nbgl::{NbglSpinner, NbglStreamingReview},
};
use ledger_secure_sdk_sys::*;
use ml_common::{
    AccountCommand, AccountSpending, Amount, OrderAccountCommand, OutputValue,
    SighashInputCommitment, TxInput, TxOutput, H256,
};
use parity_scale_codec::{Compact, Encode};

const MAX_TRANSACTION_LEN: usize = 510;
const BIP44: u32 = 44 + (1 << 31);

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

#[derive(Encode)]
pub struct Signature {
    pub signature: [u8; 64],
    pub multisig_idx: Option<u32>,
}

pub struct InputCompressed {
    pub addresses: Vec<InputAddressPathCompressed>,
}

pub struct InputAddressPathCompressed {
    pub path: [u32; 3],
    pub multisig_idx: Option<u32>,
}

impl InputAddressPathCompressed {
    fn new(addr: InputAddressPath, coin: CoinType) -> Result<Self, AppSW> {
        let path = addr.path.as_ref();
        if path.len() != 5 {
            return Err(AppSW::TxInvalidInputPath);
        }

        if path[0] != BIP44 {
            return Err(AppSW::TxInvalidInputPath);
        }

        if path[1] != coin.bip44_coin_type() {
            return Err(AppSW::TxInvalidInputPath);
        }

        Ok(Self {
            path: path[2..]
                .try_into()
                .map_err(|_| AppSW::TxInvalidInputPath)?,
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
    FillOrderV0(Amount),
    ConcludeOrderV0,
    FillOrderV1(Amount),
    FreezeOrderV1,
    ConcludeOrderV1,
}

#[derive(PartialEq, Eq)]
enum TxParsingState {
    Input(usize),
    InputAdditionalInfo(usize),
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
    coin: CoinType,
    num_inputs: u32,
    num_outputs: u32,
    review_finished: bool,
    state: TxParsingState,
    tx_type: Option<TxType>,

    tx_hasher: Blake2b_512,

    total_inputs: BTreeMap<CoinOrTokenId, Amount>,
    total_outputs: BTreeMap<CoinOrTokenId, Amount>,
    inputs: Vec<InputCompressed>,
    inputs_data: Vec<InputData>,
    num_prcessed_input_commitments: u32,

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
            tx_type: None,

            total_inputs: Default::default(),
            total_outputs: Default::default(),
            inputs: Vec::with_capacity(20),
            inputs_data: Vec::with_capacity(20),
            num_prcessed_input_commitments: 0,
            spinner: NbglSpinner::new(),
        })
    }

    pub fn coin(&self) -> CoinType {
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

    fn update_hash<T: Encode>(&mut self, data: &T) -> Result<(), AppSW> {
        self.raw_buf.clear();
        data.encode_to(&mut self.raw_buf);
        self.tx_hasher
            .update(self.raw_buf.as_slice())
            .map_err(|_| AppSW::TxHashFail)?;
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

    fn advance_next_input_step<'a>(&mut self, num_inp: usize) -> SigningState<'a> {
        self.state = if num_inp < (self.num_inputs - 1) as usize {
            TxParsingState::Input(num_inp + 1)
        } else {
            TxParsingState::InputAdditionalInfo(0)
        };

        SigningState::TxParsingNotComplete
    }

    fn advance_next_input_additional_info_step<'a>(
        &mut self,
        num_inp: usize,
        review: &'a Review,
    ) -> SigningState<'a> {
        self.state = if num_inp < (self.num_inputs - 1) as usize {
            TxParsingState::InputAdditionalInfo(num_inp + 1)
        } else {
            self.inputs_data = Vec::new();
            TxParsingState::Output(0)
        };

        match review {
            Review::Review(_) => SigningState::TxParsingNotComplete,
            Review::StreamingReview(review) => SigningState::StreamingReviewStart(review),
        }
    }

    // After processing an output advance the internal state
    fn advance_next_output_state(&mut self, n: usize) -> Result<NextTxOutputParsingState, AppSW> {
        let next_state = if n < (self.num_outputs - 1) as usize {
            NextTxOutputParsingState::Output(n + 1)
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

                NextTxOutputParsingState::CompleteNotApproved {
                    inp_idx,
                    sig_idx,
                    sighash: message_hash2[..32]
                        .try_into()
                        .map_err(|_| AppSW::TxHashFail)?,
                }
            } else {
                return Err(AppSW::NothingToSign);
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
    pub fn check_state(&self, p1: P1SignTx) -> Result<(), AppSW> {
        match (p1, &self.state) {
            (P1SignTx::Input, TxParsingState::Input(_))
            | (P1SignTx::InputAdditionalInfo, TxParsingState::InputAdditionalInfo(_))
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

    pub fn extend(&mut self, chunk: &[u8]) -> Result<(), AppSW> {
        if self.raw_buf.len() + chunk.len() > MAX_TRANSACTION_LEN {
            return Err(AppSW::TxWrongLength);
        }

        self.raw_buf.extend(chunk);
        Ok(())
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
            TxParsingState::Input(_)
            | TxParsingState::Finished
            | TxParsingState::InputAdditionalInfo(_)
            | TxParsingState::Output(_) => false,
        };

        if returning_signatures && self.num_inputs > 1 {
            self.spinner.show("Signing...");
        } else if is_transaction_big {
            self.spinner.show("Parsing transaction...");
        }
    }
}

pub fn setup_sign_tx(req: TxMetadataReq, ctx: &mut DataContext) -> Result<(), AppSW> {
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
) -> Result<SigningState<'a>, AppSW> {
    let addresses = req
        .addresses
        .into_iter()
        .map(|a| InputAddressPathCompressed::new(a, ctx.coin))
        .collect::<Result<Vec<_>, AppSW>>()?;

    ctx.inputs.push(InputCompressed { addresses });

    process_input(ctx, &req.inp)?;
    ctx.update_hash(&req.inp)?;
    Ok(ctx.advance_next_input_step(input_step))
}

fn handle_input_additional_info_req<'a>(
    req: InputAdditionalInfoReq,
    input_step: usize,
    ctx: &mut TxContext,
    review: &'a mut Review,
) -> Result<SigningState<'a>, AppSW> {
    let commitment = process_input_additional_info(ctx, req)?;
    ctx.update_hash(&commitment)?;
    Ok(ctx.advance_next_input_additional_info_step(input_step, review))
}

fn handle_output_req<'a>(
    req: TxOutputReq,
    output_step: usize,
    ctx: &mut TxContext,
    review: &'a mut Review,
) -> Result<SigningState<'a>, AppSW> {
    process_output(ctx, &req.out)?;
    // on the first output add the number of outputs to the hash
    if output_step == 0 {
        ctx.tx_hasher
            .update(&Compact::<u32>::encode(&ctx.num_outputs.into()))
            .map_err(|_| AppSW::TxHashFail)?;
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

pub fn handler_sign_tx(
    comm: &mut Comm,
    req: SignTxReq,
    ctx: &mut TxContext,
    review: &mut Review,
) -> Result<(), AppSW> {
    let signing_state = match (req, ctx.state()) {
        (SignTxReq::Input(req), TxParsingState::Input(n)) => handle_input_req(req, *n, ctx)?,
        (SignTxReq::InputAdditionalInfo(req), TxParsingState::InputAdditionalInfo(n)) => {
            handle_input_additional_info_req(req, *n, ctx, review)?
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
        ) => return Err(AppSW::Deny),
        (SignTxReq::NextSignature, TxParsingState::Finished) => {
            return Err(AppSW::TxAlreadyFinished)
        }
        (_, _) => return Err(AppSW::WrongP1P2),
    };

    match signing_state {
        SigningState::TxParsingNotComplete => Ok(()),
        SigningState::StreamingReviewStart(review) => {
            if start_streaming_review(review) {
                Ok(())
            } else {
                ctx.review_finished = true;
                Err(AppSW::Deny)
            }
        }
        SigningState::StreamingReviewOutput(review, output) => {
            if streaming_review_show_output(review, &output, ctx.coin)? {
                Ok(())
            } else {
                ctx.review_finished = true;
                Err(AppSW::Deny)
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
                Err(AppSW::Deny)
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
                Err(AppSW::Deny)
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

fn process_output(ctx: &mut TxContext, out: &TxOutput) -> Result<(), AppSW> {
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

fn process_input_additional_info(
    ctx: &mut TxContext,
    additional_info: InputAdditionalInfoReq,
) -> Result<SighashInputCommitment, AppSW> {
    let inp_data = ctx
        .inputs_data
        .get(ctx.num_prcessed_input_commitments as usize)
        .ok_or(AppSW::WrongContext)?;

    let commitment = match additional_info {
        InputAdditionalInfoReq::None => SighashInputCommitment::None,
        InputAdditionalInfoReq::Utxo { utxo } => {
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
            };
            SighashInputCommitment::Utxo(utxo)
        }
        InputAdditionalInfoReq::PoolInfo {
            utxo,
            staker_balance,
        } => {
            increase_input_totals(&mut ctx.total_inputs, CoinOrTokenId::Coin, staker_balance)?;
            SighashInputCommitment::ProduceBlockFromStakeUtxo {
                utxo,
                staker_balance,
            }
        }
        InputAdditionalInfoReq::OrderInfo {
            initially_asked,
            initially_given,
            ask_balance,
            give_balance,
        } => match &inp_data {
            InputData::FillOrderV0(fill_amount) => {
                let (fill_coin_or_token_id, asked_amount) = into_coin_or_token_id_and_amount(
                    &output_value_with_amount(&initially_asked, ask_balance)?,
                )?;
                let (given_coin_or_token_id, given_amount) = into_coin_or_token_id_and_amount(
                    &output_value_with_amount(&initially_given, give_balance)?,
                )?;

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
                SighashInputCommitment::FillOrderAccountCommand {
                    initially_asked,
                    initially_given,
                }
            }
            InputData::FillOrderV1(fill_amount) => {
                let (fill_coin_or_token_id, asked_amount) =
                    into_coin_or_token_id_and_amount(&initially_asked)?;
                let (given_coin_or_token_id, given_amount) =
                    into_coin_or_token_id_and_amount(&initially_given)?;

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

                SighashInputCommitment::FillOrderAccountCommand {
                    initially_asked,
                    initially_given,
                }
            }
            InputData::ConcludeOrderV0 | InputData::ConcludeOrderV1 => {
                let (coin_or_token_id, _) = into_coin_or_token_id_and_amount(&initially_asked)?;
                increase_input_totals(&mut ctx.total_inputs, coin_or_token_id, ask_balance)?;

                let (coin_or_token_id, _) = into_coin_or_token_id_and_amount(&initially_given)?;
                increase_input_totals(&mut ctx.total_inputs, coin_or_token_id, give_balance)?;
                SighashInputCommitment::ConcludeOrderAccountCommand {
                    initially_asked,
                    initially_given,
                    ask_balance,
                    give_balance,
                }
            }
            _ => return Err(AppSW::WrongContext),
        },
    };

    // On the first input commitment update the tx_hasher with the number of commitments
    if ctx.num_prcessed_input_commitments == 0 {
        ctx.tx_hasher
            .update(&ctx.num_inputs.to_le_bytes())
            .map_err(|_| AppSW::TxHashFail)?;
    }

    ctx.num_prcessed_input_commitments += 1;

    Ok(commitment)
}

fn process_input(ctx: &mut TxContext, inp: &TxInput) -> Result<(), AppSW> {
    let input_data = match inp {
        TxInput::Utxo(_) => InputData::Utxo,
        TxInput::Account(acc) => match acc.account {
            AccountSpending::DelegationBalance(_, amount) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::DelegationWithdrawl);
                increase_input_totals(&mut ctx.total_inputs, CoinOrTokenId::Coin, amount)?;
                InputData::DelegationWithdrawl
            }
        },
        TxInput::AccountCommand(_, cmd) => match cmd {
            AccountCommand::MintTokens(token_id, amount) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::MintTokens);
                increase_input_totals(
                    &mut ctx.total_inputs,
                    CoinOrTokenId::TokenId(*token_id),
                    *amount,
                )?;
                InputData::MintTokens
            }
            AccountCommand::ConcludeOrder(_) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::ConcludeOrder);
                InputData::ConcludeOrderV0
            }
            AccountCommand::FillOrder(_, fill_amount, _) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::FillOrder);
                InputData::FillOrderV0(*fill_amount)
            }
            AccountCommand::UnmintTokens(_) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::UnmintTokens);
                InputData::UnmintTokens
            }
            AccountCommand::LockTokenSupply(_) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::LockTokenSupply);
                InputData::LockTokenSupply
            }
            AccountCommand::FreezeToken(_, _) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::FreezeToken);
                InputData::FreezeToken
            }
            AccountCommand::UnfreezeToken(_) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::UnfreezeToken);
                InputData::UnfreezeToken
            }
            AccountCommand::ChangeTokenAuthority(_, _) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::ChangeTokenAuthority);
                InputData::ChangeTokenAuthority
            }
            AccountCommand::ChangeTokenMetadataUri(_, _) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::ChangeTokenMetadataUri);
                InputData::ChangeTokenMetadataUri
            }
        },
        TxInput::OrderAccountCommand(cmd) => match cmd {
            OrderAccountCommand::FillOrder(_, fill_amount) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::FillOrder);
                InputData::FillOrderV1(*fill_amount)
            }
            OrderAccountCommand::ConcludeOrder(_) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::ConcludeOrder);
                InputData::ConcludeOrderV1
            }
            OrderAccountCommand::FreezeOrder(_) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::FreezeOrder);
                InputData::FreezeOrderV1
            }
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
    comm.append(&sig.encode());
    if ctx.state == TxParsingState::Finished {
        comm.append(&[P2_DONE])
    } else {
        comm.append(&[P2_SIGN_MORE])
    }

    Ok(())
}

fn output_value_with_amount(value: &OutputValue, new_amount: Amount) -> Result<OutputValue, AppSW> {
    match value {
        OutputValue::Coin(_) => Ok(OutputValue::Coin(new_amount)),
        OutputValue::TokenV0 => Err(AppSW::TxInvalidTokenV0),
        OutputValue::TokenV1(token_id, _) => Ok(OutputValue::TokenV1(*token_id, new_amount)),
    }
}
