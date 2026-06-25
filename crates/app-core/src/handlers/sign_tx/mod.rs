/*****************************************************************************
 *   Mintlayer Ledger App.
 *   (c) 2025-2026 RBB S.r.l.
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

use alloc::{boxed::Box, vec::Vec};

use ledger_device_sdk::{
    ecc::{Secp256k1, SeedDerive},
    hash::{blake2::Blake2b_512, HashInit},
    nbgl::{NbglSpinner, NbglStreamingReview},
};

use mintlayer_messages::{
    encode_as_compact, encode_to, Encode, InputAddressPath, Response, SignTxNextReq,
    SignTxStartReq, Signature, TransactionVersion, TxInputCommitmentData, TxInputData,
    TxInputSignatureResponse, TxOutputData, H256,
};

use crate::{
    app_ui::sign::{
        ui_approve_streaming_review, ui_new_streaming_review, ui_start_streaming_review,
        ui_streaming_review_show_input, ui_streaming_review_show_output,
    },
    handlers::{sign_message::schnorr_sign, utils::mintlayer_hash},
    mlcp,
    utils::{check_derivation_path_for_tx_signing, CompressedDerivationPathForTxSigning},
    DataContext, StatusWord,
};

mod summary_collector;

pub use summary_collector::{CoinOrTokenId, InputCommand, TxSummaryCollector, TxType};

// FIXME: usize is already 32-bit.
// we try to save a few bytes instead of using usize for indexes,
// u32 is enough to cover max possible number of inputs and outputs
type Index = u32;

pub struct InputCompressed {
    pub path: CompressedDerivationPathForTxSigning,
    pub input_idx: Index,
    pub multisig_idx: Option<Index>,
}

impl InputCompressed {
    fn new(
        addr: InputAddressPath,
        input_idx: Index,
        coin: mlcp::CoinType,
    ) -> Result<Self, StatusWord> {
        let path = check_derivation_path_for_tx_signing(addr.path.as_ref(), coin)?;

        Ok(Self {
            path,
            input_idx,
            multisig_idx: addr.multisig_idx,
        })
    }
}

pub struct TxMetadata {
    coin: mlcp::CoinType,
    num_inputs: Index,
    num_outputs: Index,
}

// FIXME: rename this and other types, so that the names are based on "process" rather than "parse".
pub struct TxParsingInputsContext {
    metadata: TxMetadata,

    tx_hasher: Blake2b_512,

    // Note: input commitments have to be sent together with the inputs, because they contain
    // the actual amounts that the inputs consume. But they can't be put into the transaction hasher
    // until all inputs have been processed, so they'll have to be sent again via a separate pass.
    // We hash the commitments to ensure that the same ones are sent during both passes.
    input_commitments_hasher: Blake2b_512,

    summary: TxSummaryCollector,
    inputs: Vec<InputCompressed>,

    spinner: NbglSpinner,

    num_inputs_parsed: Index,
}

pub struct TxParsingInputCommitmentsContext {
    metadata: TxMetadata,

    tx_hasher: Blake2b_512,
    input_commitments_hasher: Blake2b_512,
    expected_input_commitments_hash: [u8; 64],

    summary: TxSummaryCollector,
    inputs: Vec<InputCompressed>,

    spinner: NbglSpinner,

    num_inputs_parsed: Index,
}

impl TxParsingInputCommitmentsContext {
    fn advance_next_input_additional_info_step(
        mut self: Box<Self>,
        review: &NbglStreamingReview,
    ) -> Result<TxParsingContext, StatusWord> {
        let finished_with_inputs = self.num_inputs_parsed >= (self.metadata.num_inputs - 1);

        if finished_with_inputs {
            // Make sure the hashes match before continuing with the outputs
            let mut input_commitments_hash: [u8; 64] = [0u8; 64];
            self.input_commitments_hasher
                .finalize(&mut input_commitments_hash)
                .map_err(|_| StatusWord::TxHashFail)?;

            if input_commitments_hash != self.expected_input_commitments_hash {
                return Err(StatusWord::DifferentInputCommitmentHash);
            }

            if !ui_start_streaming_review(review) {
                return Err(StatusWord::Deny);
            }

            if let Some(command) = self.summary.input_command() {
                if !ui_streaming_review_show_input(review, command, self.metadata.coin)? {
                    return Err(StatusWord::Deny);
                }
            }

            self.tx_hasher
                .update(&encode_as_compact(self.metadata.num_outputs))
                .map_err(|_| StatusWord::TxHashFail)?;
            if self.metadata.num_outputs > 0 {
                let new_context =
                    TxParsingContext::ParsingOutputs(Box::new(TxParsingOutputsContext {
                        metadata: self.metadata,
                        tx_hasher: self.tx_hasher,
                        summary: self.summary,
                        inputs: self.inputs,
                        spinner: self.spinner,
                        num_outputs_parsed: 0,
                    }));
                Ok(new_context)
            } else {
                switch_to_signing(
                    review,
                    self.tx_hasher,
                    self.summary,
                    self.metadata,
                    self.inputs,
                    self.spinner,
                )
            }
        } else {
            self.num_inputs_parsed += 1;
            Ok(TxParsingContext::ParsingInputCommitments(self))
        }
    }
}

impl TxParsingInputsContext {
    fn advance_next_input_step(mut self: Box<Self>) -> Result<TxParsingContext, StatusWord> {
        self.num_inputs_parsed += 1;
        let finished_with_inputs = self.num_inputs_parsed >= self.metadata.num_inputs;

        if finished_with_inputs {
            if self.inputs.is_empty() {
                return Err(StatusWord::NothingToSign);
            }

            // Update hash for input commitments and proceed with outputs
            self.tx_hasher
                .update(&self.metadata.num_inputs.to_le_bytes())
                .map_err(|_| StatusWord::TxHashFail)?;

            let mut input_commitments_hash: [u8; 64] = [0u8; 64];
            self.input_commitments_hasher
                .finalize(&mut input_commitments_hash)
                .map_err(|_| StatusWord::TxHashFail)?;

            Ok(TxParsingContext::ParsingInputCommitments(Box::new(
                TxParsingInputCommitmentsContext {
                    metadata: self.metadata,
                    tx_hasher: self.tx_hasher,
                    input_commitments_hasher: Blake2b_512::new(),
                    expected_input_commitments_hash: input_commitments_hash,
                    summary: self.summary,
                    inputs: self.inputs,
                    spinner: self.spinner,
                    num_inputs_parsed: 0,
                },
            )))
        } else {
            Ok(TxParsingContext::ParsingInputs(self))
        }
    }
}

pub struct TxParsingOutputsContext {
    metadata: TxMetadata,

    tx_hasher: Blake2b_512,

    summary: TxSummaryCollector,
    inputs: Vec<InputCompressed>,

    spinner: NbglSpinner,

    num_outputs_parsed: Index,
}

impl TxParsingOutputsContext {
    pub fn coin(&self) -> mlcp::CoinType {
        self.metadata.coin
    }

    pub fn summary(&self) -> &TxSummaryCollector {
        &self.summary
    }

    fn advance_next_output_state(
        mut self: Box<Self>,
        review: &NbglStreamingReview,
    ) -> Result<TxParsingContext, StatusWord> {
        if self.num_outputs_parsed < (self.metadata.num_outputs - 1) {
            self.num_outputs_parsed += 1;
            Ok(TxParsingContext::ParsingOutputs(self))
        } else {
            switch_to_signing(
                review,
                self.tx_hasher,
                self.summary,
                self.metadata,
                self.inputs,
                self.spinner,
            )
        }
    }
}

fn switch_to_signing(
    review: &NbglStreamingReview,
    mut tx_hasher: Blake2b_512,
    summary: TxSummaryCollector,
    metadata: TxMetadata,
    inputs: Vec<InputCompressed>,
    spinner: NbglSpinner,
) -> Result<TxParsingContext, StatusWord> {
    if ui_approve_streaming_review(review, &summary, metadata.coin)? {
        // Finalize the tx hash for signing
        let mut message_hash: [u8; 64] = [0u8; 64];
        tx_hasher
            .finalize(&mut message_hash)
            .map_err(|_| StatusWord::TxHashFail)?;

        let tx_hash = mintlayer_hash(&message_hash[0..32])?;

        Ok(TxParsingContext::Signing(Box::new(TxSigningContext {
            metadata,
            inputs,
            spinner,
            num_inputs_signed: 0,
            tx_hash,
        })))
    } else {
        Err(StatusWord::Deny)
    }
}

pub struct TxSigningContext {
    metadata: TxMetadata,
    tx_hash: H256,

    inputs: Vec<InputCompressed>,

    spinner: NbglSpinner,

    num_inputs_signed: Index,
}

impl TxSigningContext {
    fn compute_signature_and_append(
        mut self: Box<Self>,
    ) -> Result<(TxInputSignatureResponse, TxParsingContext), StatusWord> {
        let address = self
            .inputs
            .get(self.num_inputs_signed as usize)
            .ok_or(StatusWord::WrongContext)?;

        let addr = address.path.to_full_path(self.metadata.coin);
        let private_key = Secp256k1::derive_from_path(&addr);
        let sig = schnorr_sign(&private_key, self.tx_hash.as_bytes())?;

        let signature = Signature(sig);
        let input_idx = address.input_idx;
        let multisig_idx = address.multisig_idx;

        let has_next = ((self.num_inputs_signed + 1) as usize) < self.inputs.len();

        let response = TxInputSignatureResponse {
            signature,
            multisig_idx,
            input_idx,
            has_next,
        };

        let new_ctx = if has_next {
            self.num_inputs_signed += 1;
            TxParsingContext::Signing(self)
        } else {
            TxParsingContext::Finished
        };

        Ok((response, new_ctx))
    }
}

pub enum TxParsingContext {
    ParsingInputs(Box<TxParsingInputsContext>),
    ParsingInputCommitments(Box<TxParsingInputCommitmentsContext>),
    ParsingOutputs(Box<TxParsingOutputsContext>),
    Signing(Box<TxSigningContext>),
    Finished,
}

impl TxParsingContext {
    pub fn new(
        SignTxStartReq {
            coin,
            version,
            num_inputs,
            num_outputs,
        }: SignTxStartReq,
    ) -> Result<Self, StatusWord> {
        match version {
            TransactionVersion::V1 => {
                const VERSION_1: u8 = 1;
                const SIG_HASH_TYPE_ALL: u8 = 1;

                let mut tx_hasher = Blake2b_512::new();
                // mode
                tx_hasher
                    .update(&[SIG_HASH_TYPE_ALL])
                    .map_err(|_| StatusWord::TxHashFail)?;
                // version
                tx_hasher
                    .update(&[VERSION_1])
                    .map_err(|_| StatusWord::TxHashFail)?;
                // flags
                tx_hasher
                    .update(&[0; 16])
                    .map_err(|_| StatusWord::TxHashFail)?;

                tx_hasher
                    .update(&num_inputs.to_le_bytes())
                    .map_err(|_| StatusWord::TxHashFail)?;

                Ok(Self::ParsingInputs(Box::new(TxParsingInputsContext {
                    metadata: TxMetadata {
                        coin: coin.into(),
                        num_inputs,
                        num_outputs,
                    },
                    tx_hasher,
                    spinner: NbglSpinner::new(),
                    summary: TxSummaryCollector::new(),
                    num_inputs_parsed: 0,
                    input_commitments_hasher: Blake2b_512::new(),
                    inputs: Vec::new(),
                })))
            }
        }
    }

    /// Shows a spinner while processing the inputs and input commitments if there are more than a few,
    /// as well as while signing and returning the signatures.
    pub fn show_spinner(&mut self) {
        let (metadata, spinner) = match self {
            Self::ParsingInputs(ctx) => (&ctx.metadata, &mut ctx.spinner),
            Self::ParsingInputCommitments(ctx) => (&ctx.metadata, &mut ctx.spinner),
            Self::Signing(ctx) => {
                ctx.spinner.show("Signing...");
                return;
            }
            // While parsing outputs we are showing the review and not the spinner.
            // FIXME: inputs may need to be reviewed one by one, just like outputs, see the FIXME near
            // TxSummaryCollector::input_command.
            // FIXME: change outputs should be detected and not presented for review; once this is
            // implemented, the spinner logic may need to be revised.
            Self::ParsingOutputs(_) | Self::Finished => return,
        };

        // We show a spinner while processing the inputs and input commitments if there are more than 5;
        // 5 was chosen somewhat arbitrarily
        let transaction_has_many_inputs = metadata.num_inputs > 5;

        if transaction_has_many_inputs {
            spinner.show("Parsing transaction...");
        }
    }

    pub fn finished(&self) -> bool {
        matches!(self, Self::Finished)
    }
}

pub fn setup_sign_tx(req: SignTxStartReq) -> Result<DataContext, StatusWord> {
    let mut tx_ctx = TxParsingContext::new(req)?;

    tx_ctx.show_spinner();

    Ok(DataContext::TxContext(tx_ctx, ui_new_streaming_review()))
}

fn handle_input(
    input_data: Box<TxInputData>,
    mut ctx: Box<TxParsingInputsContext>,
) -> Result<TxParsingContext, StatusWord> {
    let num_inputs_parsed = ctx.num_inputs_parsed;
    let compressed_inputs = input_data
        .addresses
        .into_iter()
        .map(|a| InputCompressed::new(a, num_inputs_parsed, ctx.metadata.coin))
        .collect::<Result<Vec<_>, StatusWord>>()?;
    // FIXME: `ctx.inputs` is not really a collection of inputs, as it can contain multiple entries
    // for one input in the case of multisig. Possible alternative name: "signature targets"
    // (need to rename InputCompressed as well).
    ctx.inputs.extend(compressed_inputs);

    ctx.summary.process_input(&input_data.input)?;

    let (input, commitment) = input_data.input.into_input_and_commitment();
    update_hash(&commitment, &mut ctx.input_commitments_hasher)?;
    update_hash(&input, &mut ctx.tx_hasher)?;
    ctx.advance_next_input_step()
}

fn handle_input_commitment(
    comm_data: &TxInputCommitmentData,
    mut ctx: Box<TxParsingInputCommitmentsContext>,
    review: &NbglStreamingReview,
) -> Result<TxParsingContext, StatusWord> {
    update_hash(&comm_data.commitment, &mut ctx.input_commitments_hasher)?;
    update_hash(&comm_data.commitment, &mut ctx.tx_hasher)?;
    ctx.advance_next_input_additional_info_step(review)
}

fn handle_output(
    output_data: &TxOutputData,
    mut ctx: Box<TxParsingOutputsContext>,
    review: &NbglStreamingReview,
) -> Result<TxParsingContext, StatusWord> {
    if ui_streaming_review_show_output(review, &output_data.output, ctx.metadata.coin)? {
        ctx.summary.process_output(&output_data.output)?;
        update_hash(&output_data.output, &mut ctx.tx_hasher)?;
        ctx.advance_next_output_state(review)
    } else {
        Err(StatusWord::Deny)
    }
}

pub fn handle_sign_tx(
    req: SignTxNextReq,
    ctx: TxParsingContext,
    review: &mut NbglStreamingReview,
) -> Result<(Response, TxParsingContext), StatusWord> {
    match (req, ctx) {
        (SignTxNextReq::ProcessInput(req), TxParsingContext::ParsingInputs(ctx)) => {
            let new_ctx = handle_input(req, ctx)?;
            Ok((Response::TxNext, new_ctx))
        }
        (
            SignTxNextReq::ProcessInputCommitment(req),
            TxParsingContext::ParsingInputCommitments(ctx),
        ) => {
            let new_ctx = handle_input_commitment(req.as_ref(), ctx, review)?;
            Ok((Response::TxNext, new_ctx))
        }
        (SignTxNextReq::ProcessOutput(req), TxParsingContext::ParsingOutputs(ctx)) => {
            let new_ctx = handle_output(req.as_ref(), ctx, review)?;
            Ok((Response::TxNext, new_ctx))
        }
        (SignTxNextReq::ReturnNextSignature, TxParsingContext::Signing(ctx)) => {
            let (response, mut new_ctx) = ctx.compute_signature_and_append()?;
            new_ctx.show_spinner();

            Ok((Response::TxInputSignature(response), new_ctx))
        }
        (SignTxNextReq::ReturnNextSignature, TxParsingContext::Finished) => {
            Err(StatusWord::TxAlreadyFinished)
        }
        _ => Err(StatusWord::WrongContext),
    }
}

// FIXME: this function is sometimes called twice in a row; re-use the buffer in such a case
// instead of reallocating it.
fn update_hash<T: Encode>(data: &T, hasher: &mut Blake2b_512) -> Result<(), StatusWord> {
    let mut buf = Vec::<u8>::new();
    encode_to(data, &mut buf);
    hasher
        .update(buf.as_slice())
        .map_err(|_| StatusWord::TxHashFail)?;
    Ok(())
}
