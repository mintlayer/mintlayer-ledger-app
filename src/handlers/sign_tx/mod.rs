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
        Amount, CoinType as PCoinType,
        SighashInputCommitment, TxOutput, H256,
    },
    CoinType, Encode, InputAddressPath, Response,
    SignTxReq, Signature, TxInputReq, TxInputSignatureResponse,
    TxMetadataReq, TxMetadataV1Req, TxMetadataVersionReq, TxOutputReq,
};

use ledger_device_sdk::{
    ecc::{Secp256k1, SeedDerive},
    hash::{blake2::Blake2b_512, HashInit},
    nbgl::{NbglSpinner, NbglStreamingReview},
};

mod summary_collector;
use summary_collector::TxSummaryCollector;
pub use summary_collector::{TxType, CoinOrTokenId};

const BIP44: u32 = 44 + (1 << 31);

// BIP44/COIN/ACCOUNT/PURPOSE/INDEX
const DERIVATION_PATH_LEN: usize = 5;
// DERIVATION_PATH_LEN without the BIP44 and COIN as they are the same for all
const COMPRESSED_DERIVATION_PATH_LEN: usize = 3;

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

    tx_hasher: Blake2b_512,
    input_commitments_hasher: Blake2b_512,

    summary: TxSummaryCollector,
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
            summary: TxSummaryCollector::new(),
            inputs: Vec::with_capacity(20),
            spinner: NbglSpinner::new(),
        })
    }

    pub fn coin(&self) -> PCoinType {
        self.coin
    }

    pub fn summary(&self) -> &TxSummaryCollector {
        &self.summary
    }

    pub fn summary_mut(&mut self) -> &mut TxSummaryCollector {
        &mut self.summary
    }

    pub fn tx_type(&self) -> Option<TxType> {
        self.summary.tx_type()
    }

    pub fn total_inputs(&self) -> &BTreeMap<CoinOrTokenId, Amount> {
        self.summary.total_inputs()
    }

    pub fn total_outputs(&self) -> &BTreeMap<CoinOrTokenId, Amount> {
        self.summary.total_outputs()
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

    ctx.summary.process_input(&req.inp)?;

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
    ctx.summary.process_output(&req.out)?;
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
