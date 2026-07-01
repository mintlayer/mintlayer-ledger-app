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

use alloc::vec::Vec;

use ledger_device_sdk::ecc::{ECPrivateKey, Secp256k1, SeedDerive};
use ledger_secure_sdk_sys::*;

use mintlayer_messages::{
    AddrType, Bip32Path, MsgSignatureResponse, SignMessageStartReq, Signature,
};

use crate::{
    DataContext, StatusWord, app_ui::sign::ui_display_message, errors::cx_err_to_status,
    hasher::Hasher, mlcp, utils::check_derivation_path,
};

pub struct SignMessageContext {
    path: Bip32Path,
    coin: mlcp::CoinType,
    addr_type: AddrType,
    review_finished: bool,
}

impl SignMessageContext {
    pub fn new(req: SignMessageStartReq) -> Self {
        Self {
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

pub fn setup_sign_message(req: SignMessageStartReq) -> Result<DataContext, StatusWord> {
    check_derivation_path(req.path.as_ref(), req.coin.into())?;

    Ok(DataContext::SignMessageContext(SignMessageContext::new(
        req,
    )))
}

// TODO: implement stateful message signing, where the message is received and displayed for review
// in portions, to allow signing messages of arbitrary sizes.
// See https://github.com/mintlayer/mintlayer-ledger-app/issues/13.
pub fn handle_sign_message(
    message: &[u8],
    ctx: &mut SignMessageContext,
) -> Result<MsgSignatureResponse, StatusWord> {
    let private_key = Secp256k1::derive_from_path(ctx.path.as_ref());
    let public_key = private_key
        .public_key()
        .map_err(|_| StatusWord::KeyDeriveFail)?;

    // Display review. If user approves, sign it.
    // Otherwise, return a "deny" status word.
    if ui_display_message(message, &public_key, ctx.coin, ctx.addr_type)? {
        ctx.review_finished = true;
        Ok(compute_signature(&private_key, message)?)
    } else {
        ctx.review_finished = true;
        Err(StatusWord::Deny)
    }
}

fn compute_signature<const N: usize>(
    private_key: &ECPrivateKey<N, 'W'>,
    message: &[u8],
) -> Result<MsgSignatureResponse, StatusWord> {
    const MESSAGE_MAGIC_PREFIX: &str = "===MINTLAYER MESSAGE BEGIN===\n";
    const MESSAGE_MAGIC_SUFFIX: &str = "\n===MINTLAYER MESSAGE END===";

    let message = MESSAGE_MAGIC_PREFIX
        .as_bytes()
        .iter()
        .chain(message.iter())
        .chain(MESSAGE_MAGIC_SUFFIX.as_bytes().iter())
        .copied()
        .collect::<Vec<_>>();

    let message_hash = Hasher::hash(&message)?;
    let message_hash2 = Hasher::hash(message_hash.as_bytes())?;

    let sig = schnorr_sign(private_key, message_hash2.as_bytes())?;

    let response = MsgSignatureResponse {
        signature: Signature(sig),
    };

    Ok(response)
}

pub fn schnorr_sign<const N: usize>(
    private_key: &ECPrivateKey<N, 'W'>,
    msg: &[u8],
) -> Result<[u8; 64], StatusWord> {
    const HASH_ALGORITHM_ID: u8 = CX_SHA256;
    const SIGNING_MODE: u32 = CX_ECSCHNORR_BIP0340 | CX_RND_PROVIDED | CX_LAST;

    let mut sig = [0u8; 64];
    let mut sig_len = 64;

    // The `unsafe` block is required for FFI.
    let err_code = unsafe {
        cx_ecschnorr_sign_no_throw(
            private_key as *const ECPrivateKey<N, 'W'> as *const cx_ecfp_256_private_key_s,
            SIGNING_MODE,
            HASH_ALGORITHM_ID,
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
