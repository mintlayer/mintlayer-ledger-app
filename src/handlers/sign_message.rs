// Copyright (c) 2025 RBB S.r.l
// opensource@mintlayer.org
// SPDX-License-Identifier: MIT
// Licensed under the MIT License;
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://github.com/mintlayer/mintlayer-core/blob/master/LICENSE
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::app_ui::sign::ui_display_message;
use crate::utils::{AddrType, Bip32Path, CoinType};
use crate::{AppSW, DataContext};
use alloc::vec::Vec;
use ledger_device_sdk::ecc::{Secp256k1, SeedDerive};
use ledger_device_sdk::hash::{blake2::Blake2b_512, HashInit};
use ledger_device_sdk::io::Comm;

use ledger_device_sdk::ecc::CxError;
use ledger_device_sdk::ecc::ECPrivateKey;

use ledger_secure_sdk_sys::*;

use parity_scale_codec::DecodeAll;

const MAX_MESSAGE_LEN: usize = 510;

pub struct SignMessageContext {
    message: Vec<u8>,
    path: Bip32Path,
    coin: CoinType,
    addr_type: AddrType,
    review_finished: bool,
}

impl SignMessageContext {
    pub fn new(path: Bip32Path, coin: CoinType, addr_type: AddrType) -> Self {
        Self {
            message: Vec::new(),
            path,
            coin,
            addr_type,
            review_finished: false,
        }
    }

    pub fn finished(&self) -> bool {
        self.review_finished
    }
}

pub fn handler_sign_message(
    comm: &mut Comm,
    chunk: u8,
    more: bool,
    ctx: &mut DataContext,
) -> Result<(), AppSW> {
    let data = comm.get_data().map_err(|_| AppSW::WrongApduLength)?;

    if chunk == 0 {
        let coin: CoinType = (*data.get(0).ok_or(AppSW::WrongApduLength)?).try_into()?;
        let addr_type: AddrType = (*data.get(1).ok_or(AppSW::WrongApduLength)?).try_into()?;
        let path = Bip32Path::decode_all(&mut &data[2..]).map_err(|_| AppSW::DeserializeFail)?;
        if path.as_ref().get(1) != Some(&coin.coin_path()) {
            return Err(AppSW::TxInvalidInputPath);
        }

        let msg_ctx = SignMessageContext::new(path, coin, addr_type);
        *ctx = DataContext::SignMessageContext(msg_ctx);
        Ok(())
    } else {
        let ctx = match ctx {
            DataContext::SignMessageContext(ctx) => ctx,
            _ => return Err(AppSW::WrongContext),
        };

        if ctx.message.len() + data.len() > MAX_MESSAGE_LEN {
            return Err(AppSW::TxWrongLength);
        }

        ctx.message.extend(data);

        if more {
            ctx.review_finished = false;
            Ok(())
        } else {
            let private_key = Secp256k1::derive_from_path(ctx.path.as_ref());
            let public_key = private_key
                .public_key()
                .map_err(|_| AppSW::KeyDeriveFail)?
                .pubkey;

            // Display review. If user approves sign it.
            // Otherwise, return a "deny" status word.
            if ui_display_message(&ctx.message, &public_key, ctx.coin, ctx.addr_type)? {
                ctx.review_finished = true;
                compute_signature_and_append(comm, &private_key, ctx)
            } else {
                ctx.review_finished = true;
                Err(AppSW::Deny)
            }
        }
    }
}

fn compute_signature_and_append<const N: usize>(
    comm: &mut Comm,
    private_key: &ECPrivateKey<N, 'W'>,
    ctx: &mut SignMessageContext,
) -> Result<(), AppSW> {
    const MESSAGE_MAGIC_PREFIX: &str = "===MINTLAYER MESSAGE BEGIN===\n";
    const MESSAGE_MAGIC_SUFFIX: &str = "\n===MINTLAYER MESSAGE END===";

    let mut blake2b256 = Blake2b_512::new();
    let mut message_hash: [u8; 64] = [0u8; 64];

    let message = MESSAGE_MAGIC_PREFIX
        .as_bytes()
        .iter()
        .chain(ctx.message.iter())
        .chain(MESSAGE_MAGIC_SUFFIX.as_bytes().iter())
        .copied()
        .collect::<Vec<_>>();

    blake2b256
        .hash(&message, &mut message_hash)
        .map_err(|_| AppSW::TxHashFail)?;
    let mut message_hash_32: [u8; 32] = [0u8; 32];
    message_hash_32.copy_from_slice(&message_hash[0..32]);

    let mut blake2b256 = Blake2b_512::new();
    let mut message_hash2: [u8; 64] = [0u8; 64];
    blake2b256
        .hash(&message_hash_32, &mut message_hash2)
        .map_err(|_| AppSW::TxHashFail)?;

    let mut message_hash2_32: [u8; 32] = [0u8; 32];
    message_hash2_32.copy_from_slice(&message_hash2[0..32]);

    let hash_algorithm_id = CX_SHA256;
    let signing_mode = CX_ECSCHNORR_BIP0340 | CX_RND_PROVIDED | CX_LAST;

    let sig = schnorr_sign(
        private_key,
        &message_hash2_32,
        hash_algorithm_id,
        signing_mode,
    )?;

    comm.append(&[sig.len() as u8]);
    comm.append(&sig);
    Ok(())
}

pub fn schnorr_sign<const N: usize>(
    private_key: &ECPrivateKey<N, 'W'>,
    msg: &[u8],
    hash_id: u8,
    mode: u32,
) -> Result<[u8; 64], CxError> {
    let mut sig = [0u8; 64];
    let mut sig_len = 64;

    // The `unsafe` block is required for FFI.
    let err_code = unsafe {
        cx_ecschnorr_sign_no_throw(
            private_key as *const ECPrivateKey<N, 'W'> as *const cx_ecfp_256_private_key_s,
            mode,
            hash_id,
            msg.as_ptr(),
            msg.len(),
            sig.as_mut_ptr(),
            &mut sig_len,
        )
    };

    if err_code != CX_OK {
        Err(err_code.into())
    } else {
        Ok(sig)
    }
}
