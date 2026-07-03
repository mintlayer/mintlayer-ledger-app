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
    nbgl::{NbglSpinner, NbglStreamingReview},
};

use mintlayer_messages::{
    H256, InputAddressPath, OrderAccountCommand, Response, SignTxNextReq, SignTxStartReq,
    Signature, TransactionVersion, TxInputCommitmentData, TxInputData, TxInputSignatureResponse,
    TxInputWithAdditionalInfo, TxOutputData, encode_as_compact_to, encode_to,
};

use crate::{
    DataContext, StatusWord,
    app_ui::sign::{
        ui_approve_streaming_review, ui_new_streaming_review, ui_start_streaming_review,
        ui_streaming_review_show_input, ui_streaming_review_show_output,
    },
    handlers::sign_message::schnorr_sign,
    hasher::Hasher,
    mlcp,
    utils::{CompressedDerivationPathForTxSigning, check_derivation_path_for_tx_signing},
};

mod summary_collector;

pub use summary_collector::{CoinOrTokenId, InputCommand, TxSummaryCollector, TxType};

// u32 is enough to cover max possible number of inputs and outputs (note that usize is also
// 32-bit on Ledger, but we want to be specific about size)
type Index = u32;

/// A "signature target". Usually, one input will produce one SigTarget (more than one SigTarget
/// is possible in the case of multisig; it can also be zero for non-signable pseudo-inputs, such
/// as FillOrder).
pub struct SigTarget {
    pub path: CompressedDerivationPathForTxSigning,
    pub input_idx: Index,
    pub multisig_idx: Option<Index>,
}

impl SigTarget {
    fn new(
        addr: InputAddressPath,
        input_idx: Index,
        coin_type: mlcp::CoinType,
    ) -> Result<Self, StatusWord> {
        let path = check_derivation_path_for_tx_signing(addr.path.as_ref(), coin_type)?;

        Ok(Self {
            path,
            input_idx,
            multisig_idx: addr.multisig_idx,
        })
    }
}

pub struct TxMetadata {
    coin_type: mlcp::CoinType,
    num_inputs: Index,
    num_outputs: Index,
}

pub struct TxInputsProcessingContext {
    metadata: TxMetadata,

    tx_hasher: Hasher,

    // Note: input commitments have to be sent together with the inputs, because they contain
    // the actual amounts that the inputs consume. But they can't be put into the transaction hasher
    // until all inputs have been processed, so they'll have to be sent again via a separate pass.
    // We hash the commitments to ensure that the same ones are sent during both passes.
    input_commitments_hasher: Hasher,

    summary: TxSummaryCollector,
    sig_targets: Vec<SigTarget>,

    spinner: NbglSpinner,

    num_inputs_parsed: Index,
}

impl TxInputsProcessingContext {
    fn advance_next_input_step(mut self: Box<Self>) -> Result<TxProcessingContext, StatusWord> {
        self.num_inputs_parsed += 1;
        let finished_with_inputs = self.num_inputs_parsed >= self.metadata.num_inputs;

        if finished_with_inputs {
            if self.sig_targets.is_empty() {
                return Err(StatusWord::NothingToSign);
            }

            // Update hash for input commitments and proceed with outputs
            self.tx_hasher
                .update(&self.metadata.num_inputs.to_le_bytes());

            let input_commitments_hash = self.input_commitments_hasher.finalize()?;

            Ok(TxProcessingContext::ProcessingInputCommitments(Box::new(
                TxInputCommitmentsProcessingContext {
                    metadata: self.metadata,
                    tx_hasher: self.tx_hasher,
                    input_commitments_hasher: Hasher::new(),
                    expected_input_commitments_hash: input_commitments_hash,
                    summary: self.summary,
                    sig_targets: self.sig_targets,
                    spinner: self.spinner,
                    num_inputs_parsed: 0,
                },
            )))
        } else {
            Ok(TxProcessingContext::ProcessingInputs(self))
        }
    }
}

pub struct TxInputCommitmentsProcessingContext {
    metadata: TxMetadata,

    tx_hasher: Hasher,
    input_commitments_hasher: Hasher,
    expected_input_commitments_hash: H256,

    summary: TxSummaryCollector,
    sig_targets: Vec<SigTarget>,

    spinner: NbglSpinner,

    num_inputs_parsed: Index,
}

impl TxInputCommitmentsProcessingContext {
    fn advance_next_input_additional_info_step(
        mut self: Box<Self>,
        review: &NbglStreamingReview,
    ) -> Result<TxProcessingContext, StatusWord> {
        self.num_inputs_parsed += 1;

        let finished_with_inputs = self.num_inputs_parsed >= self.metadata.num_inputs;

        if finished_with_inputs {
            // Make sure the hashes match before continuing with the outputs
            let input_commitments_hash = self.input_commitments_hasher.finalize()?;

            if input_commitments_hash != self.expected_input_commitments_hash {
                return Err(StatusWord::DifferentInputCommitmentHash);
            }

            if !ui_start_streaming_review(review) {
                return Err(StatusWord::Deny);
            }

            if let Some(command) = self.summary.input_command()
                && !ui_streaming_review_show_input(review, command, self.metadata.coin_type)?
            {
                return Err(StatusWord::Deny);
            }

            encode_as_compact_to(self.metadata.num_outputs, &mut self.tx_hasher);

            if self.metadata.num_outputs > 0 {
                let new_context =
                    TxProcessingContext::ProcessingOutputs(Box::new(TxOutputsProcessingContext {
                        metadata: self.metadata,
                        tx_hasher: self.tx_hasher,
                        summary: self.summary,
                        sig_targets: self.sig_targets,
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
                    self.sig_targets,
                    self.spinner,
                )
            }
        } else {
            Ok(TxProcessingContext::ProcessingInputCommitments(self))
        }
    }
}

pub struct TxOutputsProcessingContext {
    metadata: TxMetadata,

    tx_hasher: Hasher,

    summary: TxSummaryCollector,
    sig_targets: Vec<SigTarget>,

    spinner: NbglSpinner,

    num_outputs_parsed: Index,
}

impl TxOutputsProcessingContext {
    pub fn coin_type(&self) -> mlcp::CoinType {
        self.metadata.coin_type
    }

    pub fn summary(&self) -> &TxSummaryCollector {
        &self.summary
    }

    fn advance_next_output_state(
        mut self: Box<Self>,
        review: &NbglStreamingReview,
    ) -> Result<TxProcessingContext, StatusWord> {
        if self.num_outputs_parsed < (self.metadata.num_outputs - 1) {
            self.num_outputs_parsed += 1;
            Ok(TxProcessingContext::ProcessingOutputs(self))
        } else {
            switch_to_signing(
                review,
                self.tx_hasher,
                self.summary,
                self.metadata,
                self.sig_targets,
                self.spinner,
            )
        }
    }
}

fn switch_to_signing(
    review: &NbglStreamingReview,
    tx_hasher: Hasher,
    summary: TxSummaryCollector,
    metadata: TxMetadata,
    sig_targets: Vec<SigTarget>,
    spinner: NbglSpinner,
) -> Result<TxProcessingContext, StatusWord> {
    if ui_approve_streaming_review(review, &summary, metadata.coin_type)? {
        // Finalize the tx hash for signing
        let first_hash = tx_hasher.finalize()?;
        let tx_hash = Hasher::hash(first_hash.as_bytes())?;

        Ok(TxProcessingContext::Signing(Box::new(TxSigningContext {
            metadata,
            sig_targets,
            spinner,
            num_sigs_produced: 0,
            tx_hash,
        })))
    } else {
        Err(StatusWord::Deny)
    }
}

pub struct TxSigningContext {
    metadata: TxMetadata,
    tx_hash: H256,

    sig_targets: Vec<SigTarget>,

    spinner: NbglSpinner,

    num_sigs_produced: Index,
}

impl TxSigningContext {
    fn compute_signature_and_append(
        mut self: Box<Self>,
    ) -> Result<(TxInputSignatureResponse, TxProcessingContext), StatusWord> {
        let sig_target = self
            .sig_targets
            .get(self.num_sigs_produced as usize)
            .ok_or(StatusWord::WrongContext)?;

        let path = sig_target.path.to_full_path(self.metadata.coin_type);
        let private_key = Secp256k1::derive_from_path(&path);
        let sig = schnorr_sign(&private_key, self.tx_hash.as_bytes())?;

        let signature = Signature(sig);
        let input_idx = sig_target.input_idx;
        let multisig_idx = sig_target.multisig_idx;

        let has_next = ((self.num_sigs_produced + 1) as usize) < self.sig_targets.len();

        let response = TxInputSignatureResponse {
            signature,
            multisig_idx,
            input_idx,
            has_next,
        };

        let new_ctx = if has_next {
            self.num_sigs_produced += 1;
            TxProcessingContext::Signing(self)
        } else {
            TxProcessingContext::Finished
        };

        Ok((response, new_ctx))
    }
}

pub enum TxProcessingContext {
    ProcessingInputs(Box<TxInputsProcessingContext>),
    ProcessingInputCommitments(Box<TxInputCommitmentsProcessingContext>),
    ProcessingOutputs(Box<TxOutputsProcessingContext>),
    Signing(Box<TxSigningContext>),
    Finished,
}

impl TxProcessingContext {
    pub fn new(
        SignTxStartReq {
            coin_type,
            version,
            num_inputs,
            num_outputs,
        }: SignTxStartReq,
    ) -> Result<Self, StatusWord> {
        if num_inputs == 0 {
            return Err(StatusWord::TxWithZeroInputs);
        }

        match version {
            TransactionVersion::V1 => {
                const VERSION_1: u8 = 1;
                const SIG_HASH_TYPE_ALL: u8 = 1;

                let mut tx_hasher = Hasher::new();
                // mode
                tx_hasher.update(&[SIG_HASH_TYPE_ALL]);
                // version
                tx_hasher.update(&[VERSION_1]);
                // flags
                tx_hasher.update(&[0; 16]);

                tx_hasher.update(&num_inputs.to_le_bytes());

                Ok(Self::ProcessingInputs(Box::new(
                    TxInputsProcessingContext {
                        metadata: TxMetadata {
                            coin_type: coin_type.into(),
                            num_inputs,
                            num_outputs,
                        },
                        tx_hasher,
                        spinner: NbglSpinner::new(),
                        summary: TxSummaryCollector::new(),
                        num_inputs_parsed: 0,
                        input_commitments_hasher: Hasher::new(),
                        sig_targets: Vec::new(),
                    },
                )))
            }
        }
    }

    /// Shows a spinner while processing the inputs and input commitments if there are more than a few,
    /// as well as while signing and returning the signatures.
    pub fn show_spinner(&mut self) {
        let (metadata, spinner) = match self {
            Self::ProcessingInputs(ctx) => (&ctx.metadata, &mut ctx.spinner),
            Self::ProcessingInputCommitments(ctx) => (&ctx.metadata, &mut ctx.spinner),
            Self::Signing(ctx) => {
                ctx.spinner.show("Signing...");
                return;
            }
            // While parsing outputs we are showing the review and not the spinner.

            // TODO: inputs may need to be reviewed one by one, just like outputs.
            // See https://github.com/mintlayer/mintlayer-ledger-app/issues/14.
            // Also see the TODO near `TxSummaryCollector::input_command`.

            // TODO: change outputs should be detected and not presented for review; once this is
            // implemented, the spinner logic may need to be revised.
            // See https://github.com/mintlayer/mintlayer-ledger-app/issues/17.
            Self::ProcessingOutputs(_) | Self::Finished => return,
        };

        // We show a spinner while processing the inputs and input commitments if there are more than 5;
        // 5 was chosen somewhat arbitrarily
        let transaction_has_many_inputs = metadata.num_inputs > 5;

        if transaction_has_many_inputs {
            spinner.show("Processing transaction...");
        }
    }

    pub fn finished(&self) -> bool {
        matches!(self, Self::Finished)
    }
}

pub fn setup_sign_tx(req: SignTxStartReq) -> Result<DataContext, StatusWord> {
    let mut tx_ctx = TxProcessingContext::new(req)?;

    tx_ctx.show_spinner();

    Ok(DataContext::TxContext(tx_ctx, ui_new_streaming_review()))
}

fn handle_input(
    input_data: Box<TxInputData>,
    mut ctx: Box<TxInputsProcessingContext>,
) -> Result<TxProcessingContext, StatusWord> {
    // FillOrder inputs are pseudo-inputs that should not be signed; if the host requests
    // a signature in such a case, it is doing something wrong, so we reject it.
    // Note that we only check for V1 orders here, since V0 are not supported by the app
    // (see StatusWord::OrdersV0NotSupported).
    if is_v1_fill_order_input(&input_data.input) && !input_data.addresses.is_empty() {
        return Err(StatusWord::FillOrderSigRequested);
    }

    let num_inputs_parsed = ctx.num_inputs_parsed;
    let sig_targets = input_data
        .addresses
        .into_iter()
        .map(|a| SigTarget::new(a, num_inputs_parsed, ctx.metadata.coin_type))
        .collect::<Result<Vec<_>, StatusWord>>()?;
    ctx.sig_targets.extend(sig_targets);

    ctx.summary.process_input(&input_data.input)?;

    let (input, commitment) = input_data.input.into_input_and_commitment();
    encode_to(&commitment, &mut ctx.input_commitments_hasher);
    encode_to(&input, &mut ctx.tx_hasher);
    ctx.advance_next_input_step()
}

fn is_v1_fill_order_input(input: &TxInputWithAdditionalInfo) -> bool {
    match input {
        TxInputWithAdditionalInfo::OrderAccountCommand(cmd, _) => match cmd {
            OrderAccountCommand::FillOrder(_, _) => true,
            OrderAccountCommand::FreezeOrder(_) | OrderAccountCommand::ConcludeOrder(_) => false,
        },
        TxInputWithAdditionalInfo::Utxo(_, _)
        | TxInputWithAdditionalInfo::Account(_)
        | TxInputWithAdditionalInfo::AccountCommand(_, _) => false,
    }
}

fn handle_input_commitment(
    comm_data: &TxInputCommitmentData,
    mut ctx: Box<TxInputCommitmentsProcessingContext>,
    review: &NbglStreamingReview,
) -> Result<TxProcessingContext, StatusWord> {
    encode_to(&comm_data.commitment, &mut ctx.input_commitments_hasher);
    encode_to(&comm_data.commitment, &mut ctx.tx_hasher);
    ctx.advance_next_input_additional_info_step(review)
}

fn handle_output(
    output_data: &TxOutputData,
    mut ctx: Box<TxOutputsProcessingContext>,
    review: &NbglStreamingReview,
) -> Result<TxProcessingContext, StatusWord> {
    if ui_streaming_review_show_output(review, &output_data.output, ctx.metadata.coin_type)? {
        ctx.summary.process_output(&output_data.output)?;
        encode_to(&output_data.output, &mut ctx.tx_hasher);
        ctx.advance_next_output_state(review)
    } else {
        Err(StatusWord::Deny)
    }
}

pub fn handle_sign_tx(
    req: SignTxNextReq,
    ctx: TxProcessingContext,
    review: &mut NbglStreamingReview,
) -> Result<(Response, TxProcessingContext), StatusWord> {
    match (req, ctx) {
        (SignTxNextReq::ProcessInput(req), TxProcessingContext::ProcessingInputs(ctx)) => {
            let new_ctx = handle_input(req, ctx)?;
            Ok((Response::TxNext, new_ctx))
        }
        (
            SignTxNextReq::ProcessInputCommitment(req),
            TxProcessingContext::ProcessingInputCommitments(ctx),
        ) => {
            let new_ctx = handle_input_commitment(req.as_ref(), ctx, review)?;
            Ok((Response::TxNext, new_ctx))
        }
        (SignTxNextReq::ProcessOutput(req), TxProcessingContext::ProcessingOutputs(ctx)) => {
            let new_ctx = handle_output(req.as_ref(), ctx, review)?;
            Ok((Response::TxNext, new_ctx))
        }
        (SignTxNextReq::ReturnNextSignature, TxProcessingContext::Signing(ctx)) => {
            let (response, mut new_ctx) = ctx.compute_signature_and_append()?;
            new_ctx.show_spinner();

            Ok((Response::TxInputSignature(response), new_ctx))
        }
        (SignTxNextReq::ReturnNextSignature, TxProcessingContext::Finished) => {
            Err(StatusWord::TxAlreadyFinished)
        }
        _ => Err(StatusWord::WrongContext),
    }
}
