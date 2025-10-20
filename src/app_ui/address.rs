/*****************************************************************************
 *   Mintlayer Ledger App.
 *   (c) 2023 Ledger SAS.
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

use crate::{app_ui::utils::to_address, AppSW};
use messages::CoinType;

use include_gif::include_gif;
use ledger_device_sdk::{
    ecc::ECPublicKey,
    nbgl::{NbglAddressReview, NbglGlyph},
};

pub fn compress_public_key<const T: char>(
    public_key: &ECPublicKey<65, T>,
) -> Result<[u8; 33], AppSW> {
    let uncompressed_key = &public_key.pubkey;
    if uncompressed_key[0] != 0x04 {
        return Err(AppSW::InvalidUncompressedPublicKey);
    }

    let mut compressed_key = [0u8; 33];

    let y_coordinate = &uncompressed_key[33..65];
    let prefix = if y_coordinate[31] % 2 == 0 {
        0x02
    } else {
        0x03
    };

    compressed_key[0] = prefix;

    let x_coordinate = &uncompressed_key[1..33];
    compressed_key[1..].copy_from_slice(x_coordinate);

    Ok(compressed_key)
}

pub fn ui_display_pk<const T: char>(
    public_key: &ECPublicKey<65, T>,
    coin_type: CoinType,
) -> Result<bool, AppSW> {
    let pk = compress_public_key(public_key)?;

    let dest = ml_common::Destination::PublicKey(ml_common::PublicKeyHolder::Secp256k1Schnorr(
        ml_common::PublicKey(pk),
    ));
    let addr = to_address(&dest, coin_type)?;

    // Load glyph from file with include_gif macro. Creates an NBGL compatible glyph.
    #[cfg(target_os = "apex_p")]
    const FERRIS: NbglGlyph =
        NbglGlyph::from_include(include_gif!("glyphs/mintlayer_48x48.png", NBGL));
    #[cfg(any(target_os = "stax", target_os = "flex"))]
    const FERRIS: NbglGlyph =
        NbglGlyph::from_include(include_gif!("glyphs/mintlayer_64x64.gif", NBGL));
    #[cfg(any(target_os = "nanosplus", target_os = "nanox"))]
    const FERRIS: NbglGlyph =
        NbglGlyph::from_include(include_gif!("icons/mintlayer_14x14.gif", NBGL));

    // Display the address confirmation screen.
    Ok(NbglAddressReview::new()
        .glyph(&FERRIS)
        .review_title("Verify ML address")
        .show(&addr))
}
