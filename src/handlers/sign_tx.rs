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
    handlers::{sign_message::schnorr_sign, utils::mintlayer_hash},
    DataContext, StatusWord,
};
use messages::{
    encode_as_compact, encode_to,
    mlcp::{
        AccountCommand, AccountSpending, Amount, CoinType as PCoinType, OrderAccountCommand,
        OutputValue, SighashInputCommitment, TxOutput, H256,
    },
    AdditionalOrderInfo, AdditionalUtxoInfo, CoinType, Encode, InputAddressPath, Response,
    SignTxReq, Signature, TxInputReq, TxInputSignatureResponse, TxInputWithAdditionalInfo,
    TxMetadataReq, TxMetadataV1Req, TxMetadataVersionReq, TxOutputReq,
};

use ledger_device_sdk::{
    ecc::{Secp256k1, SeedDerive},
    hash::{blake2::Blake2b_512, HashInit},
    nbgl::{NbglSpinner, NbglStreamingReview},
};

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
    pub path: [u32; COMPRESSED_DERIVATION_PATH_LEN],
    pub input_idx: u32,
    pub multisig_idx: Option<u32>,
}

impl InputCompressed {
    fn new(addr: InputAddressPath, input_idx: u32, coin: PCoinType) -> Result<Self, StatusWord> {
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
            input_idx,
            multisig_idx: addr.multisig_idx,
        })
    }
}

#[derive(PartialEq, Eq)]
enum TxParsingState {
    Input(u32),
    InputCommitment {
        inp_idx: u32,
        input_commitments_hash: [u8; 64],
    },
    Output(u32),
    CompleteNotApproved {
        inp_idx: u32,
        sighash: H256,
    },
    ApprovedNotFinishedSigning {
        inp_idx: u32,
        sighash: H256,
    },
    Finished,
}

#[derive(PartialEq, Eq)]
pub enum NextTxOutputParsingState {
    Output(u32),
    CompleteNotApproved { inp_idx: u32, sighash: H256 },
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
    input_commitments_hasher: Blake2b_512,

    total_inputs: BTreeMap<CoinOrTokenId, Amount>,
    total_outputs: BTreeMap<CoinOrTokenId, Amount>,
    inputs: Vec<InputCompressed>,

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
        inp_idx: u32,
        sighash: H256,
    },
    TxParsingNotComplete,
    CompleteNotApproved {
        inp_idx: u32,
        sighash: H256,
        outputs: &'a [TxOutput],
    },
    ApprovedNotFinishedSigning {
        inp_idx: u32,
        sighash: H256,
    },
}

impl TxContext {
    pub fn from_v1(
        coin: CoinType,
        TxMetadataV1Req {
            num_inputs,
            num_outputs,
        }: TxMetadataV1Req,
    ) -> Result<TxContext, StatusWord> {
        const VERSION_1: u8 = 1;
        let mut tx_hasher = Blake2b_512::new();
        // mode
        tx_hasher
            .update(b"\x01")
            .map_err(|_| StatusWord::TxHashFail)?;
        // version
        tx_hasher
            .update(&[VERSION_1])
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
            input_commitments_hasher: Blake2b_512::new(),
            state: TxParsingState::Input(0),
            tx_type: None,

            total_inputs: Default::default(),
            total_outputs: Default::default(),
            inputs: Vec::with_capacity(20),
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
        update_hash(&mut self.raw_buf, data, &mut self.tx_hasher)
    }

    fn update_input_commitments_hash<T: Encode>(&mut self, data: &T) -> Result<(), StatusWord> {
        update_hash(&mut self.raw_buf, data, &mut self.input_commitments_hasher)
    }

    fn completed_all_signatures(&self) -> bool {
        self.state == TxParsingState::Finished
    }

    // Get review status
    #[allow(dead_code)]
    pub fn finished(&self) -> bool {
        self.review_finished
    }

    fn advance_next_input_step<'a>(
        &mut self,
        current_input_step: u32,
    ) -> Result<SigningState<'a>, StatusWord> {
        let finished_with_inputs = current_input_step >= (self.num_inputs - 1);

        self.state = if finished_with_inputs {
            // Update hash for input commitments and proceed with outputs
            self.tx_hasher
                .update(&self.num_inputs.to_le_bytes())
                .map_err(|_| StatusWord::TxHashFail)?;

            let mut input_commitments_hash: [u8; 64] = [0u8; 64];
            self.input_commitments_hasher
                .finalize(&mut input_commitments_hash)
                .map_err(|_| StatusWord::TxHashFail)?;

            self.input_commitments_hasher = Blake2b_512::new();

            TxParsingState::InputCommitment {
                inp_idx: 0,
                input_commitments_hash,
            }
        } else {
            TxParsingState::Input(current_input_step + 1)
        };

        Ok(SigningState::TxParsingNotComplete)
    }

    fn advance_next_input_additional_info_step<'a>(
        &mut self,
        current_input_step: u32,
        expected_input_commitments_hash: [u8; 64],
        review: &'a Review,
    ) -> Result<SigningState<'a>, StatusWord> {
        let finished_with_inputs = current_input_step >= (self.num_inputs - 1);

        let signing_state = if finished_with_inputs {
            // Make sure the hashes match before continuing with the outputs
            let mut input_commitments_hash: [u8; 64] = [0u8; 64];
            self.input_commitments_hasher
                .finalize(&mut input_commitments_hash)
                .map_err(|_| StatusWord::TxHashFail)?;

            if input_commitments_hash != expected_input_commitments_hash {
                return Err(StatusWord::DifferentInputCommitmentHash);
            }

            self.state = TxParsingState::Output(0);

            match review {
                Review::Review(_) => SigningState::TxParsingNotComplete,
                Review::StreamingReview(review) => SigningState::StreamingReviewStart(review),
            }
        } else {
            self.state = TxParsingState::InputCommitment {
                inp_idx: current_input_step + 1,
                input_commitments_hash: expected_input_commitments_hash,
            };
            SigningState::TxParsingNotComplete
        };

        Ok(signing_state)
    }

    // After processing an output advance the internal state
    fn advance_next_output_state(
        &mut self,
        n: u32,
    ) -> Result<NextTxOutputParsingState, StatusWord> {
        let next_state = if n < (self.num_outputs - 1) {
            NextTxOutputParsingState::Output(n + 1)
        } else {
            let inp_idx = 0;
            // Finalize the tx hash for signing
            let mut message_hash: [u8; 64] = [0u8; 64];
            self.tx_hasher
                .finalize(&mut message_hash)
                .map_err(|_| StatusWord::TxHashFail)?;

            let message_hash2 = mintlayer_hash(&message_hash[0..32])?;

            NextTxOutputParsingState::CompleteNotApproved {
                inp_idx,
                sighash: message_hash2,
            }
        };

        self.state = match next_state {
            NextTxOutputParsingState::Output(out) => TxParsingState::Output(out),
            NextTxOutputParsingState::CompleteNotApproved { inp_idx, sighash } => {
                TxParsingState::CompleteNotApproved { inp_idx, sighash }
            }
        };

        Ok(next_state)
    }

    // After processing a signature advance the internal state
    fn advance_next_signing_step(&mut self, inp_idx: u32, sighash: &H256) {
        self.state = if ((inp_idx + 1) as usize) < self.inputs.len() {
            TxParsingState::ApprovedNotFinishedSigning {
                inp_idx: inp_idx + 1,
                sighash: *sighash,
            }
        } else {
            TxParsingState::Finished
        };
    }

    // show a spinner for bigger transactions
    pub fn show_spinner(&mut self) {
        let is_transaction_big = self.num_inputs * 2 + self.num_outputs > 10;
        let returning_signatures = match self.state {
            TxParsingState::ApprovedNotFinishedSigning {
                inp_idx: _,
                sighash: _,
            }
            | TxParsingState::CompleteNotApproved {
                inp_idx: _,
                sighash: _,
            } => true,
            TxParsingState::Input(_)
            | TxParsingState::InputCommitment {
                inp_idx: _,
                input_commitments_hash: _,
            }
            | TxParsingState::Finished
            | TxParsingState::Output(_) => false,
        };

        if returning_signatures && self.num_inputs > 1 {
            self.spinner.show("Signing...");
        } else if is_transaction_big {
            self.spinner.show("Parsing transaction...");
        }
    }
}

pub fn setup_sign_tx(req: TxMetadataReq, ctx: &mut DataContext) -> Result<(), StatusWord> {
    let mut tx_ctx = match req.version {
        TxMetadataVersionReq::V1(v1_req) => TxContext::from_v1(req.coin, v1_req)?,
    };

    tx_ctx.show_spinner();

    // if the tx has many outputs use a streaming review
    if tx_ctx.num_outputs > 10 {
        *ctx = DataContext::TxContext(tx_ctx, Review::StreamingReview(new_streaming_review()));
    } else {
        *ctx = DataContext::TxContext(tx_ctx, Review::Review(Vec::new()));
    }

    Ok(())
}

fn handle_input_commitment_req<'a>(
    req: SighashInputCommitment,
    input_step: u32,
    input_commitments_hash: [u8; 64],
    ctx: &mut TxContext,
    review: &'a mut Review,
) -> Result<SigningState<'a>, StatusWord> {
    ctx.update_input_commitments_hash(&req)?;
    ctx.update_hash(&req)?;
    ctx.advance_next_input_additional_info_step(input_step, input_commitments_hash, review)
}

fn handle_input_req<'a>(
    req: TxInputReq,
    input_step: u32,
    ctx: &mut TxContext,
) -> Result<SigningState<'a>, StatusWord> {
    let addresses = req
        .addresses
        .into_iter()
        .map(|a| InputCompressed::new(a, input_step, ctx.coin))
        .collect::<Result<Vec<_>, StatusWord>>()?;
    ctx.inputs.extend(addresses);

    process_input(ctx, &req.inp)?;

    let (input, commitment) = req.inp.into_input_and_commitment();
    ctx.update_input_commitments_hash(&commitment)?;

    if input_step == 0 {
        ctx.tx_hasher
            .update(&ctx.num_inputs.to_le_bytes())
            .map_err(|_| StatusWord::TxHashFail)?;
    }
    ctx.update_hash(&input)?;
    ctx.advance_next_input_step(input_step)
}

fn handle_output_req<'a>(
    req: TxOutputReq,
    output_step: u32,
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
                NextTxOutputParsingState::CompleteNotApproved { inp_idx, sighash } => {
                    SigningState::CompleteNotApproved {
                        inp_idx,
                        sighash,
                        outputs,
                    }
                }
            }
        }
        Review::StreamingReview(review) => {
            // on last output show it and ask for approval
            match next_step {
                NextTxOutputParsingState::Output(_) => {
                    SigningState::StreamingReviewOutput(review, req.out)
                }
                NextTxOutputParsingState::CompleteNotApproved { inp_idx, sighash } => {
                    SigningState::StreamingReviewApprove {
                        review,
                        output: req.out,
                        inp_idx,
                        sighash,
                    }
                }
            }
        }
    };

    Ok(signin_state)
}

pub fn handle_sign_tx(
    req: SignTxReq,
    ctx: &mut TxContext,
    review: &mut Review,
) -> Result<Response, StatusWord> {
    let signing_state = match (req, ctx.state()) {
        (SignTxReq::Input(req), TxParsingState::Input(n)) => handle_input_req(req, *n, ctx)?,
        (
            SignTxReq::InputCommitment(req),
            TxParsingState::InputCommitment {
                inp_idx,
                input_commitments_hash,
            },
        ) => handle_input_commitment_req(req, *inp_idx, *input_commitments_hash, ctx, review)?,
        (SignTxReq::Output(req), TxParsingState::Output(n)) => {
            handle_output_req(req, *n, ctx, review)?
        }
        (
            SignTxReq::NextSignature,
            TxParsingState::ApprovedNotFinishedSigning { inp_idx, sighash },
        ) => SigningState::ApprovedNotFinishedSigning {
            inp_idx: *inp_idx,
            sighash: *sighash,
        },
        (
            SignTxReq::NextSignature,
            TxParsingState::CompleteNotApproved {
                inp_idx: _,
                sighash: _,
            },
        ) => return Err(StatusWord::Deny),
        (SignTxReq::NextSignature, TxParsingState::Finished) => {
            return Err(StatusWord::TxAlreadyFinished)
        }
        (_, _) => return Err(StatusWord::WrongP1P2),
    };

    match signing_state {
        SigningState::TxParsingNotComplete => Ok(Response::TxNext),
        SigningState::StreamingReviewStart(review) => {
            if start_streaming_review(review) {
                Ok(Response::TxNext)
            } else {
                ctx.review_finished = true;
                Err(StatusWord::Deny)
            }
        }
        SigningState::StreamingReviewOutput(review, output) => {
            if streaming_review_show_output(review, &output, ctx.coin)? {
                Ok(Response::TxNext)
            } else {
                ctx.review_finished = true;
                Err(StatusWord::Deny)
            }
        }
        SigningState::StreamingReviewApprove {
            review,
            output,
            inp_idx,
            sighash,
        } => {
            if approve_streaming_review(review, &output, ctx)? {
                let response = compute_signature_and_append(ctx, inp_idx, &sighash)?;
                if ctx.completed_all_signatures() {
                    ctx.review_finished = true;
                } else {
                    ctx.show_spinner();
                }
                Ok(Response::TxSignature(response))
            } else {
                ctx.review_finished = true;
                Err(StatusWord::Deny)
            }
        }
        SigningState::CompleteNotApproved {
            inp_idx,
            sighash,
            outputs,
        } => {
            // Display transaction. If user approves the transaction, sign it.
            // Otherwise, return a "deny" status word.
            if ui_display_tx(ctx, outputs)? {
                let response = compute_signature_and_append(ctx, inp_idx, &sighash)?;
                if ctx.completed_all_signatures() {
                    ctx.review_finished = true;
                } else {
                    ctx.show_spinner();
                }

                Ok(Response::TxSignature(response))
            } else {
                ctx.review_finished = true;
                Err(StatusWord::Deny)
            }
        }
        SigningState::ApprovedNotFinishedSigning { inp_idx, sighash } => {
            // Already approved sign and return the next signature
            let response = compute_signature_and_append(ctx, inp_idx, &sighash)?;
            if ctx.completed_all_signatures() {
                ctx.review_finished = true;
            } else {
                ctx.show_spinner();
            }

            Ok(Response::TxSignature(response))
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

fn process_input(ctx: &mut TxContext, inp: &TxInputWithAdditionalInfo) -> Result<(), StatusWord> {
    match inp {
        TxInputWithAdditionalInfo::Utxo(_, info) => match info {
            AdditionalUtxoInfo::UtxoWithPoolData {
                utxo: _,
                staker_balance,
            } => {
                increase_input_totals(&mut ctx.total_inputs, CoinOrTokenId::Coin, *staker_balance)?;
            }
            AdditionalUtxoInfo::Utxo(utxo) => {
                match &utxo {
                    TxOutput::Transfer(value, _)
                    | TxOutput::LockThenTransfer(value, _, _)
                    | TxOutput::Htlc(value, _) => {
                        let (coin_or_token_id, amount) = into_coin_or_token_id_and_amount(&value)?;
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
                        increase_input_totals(
                            &mut ctx.total_inputs,
                            CoinOrTokenId::Coin,
                            data.pledge,
                        )?;
                    }
                    TxOutput::IssueNft(nft_id, _, _) => {
                        increase_input_totals(
                            &mut ctx.total_inputs,
                            CoinOrTokenId::TokenId(*nft_id.hash()),
                            Amount::from_atoms(1),
                        )?;
                    }
                };
            }
        },
        TxInputWithAdditionalInfo::Account(acc) => match acc.spending {
            AccountSpending::DelegationBalance(_, amount) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::DelegationWithdrawl);
                increase_input_totals(&mut ctx.total_inputs, CoinOrTokenId::Coin, amount)?;
            }
        },
        TxInputWithAdditionalInfo::AccountCommand(_, cmd) => match cmd {
            AccountCommand::MintTokens(token_id, amount) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::MintTokens);
                increase_input_totals(
                    &mut ctx.total_inputs,
                    CoinOrTokenId::TokenId(*token_id.hash()),
                    *amount,
                )?;
            }
            AccountCommand::ConcludeOrder(_) | AccountCommand::FillOrder(_, _, _) => {
                return Err(StatusWord::OrdersV0NotSupported)
            }
            AccountCommand::UnmintTokens(_) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::UnmintTokens);
            }
            AccountCommand::LockTokenSupply(_) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::LockTokenSupply);
            }
            AccountCommand::FreezeToken(_, _) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::FreezeToken);
            }
            AccountCommand::UnfreezeToken(_) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::UnfreezeToken);
            }
            AccountCommand::ChangeTokenAuthority(_, _) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::ChangeTokenAuthority);
            }
            AccountCommand::ChangeTokenMetadataUri(_, _) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::ChangeTokenMetadataUri);
            }
        },
        TxInputWithAdditionalInfo::OrderAccountCommand(
            cmd,
            AdditionalOrderInfo {
                initially_asked,
                initially_given,
                ask_balance,
                give_balance,
            },
        ) => match cmd {
            OrderAccountCommand::FillOrder(_, fill_amount) => {
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
                    .ok_or(StatusWord::TxNumericOperationFail)?
                    .checked_div(asked_amount.into_atoms())
                    .ok_or(StatusWord::TxNumericOperationFail)?;
                let amount = Amount::from_atoms(atoms);
                increase_input_totals(&mut ctx.total_inputs, given_coin_or_token_id, amount)?;

                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::FillOrder);
            }
            OrderAccountCommand::ConcludeOrder(_) => {
                let (coin_or_token_id, _) = into_coin_or_token_id_and_amount(&initially_asked)?;
                increase_input_totals(&mut ctx.total_inputs, coin_or_token_id, *ask_balance)?;

                let (coin_or_token_id, _) = into_coin_or_token_id_and_amount(&initially_given)?;
                increase_input_totals(&mut ctx.total_inputs, coin_or_token_id, *give_balance)?;

                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::ConcludeOrder);
            }
            OrderAccountCommand::FreezeOrder(_) => {
                ctx.tx_type = merge_tx_type(ctx.tx_type, TxType::FreezeOrder);
            }
        },
    };

    Ok(())
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
    ctx: &mut TxContext,
    inp_idx: u32,
    sighash: &H256,
) -> Result<TxInputSignatureResponse, StatusWord> {
    let address = ctx
        .inputs
        .get(inp_idx as usize)
        .ok_or(StatusWord::WrongContext)?;

    let [p1, p2, p3] = address.path;
    let addr = [BIP44, ctx.coin.bip44_coin_type(), p1, p2, p3];

    let private_key = Secp256k1::derive_from_path(&addr);
    let sig = schnorr_sign(&private_key, sighash.as_bytes())?;

    let signature = Signature(sig);
    let input_idx = address.input_idx;
    let multisig_idx = address.multisig_idx;

    ctx.advance_next_signing_step(inp_idx, sighash);
    let response = TxInputSignatureResponse {
        signature,
        multisig_idx,
        input_idx,
        has_next: ctx.state != TxParsingState::Finished,
    };

    Ok(response)
}

fn update_hash<T: Encode>(
    raw_buf: &mut Vec<u8>,
    data: &T,
    hasher: &mut Blake2b_512,
) -> Result<(), StatusWord> {
    raw_buf.clear();
    encode_to(data, raw_buf);
    hasher
        .update(raw_buf.as_slice())
        .map_err(|_| StatusWord::TxHashFail)?;
    raw_buf.clear();
    Ok(())
}
