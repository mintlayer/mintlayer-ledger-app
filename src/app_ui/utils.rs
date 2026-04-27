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

use alloc::string::String;

use ledger_device_sdk::{include_gif, nbgl::NbglGlyph};

use crate::StatusWord;
use messages::{encode, Destination, PCoinType};

pub fn bech32m_encode(hrp: &str, data: &[u8]) -> Result<String, StatusWord> {
    let parsed_hrp = bech32::Hrp::parse(hrp).map_err(|_| StatusWord::TxAddressFail)?;

    let encoded = bech32::encode::<bech32::Bech32m>(parsed_hrp, data)
        .map_err(|_| StatusWord::TxAddressFail)?;

    Ok(encoded)
}

pub fn to_address(destination: &Destination, coin: PCoinType) -> Result<String, StatusWord> {
    let hrp = coin.address_prefix(destination.into());
    bech32m_encode(hrp, &encode(destination))
}

/// Load glyph from file with include_gif macro. Creates an NBGL compatible glyph.
pub const fn load_glyph() -> NbglGlyph<'static> {
    #[cfg(target_os = "apex_p")]
    const MINTLAYER: NbglGlyph =
        NbglGlyph::from_include(include_gif!("glyphs/mintlayer_48x48.png", NBGL));
    #[cfg(any(target_os = "stax", target_os = "flex"))]
    const MINTLAYER: NbglGlyph =
        NbglGlyph::from_include(include_gif!("glyphs/mintlayer_64x64.gif", NBGL));
    #[cfg(any(target_os = "nanosplus", target_os = "nanox"))]
    const MINTLAYER: NbglGlyph =
        NbglGlyph::from_include(include_gif!("icons/mintlayer_14x14.gif", NBGL));

    MINTLAYER
}
