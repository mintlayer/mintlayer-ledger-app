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

use crate::app_ui::address::ui_display_pk;
use crate::utils::{Bip32Path, CoinType};
use crate::AppSW;

use parity_scale_codec::DecodeAll;

use ledger_device_sdk::ecc::{Secp256k1, SeedDerive};
use ledger_device_sdk::io::Comm;

pub fn handler_get_public_key(comm: &mut Comm, display: bool) -> Result<(), AppSW> {
    let data = comm.get_data().map_err(|_| AppSW::WrongApduLength)?;
    let chain_type = CoinType::try_from(*data.get(0).ok_or(AppSW::WrongApduLength)?)?;
    let path = Bip32Path::decode_all(&mut &data[1..]).map_err(|_| AppSW::DeserializeFail)?;

    if path.as_ref().len() < 3 {
        return Err(AppSW::InvalidPath);
    }
    if path.as_ref()[1] != chain_type.coin_path() {
        return Err(AppSW::InvalidPath);
    }

    let (k, cc) = Secp256k1::derive_from(path.as_ref());
    let pk = k.public_key().map_err(|_| AppSW::KeyDeriveFail)?;
    let code = cc.ok_or(AppSW::KeyDeriveFail)?;

    // Display address on device if requested
    if display {
        if !ui_display_pk(&pk.pubkey, chain_type)? {
            return Err(AppSW::Deny);
        }
    }

    comm.append(&[pk.pubkey.len() as u8]);
    comm.append(&pk.pubkey);

    comm.append(&[code.value.len() as u8]);
    comm.append(&code.value);

    Ok(())
}
