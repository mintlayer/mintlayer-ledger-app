use alloc::string::String;

use crate::{utils::CoinType, AppSW};

use ml_common::Destination;
use parity_scale_codec::Encode;

pub fn bech32m_encode(hrp: &str, data: &[u8]) -> Result<String, AppSW> {
    let parsed_hrp = bech32::Hrp::parse(hrp).map_err(|_| AppSW::TxAddressFail)?;

    let encoded =
        bech32::encode::<bech32::Bech32m>(parsed_hrp, data).map_err(|_| AppSW::TxAddressFail)?;

    Ok(encoded)
}

pub fn to_address(destination: &Destination, coin: CoinType) -> Result<String, AppSW> {
    let hrp = coin.address_prefix(destination);
    bech32m_encode(hrp, &destination.encode())
}
