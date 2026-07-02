/*****************************************************************************
 *   Mintlayer Ledger App.
 *   (c) 2023 Ledger SAS.
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

use ledger_device_sdk::{
    ecc::ECPublicKey,
    nbgl::{NbglAddressReview, NbglGlyph},
};

use mintlayer_messages::{Destination, PublicKey};

use crate::{
    StatusWord,
    app_ui::utils::{compress_public_key, load_glyph, to_address},
    mlcp,
};

pub fn ui_display_pk<const T: char>(
    public_key: &ECPublicKey<65, T>,
    coin_type: mlcp::CoinType,
) -> Result<bool, StatusWord> {
    let pk = compress_public_key(public_key)?;

    let dest = Destination::PublicKey(PublicKey::Secp256k1Schnorr(pk));
    let addr = to_address(&dest, coin_type)?;

    const MINTLAYER: NbglGlyph = load_glyph();

    // Display the address confirmation screen.
    Ok(NbglAddressReview::new()
        .glyph(&MINTLAYER)
        .review_title("Verify ML address")
        .show(&addr))
}
