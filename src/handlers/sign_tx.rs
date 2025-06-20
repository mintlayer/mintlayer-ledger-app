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
use crate::app_ui::sign::{ui_display_message, ui_display_tx};
use crate::handlers::sign_message::schnorr_sign;
use crate::utils::Bip32Path;
use crate::{AppSW, DataContext};
use alloc::vec::Vec;
use ledger_device_sdk::ecc::CxError;
use ledger_device_sdk::ecc::ECPrivateKey;
use ledger_device_sdk::ecc::{Secp256k1, SeedDerive};
use ledger_device_sdk::hash::{blake2::Blake2b_512, sha3::Keccak256, HashInit};
use ledger_device_sdk::io::Comm;

use ledger_secure_sdk_sys::*;

use parity_scale_codec::{Compact, DecodeAll, Encode};
use serde::Deserialize;
use serde_json_core::from_slice;

const MAX_TRANSACTION_LEN: usize = 510;

#[derive(Deserialize)]
pub struct Tx {}

pub struct Input {
    pub path: Option<Bip32Path>,
    // add amount
}

pub struct TxContext {
    raw_tx: Vec<u8>,
    version: u8,
    num_inputs: u32,
    num_outputs: u32,
    path: Bip32Path,
    review_finished: bool,

    tx_hasher: Blake2b_512,

    inputs: Vec<Input>,
    utxos: Vec<Option<ml_common::TxOutput>>,
    outputs: Vec<ml_common::TxOutput>,
}

// Implement constructor for TxInfo with default values
impl TxContext {
    // Constructor
    pub fn new(version: u8, num_inputs: u32, num_outputs: u32) -> Result<TxContext, AppSW> {
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
            raw_tx: Vec::new(),
            version,
            num_inputs,
            num_outputs,
            review_finished: false,
            tx_hasher,

            path: Default::default(),
            inputs: Default::default(),
            utxos: Default::default(),
            outputs: Default::default(),
        })
    }

    // True if all of the inputs, utxos and outputs have been transfered.
    fn completed(&self) -> bool {
        self.num_inputs as usize == self.inputs.len()
            && self.num_inputs as usize == self.utxos.len()
            && self.num_outputs as usize == self.outputs.len()
    }

    // Get review status
    #[allow(dead_code)]
    pub fn finished(&self) -> bool {
        self.review_finished
    }
}

pub fn handler_sign_tx(
    comm: &mut Comm,
    chunk: u8,
    data_type: u8,
    ctx: &mut DataContext,
) -> Result<(), AppSW> {
    // Try to get data from comm
    let data = comm.get_data().map_err(|_| AppSW::WrongApduLength)?;
    // First chunk, try to parse the path
    if chunk == 0 {
        // Reset transaction context
        if data.len() != 9 {
            return Err(AppSW::Foo4);
        }

        let version = u8::from_be_bytes(data[0..1].try_into().unwrap());
        let num_inputs = u32::from_be_bytes(data[1..5].try_into().unwrap());
        let num_outputs = u32::from_be_bytes(data[5..9].try_into().unwrap());

        let tx_ctx = TxContext::new(version, num_inputs, num_outputs)?;
        *ctx = DataContext::TxContext(tx_ctx);
        Ok(())
    // Next chunks, append data to raw_tx and return or parse
    // the transaction if it is the last chunk.
    } else {
        let ctx = match ctx {
            DataContext::TxContext(ctx) => ctx,
            _ => return Err(AppSW::Foo1),
        };

        if ctx.raw_tx.len() + data.len() > MAX_TRANSACTION_LEN {
            return Err(AppSW::TxWrongLength);
        }

        if data_type == 0 {
            // get path
            let path = if data[0] == 1 {
                Some(data[1..].try_into()?)
            } else {
                None
            };
            let inp = Input { path };
            if inp.path.is_some() {
                comm.append(&[1]);
            } else {
                comm.append(&[0]);
            }
            comm.append(&[1]);

            ctx.inputs.push(inp);
            return Ok(());
        }

        // Append data to raw_tx
        ctx.raw_tx.extend(data);

        // Path 32b + Input 100-700b + extra data 60b
        // data_type 0 = Path 32b
        // data_type 1 = Input bytes but have more
        // data_type 2 = Input bytes last
        // data_type 3 = extra_bytes more
        // data_type 4 = extra_bytes last

        // If we expect more chunks, return
        if data_type == 1 {
            ctx.review_finished = false;
            comm.append(&[0]);
            return Ok(());
        }

        match chunk {
            1 => {
                let _inp = ml_common::TxInput::decode_all(&mut ctx.raw_tx.as_slice())
                    .map_err(|_| AppSW::Foo1)?;
                //ctx.inputs.push(inp);

                // on the first one encode the size of the inputs
                if ctx.inputs.len() == 1 {
                    ctx.tx_hasher
                        .update(&ctx.num_inputs.to_le_bytes())
                        .map_err(|_| AppSW::TxHashFail)?;
                }
                comm.append(&[2]);
            }
            2 => {
                let utxo = Option::<ml_common::TxOutput>::decode_all(&mut ctx.raw_tx.as_slice())
                    .map_err(|_| AppSW::Foo1)?;
                ctx.utxos.push(utxo);

                // on the first one encode the size of the inputs
                if ctx.utxos.len() == 1 {
                    ctx.tx_hasher
                        .update(&ctx.num_inputs.to_le_bytes())
                        .map_err(|_| AppSW::TxHashFail)?;
                }
                comm.append(&[3]);
            }
            _ => {
                let out = ml_common::TxOutput::decode_all(&mut ctx.raw_tx.as_slice())
                    .map_err(|_| AppSW::Foo1)?;
                ctx.outputs.push(out);

                // on the first one encode the size of the inputs
                if ctx.utxos.len() == 1 {
                    ctx.tx_hasher
                        .update(&Compact::<u32>::encode(&ctx.num_outputs.into()))
                        .map_err(|_| AppSW::TxHashFail)?;
                }
                comm.append(&[4]);
            }
        };

        ctx.tx_hasher
            .update(ctx.raw_tx.as_slice())
            .map_err(|_| AppSW::TxHashFail)?;

        ctx.raw_tx.clear();

        if ctx.completed() {
            comm.append(&[99]);
            ctx.review_finished = true;

            // Display transaction. If user approves
            // the transaction, sign it. Otherwise,
            // return a "deny" status word.
            let res = ui_display_tx(&ctx)?;
            //let res = ui_display_message(&[0, 0])?;

            if true {
                comm.append(&[100]);
                ctx.review_finished = true;
                compute_signature_and_append(comm, ctx)
            } else {
                ctx.review_finished = true;
                Err(AppSW::Deny)
            }
        } else {
            ctx.review_finished = false;
            Ok(())
        }
    }
}

fn compute_signature_and_append(comm: &mut Comm, ctx: &mut TxContext) -> Result<(), AppSW> {
    comm.append(&[101]);

    let mut message_hash: [u8; 64] = [0u8; 64];
    ctx.tx_hasher
        .finalize(&mut message_hash)
        .map_err(|_| AppSW::TxHashFail)?;

    let mut blake2b256 = Blake2b_512::new();
    let mut message_hash2: [u8; 64] = [0u8; 64];
    blake2b256
        .hash(&message_hash[0..32], &mut message_hash2)
        .map_err(|_| AppSW::Foo1)?;

    let hash_algorithm_id = CX_SHA256;
    let signing_mode = CX_ECSCHNORR_BIP0340 | CX_RND_TRNG | CX_LAST;

    if let Some(path) = ctx.inputs[0].path.as_ref() {
        let private_key = Secp256k1::derive_from_path(path.as_ref());
        let (sig, siglen) = schnorr_sign(
            &private_key,
            &message_hash2[0..32],
            hash_algorithm_id,
            signing_mode,
        )?;
        comm.append(&[message_hash2[0], message_hash2[1]]);

        comm.append(&[siglen as u8]);
        comm.append(&sig[..siglen as usize]);
        Ok(())
    } else {
        comm.append(&[106]);
        Ok(())
    }
}
