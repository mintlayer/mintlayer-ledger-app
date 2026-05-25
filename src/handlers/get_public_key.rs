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

use crate::app_ui::address::ui_display_pk;
use crate::StatusWord;
use messages::{mlcp::CoinType, ChainCode, GetPublicKeyResponse, PublicKey, PublicKeyReq};

use ledger_device_sdk::ecc::{Secp256k1, SeedDerive};

// Path should be at least [bip44, coin_type, account_index]
const MIN_PATH_LEN: usize = 3;
const COIN_TYPE_INDEX: usize = 1;

pub fn handle_get_public_key(
    req: PublicKeyReq,
    display: bool,
) -> Result<GetPublicKeyResponse, StatusWord> {
    if req.path.as_ref().len() < MIN_PATH_LEN {
        return Err(StatusWord::InvalidPath);
    }
    let coin_type: CoinType = req.coin_type.into();
    if req.path.as_ref()[COIN_TYPE_INDEX] != coin_type.bip44_coin_type() {
        return Err(StatusWord::InvalidPath);
    }

    let (k, cc) = Secp256k1::derive_from(req.path.as_ref());
    let pk = k.public_key().map_err(|_| StatusWord::KeyDeriveFail)?;
    let code = cc.ok_or(StatusWord::KeyDeriveFail)?;

    // Display address on device if requested
    if display && !ui_display_pk(&pk, req.coin_type.into())? {
        return Err(StatusWord::Deny);
    }
    let response = GetPublicKeyResponse {
        public_key: PublicKey(pk.pubkey),
        chain_code: ChainCode(code.value),
    };

    Ok(response)
}
