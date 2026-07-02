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

use ledger_device_sdk::ecc::{Secp256k1, SeedDerive};

use mintlayer_messages::{
    ChainCode, GetPubKeyReq, PublicKeyResponse, UncompressedSecp256k1PublicKey,
};

use crate::{StatusWord, app_ui::address::ui_display_pk, utils::check_derivation_path};

pub fn handle_get_public_key(
    req: GetPubKeyReq,
    display: bool,
) -> Result<PublicKeyResponse, StatusWord> {
    check_derivation_path(req.path.as_ref(), req.coin_type.into())?;

    let (k, cc) = Secp256k1::derive_from(req.path.as_ref());
    let pk = k.public_key().map_err(|_| StatusWord::KeyDeriveFail)?;
    let code = cc.ok_or(StatusWord::KeyDeriveFail)?;

    // Display address on device if requested
    if display && !ui_display_pk(&pk, req.coin_type.into())? {
        return Err(StatusWord::Deny);
    }
    let response = PublicKeyResponse {
        public_key: UncompressedSecp256k1PublicKey(pk.pubkey),
        chain_code: ChainCode(code.value),
    };

    Ok(response)
}
