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

use alloc::string::String;

use ledger_device_sdk::{
    ecc::ECPublicKey,
    hash::{blake2::Blake2b_512, HashInit},
    include_gif,
    nbgl::NbglGlyph,
};

use crate::StatusWord;
use mintlayer_messages::{
    encode,
    mlcp::{CoinType, Destination, PublicKeyHash, Secp256k1PublicKey},
};

pub fn bech32m_encode(hrp: &str, data: &[u8]) -> Result<String, StatusWord> {
    let parsed_hrp = bech32::Hrp::parse(hrp).map_err(|_| StatusWord::TxAddressFail)?;

    let encoded = bech32::encode::<bech32::Bech32m>(parsed_hrp, data)
        .map_err(|_| StatusWord::TxAddressFail)?;

    Ok(encoded)
}

pub fn to_address(destination: &Destination, coin: CoinType) -> Result<String, StatusWord> {
    let hrp = coin.address_prefix(destination.into());
    bech32m_encode(hrp, &encode(destination))
}

/// Load glyph from file with include_gif macro. Creates an NBGL compatible glyph.
pub const fn load_glyph() -> NbglGlyph<'static> {
    #[cfg(target_os = "apex_p")]
    const MINTLAYER: NbglGlyph =
        NbglGlyph::from_include(include_gif!("../../media/glyphs/mintlayer_48x48.png", NBGL));
    #[cfg(any(target_os = "stax", target_os = "flex"))]
    const MINTLAYER: NbglGlyph =
        NbglGlyph::from_include(include_gif!("../../media/glyphs/mintlayer_64x64.gif", NBGL));
    #[cfg(any(target_os = "nanosplus", target_os = "nanox"))]
    const MINTLAYER: NbglGlyph =
        NbglGlyph::from_include(include_gif!("../../media/icons/mintlayer_14x14.gif", NBGL));

    MINTLAYER
}

pub fn compress_public_key<const T: char>(
    public_key: &ECPublicKey<65, T>,
) -> Result<Secp256k1PublicKey, StatusWord> {
    let uncompressed_key = &public_key.pubkey;
    if uncompressed_key[0] != 0x04 {
        return Err(StatusWord::InvalidUncompressedPublicKey);
    }

    let mut compressed_key = [0u8; 33];

    let y_coordinate = &uncompressed_key[33..65];
    let prefix = if y_coordinate[31].is_multiple_of(2) {
        0x02
    } else {
        0x03
    };

    compressed_key[0] = prefix;

    let x_coordinate = &uncompressed_key[1..33];
    compressed_key[1..].copy_from_slice(x_coordinate);

    Ok(Secp256k1PublicKey(compressed_key))
}

pub fn to_public_key_hash(pk: &Secp256k1PublicKey) -> Result<PublicKeyHash, StatusWord> {
    let mut blake2b256 = Blake2b_512::new();
    let mut public_key_hash: [u8; 64] = [0u8; 64];

    blake2b256
        .update(&[0])
        .map_err(|_| StatusWord::TxHashFail)?;
    blake2b256
        .update(&pk.0)
        .map_err(|_| StatusWord::TxHashFail)?;

    blake2b256
        .finalize(&mut public_key_hash)
        .map_err(|_| StatusWord::TxHashFail)?;

    let mut pkh = [0u8; 20];
    pkh.copy_from_slice(&public_key_hash[0..20]);

    Ok(PublicKeyHash(pkh))
}
