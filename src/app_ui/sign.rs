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
use crate::app_ui::utils::{bech32m_encode, to_address};
use crate::handlers::sign_tx::{CoinOrTokenId, TxContext};
use crate::utils::CoinType;
use crate::AppSW;

use chrono::{TimeZone, Utc};
use include_gif::include_gif;
use ledger_device_sdk::nbgl::{Field, NbglGlyph, NbglReview, NbglStreamingReview, TransactionType};
use ml_common::{
    Amount, Destination, IsTokenFreezable, NftIssuance, OutputTimeLock, OutputValue, TokenIssuance,
    TokenTotalSupply, TxOutput, VRFPublicKeyHolder, H256,
};
use parity_scale_codec::Encode;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::{format, string::ToString};
use core::fmt::Write;

/// Displays a transaction and returns true if user approved it.
///
/// This method can return [`AppSW::TxDisplayFail`] error if the coin name length is too long.
///
/// # Arguments
///
/// * `tx` - Transaction to be displayed for validation
pub fn ui_display_tx(tx: &TxContext) -> Result<bool, AppSW> {
    let fees = tx.total_inputs.iter().try_fold(
        String::new(),
        |mut acc, (coin_or_token, amount)| -> Result<_, AppSW> {
            let out = *tx.total_outputs.get(coin_or_token).unwrap_or(&Amount::ZERO);
            let fee: u128 = amount
                .into_atoms()
                .checked_sub(out.into_atoms())
                .ok_or(AppSW::TxNumericOperationFail)?;

            match coin_or_token {
                CoinOrTokenId::Coin => writeln!(
                    acc,
                    "{} {}",
                    format_amount(Amount::from_atoms(fee), tx.coin),
                    tx.coin.coin_ticker()
                )
                .map_err(|_| AppSW::TxDisplayFail)?,
                CoinOrTokenId::TokenId(token_id) => {
                    if fee != 0 {
                        writeln!(
                            acc,
                            "{fee} {}",
                            id_to_address(token_id, tx.coin.token_id_address_prefix())?
                        )
                        .map_err(|_| AppSW::TxDisplayFail)?
                    }
                }
            };

            Ok(acc)
        },
    )?;

    let formated_outputs: Vec<(&str, String)> = tx
        .outputs
        .iter()
        .map(|out| format_output(out, tx.coin))
        .collect::<Result<Vec<_>, _>>()?;

    // Define transaction review fields
    let my_fields: Vec<_> = formated_outputs
        .iter()
        .map(|(name, value)| Field { name, value })
        .chain([Field {
            name: "Fees:",
            value: &fees,
        }])
        .collect();

    // Create transaction review

    // Load glyph from 64x64 4bpp gif file with include_gif macro. Creates an NBGL compatible glyph.
    #[cfg(any(target_os = "stax", target_os = "flex"))]
    const FERRIS: NbglGlyph = NbglGlyph::from_include(include_gif!("crab_64x64.gif", NBGL));
    #[cfg(any(target_os = "nanosplus", target_os = "nanox"))]
    const FERRIS: NbglGlyph = NbglGlyph::from_include(include_gif!("crab_16x16.gif", NBGL));


    /*
    let mut review: NbglStreamingReview = NbglStreamingReview::new()
        .glyph(&FERRIS)
        .tx_type(TransactionType::Transaction);

    if !review.start("Review transaction\nto send ML", "Foo") {
        panic!("hello");
        return Ok(false);
    }
    let f = Field {
            name: "Fees:",
            value: &fees,
        };
    if !review.continue_review(&[f]) {
            return Ok(false);
        }

    Ok(review.finish("Sign transaction\nto send ML"))

    
     */
    

    // Create NBGL review. Maximum number of fields and string buffer length can be customised
    // with constant generic parameters of NbglReview. Default values are 32 and 1024 respectively.
    let review: NbglReview = NbglReview::new()
        .titles(
            "Review transaction\nto send ML",
            "",
            "Sign transaction\nto send ML",
        )
        .glyph(&FERRIS);

    // If first setting switch is disabled do not display the transaction memo
    //let settings: Settings = Default::default();
    //if settings.get_element(0) == 0 {
    //    Ok(review.show(&my_fields[0..2]))
    //} else {
    Ok(review.show(&my_fields))
    //}
}

/// Displays a message for review and signing confirmation on the device.
///
/// This function handles both printable text and raw binary data by
/// displaying UTF-8 content directly and falling back to hex encoding for other data.
///
/// # Arguments
///
/// * `message` - A byte slice (`&[u8]`) containing the message to be signed.
///
/// # Returns
///
/// * `Ok(true)` if the user approves the signing.
/// * `Ok(false)` if the user rejects.
/// * `Err(AppSW)` on error.
pub fn ui_display_message(message: &[u8]) -> Result<bool, AppSW> {
    let message_str = match core::str::from_utf8(message) {
        Ok(s) if s.is_ascii() => s.to_string(),
        Ok(_) | Err(_) => format!("0x{}", hex::encode(message)),
    };

    let my_fields = [Field {
        name: "Message",
        value: message_str.as_str(),
    }];

    // Load a generic icon for signing. You should replace this with your app's icon.
    // The `include_gif!` macro selects the correct size based on the target device.
    #[cfg(any(target_os = "stax", target_os = "flex"))]
    const SIGN_ICON: NbglGlyph = NbglGlyph::from_include(include_gif!("crab_64x64.gif", NBGL));
    #[cfg(any(target_os = "nanosplus", target_os = "nanox"))]
    const SIGN_ICON: NbglGlyph = NbglGlyph::from_include(include_gif!("crab_16x16.gif", NBGL));

    // Create the NBGL review flow with titles appropriate for message signing.
    let review: NbglReview = NbglReview::new()
        .titles(
            "Review message",   // Initial title
            "Cannot be undone", // Warning on the second screen
            "Sign message",     // Final confirmation prompt
        )
        .tx_type(TransactionType::Message)
        .glyph(&SIGN_ICON);

    // Show the review screen with the defined fields and return the user's choice.
    Ok(review.show(&my_fields))
}

fn vrf_to_address(key: &VRFPublicKeyHolder, coin: CoinType) -> Result<String, AppSW> {
    bech32m_encode(coin.vrf_public_key_address_prefix(), &key.encode())
}

fn id_to_address(id: &H256, hrp: &str) -> Result<String, AppSW> {
    bech32m_encode(hrp, &id.0)
}

fn format_amount(amount: Amount, coin: CoinType) -> String {
    let decimals = coin.coin_decimals() as usize;
    let mantissa = amount.into_atoms();
    // ceil(log10(u128::MAX)) + 1 for decimal point = 40
    // This is not the maximum possible length, but a reasonable expectation of it.
    let mut buffer = String::with_capacity(40);
    write!(&mut buffer, "{mantissa:0>width$}", width = decimals + 1).unwrap();
    buffer.insert(buffer.len() - decimals, '.');
    buffer
}

fn format_atoms(amount: Amount) -> String {
    amount.into_atoms().to_string()
}

fn format_value(value: &OutputValue, coin: CoinType) -> Result<String, AppSW> {
    match value {
        OutputValue::Coin(amount) => Ok(format!("Coins: {}", format_amount(*amount, coin))),
        OutputValue::TokenV0 => Err(AppSW::TxInvalidTokenV0),
        OutputValue::TokenV1(token_id, amount) => Ok(format!(
            "Token: {} {}",
            id_to_address(token_id, coin.token_id_address_prefix())?,
            format_atoms(*amount)
        )),
    }
}

fn format_timestamp(seconds_u64: u64) -> Result<String, AppSW> {
    let seconds_i64: i64 = seconds_u64.try_into().map_err(|_| AppSW::TxDisplayFail)?;
    let datetime = Utc
        .timestamp_opt(seconds_i64, 0)
        .earliest()
        .ok_or(AppSW::TxDisplayFail)?;

    Ok(datetime.format("%Y-%m-%d %H:%M:%S").to_string())
}

fn format_lock(lock: &OutputTimeLock) -> Result<String, AppSW> {
    let s = match lock {
        OutputTimeLock::UntilHeight(h) => format!("Lock until block height {h}"),
        OutputTimeLock::UntilTime(t) => format!("Lock until {}", format_timestamp(*t)?),
        OutputTimeLock::ForBlockCount(b) => format!("Lock for {b} blocks"),
        OutputTimeLock::ForSeconds(s) => format!("Lock for {s} seconds"),
    };
    Ok(s)
}

/// Formats a transaction output into a tuple of (short_address, amount, address_label).
///
/// # Arguments
/// * `output` - A reference to the `TxOutput` enum variant to format.
/// * `coin` - A reference to the coin information, used for formatting amounts.
///
/// # Returns
/// A tuple containing three `String`s: `(short_address, amount, address_label)`.
pub fn format_output(output: &TxOutput, coin: CoinType) -> Result<(&str, String), AppSW> {
    let res = match output {
        TxOutput::Transfer(value, destination) => (
            "Transfer",
            format!(
                "Destination: {}\n{}\n",
                to_address(destination, coin)?,
                format_value(value, coin)?
            ),
        ),

        TxOutput::LockThenTransfer(value, destination, lock) => {
            let address_short = format!(
                "Destination: {}\n{}\n{}\n",
                to_address(destination, coin)?,
                format_lock(lock)?,
                format_value(value, coin)?
            );
            ("Lock then Transfer", address_short)
        }

        TxOutput::Burn(value) => ("BURN", format_value(value, coin)?),

        TxOutput::CreateStakePool(pool_id, data) => {
            let address_short = format!(
                "Pool ID: {}\nStaker key: {}\nDecommission key: {}\nVRF public key: {}\nMargin ratio per thousand: {}\nCost per block: {}\nPledge{}\n",
                id_to_address(pool_id, coin.pool_id_address_prefix())?, to_address(&data.staker, coin)?, to_address(&data.decommission_key, coin)?, vrf_to_address(&data.vrf_public_key, coin)?, data.margin_ratio_per_thousand, format_amount(data.cost_per_block, coin),
                format_amount(data.pledge, coin));
            ("Create staking pool", address_short)
        }

        TxOutput::ProduceBlockFromStake(destination, _pool_id) => (
            "Produce block from stake",
            format!("New staker key: {}", to_address(destination, coin)?),
        ),

        TxOutput::CreateDelegationId(destination, pool_id) => {
            let address_short = format!(
                "Address: {}\nPoolId: {}",
                to_address(destination, coin)?,
                id_to_address(pool_id, coin.pool_id_address_prefix())?
            );
            ("Create delegation", address_short)
        }

        TxOutput::DelegateStaking(amount, delegation_id) => (
            "Delegate staking",
            format!(
                "{}\n{}",
                id_to_address(delegation_id, coin.delegation_id_address_prefix())?,
                format_value(&OutputValue::Coin(*amount), coin)?
            ),
        ),

        TxOutput::IssueFungibleToken(x) => {
            let TokenIssuance::V1(data) = x;

            let ticker = String::from_utf8_lossy(data.token_ticker.as_ref());
            let metadata_uri = String::from_utf8_lossy(data.metadata_uri.as_ref());

            let total_supply_str = match data.total_supply {
                TokenTotalSupply::Unlimited => "UNLIMITED".to_string(),
                TokenTotalSupply::Lockable => "LOCKABLE".to_string(),
                TokenTotalSupply::Fixed(amount) => {
                    let formatted_amount = format_amount(amount, coin);
                    format!("FIXED {}", formatted_amount)
                }
            };
            let is_freezable = match data.is_freezable {
                IsTokenFreezable::Yes => "Yes",
                IsTokenFreezable::No => "No",
            };

            let address_short = format!(
                "Ticker: {}\nAuthority: {}\nMetadata URI: {}\nTotal token supply: {}\nNumber of decimals: {}\nIs freezable: {}",
                ticker, to_address(&data.authority, coin)?, metadata_uri, total_supply_str, data.number_of_decimals, is_freezable
            );
            ("Issue fungible token", address_short)
        }

        TxOutput::IssueNft(_nft_id, data, destination) => {
            let data = match data {
                NftIssuance::V0(data) => &data.metadata,
            };
            let address_short = format!(
                "Name: {}\nCreator: {}\nTicker: {}\nAddress: {}\nIcon URI: {}\nAdditional medatada URI: {}\nMedia URI: {}",
                String::from_utf8_lossy(data.name.as_ref()),
                data.creator.clone().map(|creator| to_address(&Destination::PublicKey(creator), coin)).transpose()?.unwrap_or_default(),
                String::from_utf8_lossy(data.ticker.as_ref()),
                to_address(destination, coin)?,
                String::from_utf8_lossy(data.icon_uri.as_ref()),
                String::from_utf8_lossy(data.additional_metadata_uri.as_ref()),
                String::from_utf8_lossy(data.media_uri.as_ref())
            );
            ("Issue NFT token", address_short)
        }

        TxOutput::DataDeposit(data) => ("Data deposit", hex::encode(data)),

        TxOutput::Htlc(value, data) => {
            let address_short = format!(
                "Secret hash: {}\nSpend key: {}\nRefund key: {}\nRefund time lock: {}\n{}",
                hex::encode(data.secret_hash.0),
                to_address(&data.spend_key, coin)?,
                to_address(&data.refund_key, coin)?,
                format_lock(&data.refund_timelock)?,
                format_value(value, coin)?
            );
            ("HTLC", address_short)
        }

        TxOutput::CreateOrder(data) => {
            let ask_amount = format_value(&data.ask, coin)?;
            let give_amount = format_value(&data.give, coin)?;
            let address_short = format!(
                "Conclude key: {}\nAsk: {}\nGive: {}",
                to_address(&data.conclude_key, coin)?,
                ask_amount,
                give_amount
            );
            ("Create order", address_short)
        }
    };

    Ok(res)
}
