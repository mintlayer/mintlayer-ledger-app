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

use crate::{app_ui::sign::ui_display_message, errors::cx_err_to_status, DataContext, StatusWord};
use messages::{encode, AddrType, Bip32Path, MsgSignature, PCoinType, SignMessageReq};

use alloc::vec::Vec;
use ledger_device_sdk::{
    ecc::{ECPrivateKey, Secp256k1, SeedDerive},
    hash::{blake2::Blake2b_512, HashInit},
    io::Comm,
};

use ledger_secure_sdk_sys::*;

const MAX_MESSAGE_LEN: usize = 510;

pub struct SignMessageContext {
    message: Vec<u8>,
    path: Bip32Path,
    coin: PCoinType,
    addr_type: AddrType,
    review_finished: bool,
}

impl SignMessageContext {
    pub fn new(req: SignMessageReq) -> Self {
        Self {
            message: Vec::new(),
            path: req.path,
            coin: req.coin.into(),
            addr_type: req.addr_type,
            review_finished: false,
        }
    }

    pub fn finished(&self) -> bool {
        self.review_finished
    }
}

pub fn setup_sign_message(req: SignMessageReq, ctx: &mut DataContext) -> Result<(), StatusWord> {
    *ctx = DataContext::SignMessageContext(SignMessageContext::new(req));
    Ok(())
}

pub fn handle_sign_message(
    comm: &mut Comm,
    more: bool,
    ctx: &mut DataContext,
) -> Result<(), StatusWord> {
    let ctx = match ctx {
        DataContext::SignMessageContext(ctx) => ctx,
        _ => return Err(StatusWord::WrongContext),
    };
    let chunk = comm.get_data().map_err(|_| StatusWord::WrongApduLength)?;

    if ctx.message.len() + chunk.len() > MAX_MESSAGE_LEN {
        return Err(StatusWord::TxWrongLength);
    }

    ctx.message.extend(chunk);

    if more {
        ctx.review_finished = false;
        Ok(())
    } else {
        let private_key = Secp256k1::derive_from_path(ctx.path.as_ref());
        let public_key = private_key
            .public_key()
            .map_err(|_| StatusWord::KeyDeriveFail)?;

        // Display review. If user approves sign it.
        // Otherwise, return a "deny" status word.
        if ui_display_message(&ctx.message, &public_key, ctx.coin, ctx.addr_type)? {
            ctx.review_finished = true;
            compute_signature_and_append(comm, &private_key, ctx)
        } else {
            ctx.review_finished = true;
            Err(StatusWord::Deny)
        }
    }
}

fn compute_signature_and_append<const N: usize>(
    comm: &mut Comm,
    private_key: &ECPrivateKey<N, 'W'>,
    ctx: &mut SignMessageContext,
) -> Result<(), StatusWord> {
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
        .map_err(|_| StatusWord::TxHashFail)?;
    let mut message_hash_32: [u8; 32] = [0u8; 32];
    message_hash_32.copy_from_slice(&message_hash[0..32]);

    let mut blake2b256 = Blake2b_512::new();
    let mut message_hash2: [u8; 64] = [0u8; 64];
    blake2b256
        .hash(&message_hash_32, &mut message_hash2)
        .map_err(|_| StatusWord::TxHashFail)?;

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

    let response = MsgSignature { signature: sig };

    comm.append(&encode(response));
    Ok(())
}

pub fn schnorr_sign<const N: usize>(
    private_key: &ECPrivateKey<N, 'W'>,
    msg: &[u8],
    hash_id: u8,
    mode: u32,
) -> Result<[u8; 64], StatusWord> {
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
        Err(cx_err_to_status(err_code.into()))
    } else {
        Ok(sig)
    }
}
