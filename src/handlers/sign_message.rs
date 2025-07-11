use crate::app_ui::sign::ui_display_message;
use crate::utils::Bip32Path;
use crate::{AppSW, DataContext};
use alloc::vec::Vec;
use ledger_device_sdk::ecc::{Secp256k1, SeedDerive};
use ledger_device_sdk::hash::{blake2::Blake2b_512, HashInit};
use ledger_device_sdk::io::Comm;

use ledger_device_sdk::ecc::CxError;
use ledger_device_sdk::ecc::ECPrivateKey;

use ledger_secure_sdk_sys::*;

const MAX_TRANSACTION_LEN: usize = 510;
pub struct SignMessageContext {
    message: Vec<u8>,
    path: Bip32Path,
    review_finished: bool,
}

// Implement constructor for TxInfo with default values
impl SignMessageContext {
    // Constructor
    pub fn new(path: Bip32Path) -> Self {
        Self {
            message: Vec::new(),
            path,
            review_finished: false,
        }
    }
    // Get review status
    #[allow(dead_code)]
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
    // Try to get data from comm
    let data = comm.get_data().map_err(|_| AppSW::WrongApduLength)?;
    // First chunk, try to parse the path
    if chunk == 0 {
        // Reset transaction context
        let msg_ctx = SignMessageContext::new(data.try_into()?);
        *ctx = DataContext::SignMessageContext(msg_ctx);
        Ok(())
    // Next chunks, append data to raw_tx and return or parse
    // the transaction if it is the last chunk.
    } else {
        let ctx = match ctx {
            DataContext::SignMessageContext(ctx) => ctx,
            _ => return Err(AppSW::WrongContext),
        };

        if ctx.message.len() + data.len() > MAX_TRANSACTION_LEN {
            return Err(AppSW::TxWrongLength);
        }

        // Append data to raw_tx
        ctx.message.extend(data);

        // If we expect more chunks, return
        if more {
            ctx.review_finished = false;
            Ok(())
        // Otherwise, try to parse the transaction
        } else {
            // Try to deserialize the transaction
            //let (tx, _): (Tx, usize) = from_slice(&ctx.raw_tx).map_err(|_| AppSW::TxParsingFail)?;
            // Display transaction. If user approves
            // the transaction, sign it. Otherwise,
            // return a "deny" status word.
            if ui_display_message(&ctx.message)? {
                ctx.review_finished = true;
                compute_signature_and_append(comm, ctx)
            } else {
                ctx.review_finished = true;
                Err(AppSW::Deny)
            }
        }
    }
}

fn compute_signature_and_append(
    comm: &mut Comm,
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
    //let signing_mode = CX_RND_RFC6979 | CX_LAST;
    let signing_mode = CX_ECSCHNORR_BIP0340 | CX_RND_TRNG | CX_LAST;

    let private_key = Secp256k1::derive_from_path(ctx.path.as_ref());
    let (sig, siglen) = schnorr_sign(
        &private_key,
        &message_hash2_32,
        hash_algorithm_id,
        signing_mode,
    )?;

    comm.append(&[siglen as u8]);
    comm.append(&sig[..siglen as usize]);
    Ok(())
}

pub fn schnorr_sign<const N: usize>(
    private_key: &ECPrivateKey<N, 'W'>,
    msg: &[u8],
    hash_id: u8,
    mode: u32,
) -> Result<([u8; 500], u32), CxError> {
    // A buffer on the stack to hold the resulting signature.
    let mut sig = [0u8; 500];
    // The C function takes a pointer to a `size_t` for the length.
    // It's an in/out parameter: we provide the buffer size, and it returns the actual signature size.
    let mut sig_len = 500;

    // The `unsafe` block is required for FFI.
    let err_code = unsafe {
        cx_ecschnorr_sign_no_throw(
            // The same "dodgy" but necessary cast as in the ECDSA function.
            private_key as *const ECPrivateKey<N, 'W'> as *const cx_ecfp_256_private_key_s,
            mode,
            hash_id,
            msg.as_ptr(),
            msg.len(),
            sig.as_mut_ptr(),
            &mut sig_len,
        )
    };

    // Standard Ledger SDK error handling.
    if err_code != CX_OK {
        Err(err_code.into())
    } else {
        // On success, return the signature buffer and the actual length.
        // Note that the `info` parameter is not present in the Schnorr C function,
        // so we don't return it here either.
        Ok((sig, sig_len as u32))
    }
}
