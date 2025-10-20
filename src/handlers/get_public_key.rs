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
use crate::AppSW;
use messages::PublicKeyReq;

use ledger_device_sdk::ecc::{Secp256k1, SeedDerive};
use ledger_device_sdk::io::Comm;

pub fn handler_get_public_key(
    comm: &mut Comm,
    req: PublicKeyReq,
    display: bool,
) -> Result<(), AppSW> {
    if req.path.as_ref().len() < 3 {
        return Err(AppSW::InvalidPath);
    }
    if req.path.as_ref()[1] != req.coin_type.bip44_coin_type() {
        return Err(AppSW::InvalidPath);
    }

    let (k, cc) = Secp256k1::derive_from(req.path.as_ref());
    let pk = k.public_key().map_err(|_| AppSW::KeyDeriveFail)?;
    let code = cc.ok_or(AppSW::KeyDeriveFail)?;

    // Display address on device if requested
    if display && !ui_display_pk(&pk, req.coin_type)? {
        return Err(AppSW::Deny);
    }

    comm.append(&[pk.pubkey.len() as u8]);
    comm.append(&pk.pubkey);

    comm.append(&[code.value.len() as u8]);
    comm.append(&code.value);

    Ok(())
}
