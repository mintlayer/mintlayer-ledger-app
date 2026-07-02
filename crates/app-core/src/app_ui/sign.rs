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

use alloc::{format, string::String, string::ToString};
use chrono::{TimeZone, Utc};
use core::fmt::Write;

use ledger_device_sdk::{
    ecc::ECPublicKey,
    nbgl::{
        Field, NbglGlyph, NbglReview, NbglStreamingReview, NbglStreamingReviewStatus,
        TransactionType,
    },
};

use mintlayer_messages::{
    AccountCommand, AccountSpending, AddrType, Amount, DelegationId, Destination, H256,
    IsTokenFreezable, IsTokenUnfreezable, NftIssuance, OrderAccountCommand, OrderId,
    OutputTimeLock, OutputValue, PoolId, PublicKey, TokenId, TokenIssuance, TokenTotalSupply,
    TxOutput, VrfPublicKey, encode,
};

use crate::{
    StatusWord,
    app_ui::utils::{
        bech32m_encode, compress_public_key, load_glyph, to_address, to_public_key_hash,
    },
    handlers::sign_tx::{CoinOrTokenId, InputCommand, TxSummaryCollector, TxType},
    mlcp,
    utils::{make_displayable, make_displayable_str, output_value_with_amount},
};

struct FormattedOutput {
    name: &'static str,
    value: String,
}

pub fn ui_new_streaming_review() -> NbglStreamingReview {
    const MINTLAYER: NbglGlyph = load_glyph();

    NbglStreamingReview::new()
        .glyph(&MINTLAYER)
        .tx_type(TransactionType::Transaction)
}

pub fn ui_start_streaming_review(review: &NbglStreamingReview) -> bool {
    review.start("Review transaction", None)
}

pub fn ui_streaming_review_show_input(
    review: &NbglStreamingReview,
    input: &InputCommand,
    coin: mlcp::CoinType,
) -> Result<bool, StatusWord> {
    let input = format_input(input, coin)?;

    let fields = [Field {
        name: input.name,
        value: &input.value,
    }];

    let res = match review.next(&fields) {
        NbglStreamingReviewStatus::Rejected => false,
        NbglStreamingReviewStatus::Next | NbglStreamingReviewStatus::Skipped => true,
    };

    Ok(res)
}

pub fn ui_streaming_review_show_output(
    review: &NbglStreamingReview,
    output: &TxOutput,
    coin: mlcp::CoinType,
) -> Result<bool, StatusWord> {
    let output = format_output(output, coin)?;

    let fields = [Field {
        name: output.name,
        value: &output.value,
    }];

    let res = match review.next(&fields) {
        NbglStreamingReviewStatus::Rejected => false,
        NbglStreamingReviewStatus::Next | NbglStreamingReviewStatus::Skipped => true,
    };

    Ok(res)
}

pub fn ui_approve_streaming_review(
    review: &NbglStreamingReview,
    tx_summary: &TxSummaryCollector,
    coin: mlcp::CoinType,
) -> Result<bool, StatusWord> {
    let fees = tx_summary.fees_iter().try_fold(
        String::new(),
        |mut acc, res| -> Result<_, StatusWord> {
            let (coin_or_token, fee) = res?;

            match coin_or_token {
                CoinOrTokenId::Coin => writeln!(
                    acc,
                    "{}",
                    format_coin_amount_with_name(Amount::from_atoms(fee), coin),
                )
                .map_err(|_| StatusWord::TxDisplayFail)?,

                CoinOrTokenId::TokenId(token_id) => {
                    if fee != 0 {
                        writeln!(
                            acc,
                            "{}",
                            format_token_amount_with_name(token_id, Amount::from_atoms(fee), coin)?,
                        )
                        .map_err(|_| StatusWord::TxDisplayFail)?;
                    }
                }
            };

            Ok(acc)
        },
    )?;

    let fields = [Field {
        name: "Fees:",
        value: &fees,
    }];

    match review.next(&fields) {
        NbglStreamingReviewStatus::Rejected => return Ok(false),
        NbglStreamingReviewStatus::Next | NbglStreamingReviewStatus::Skipped => {}
    };

    let title = transaction_title(&tx_summary.tx_type());
    Ok(review.finish(title))
}

fn transaction_title(tx_type: &Option<TxType>) -> &'static str {
    match tx_type {
        None | Some(TxType::ComplexTransaction) => "Sign transaction",
        Some(TxType::Transfer) => "Sign transfer transaction",
        Some(TxType::Burn) => "Sign burn transaction",
        Some(TxType::Htlc) => "Sign create HTLC transaction",
        Some(TxType::CreateDelegation) => "Sign create delegation transaction",
        Some(TxType::DelegateStaking) => "Sign delegate staking transaction",
        Some(TxType::DelegationWithdrawal) => "Sign delegation withdrawal transaction",
        Some(TxType::CreateStakePool) => "Sign create stake pool transaction",
        Some(TxType::DecommissionStakePool) => "Sign decommission stake pool transaction",
        Some(TxType::CreateNft) => "Sign create NFT transaction",
        Some(TxType::CreateToken) => "Sign create token transaction",
        Some(TxType::MintTokens) => "Sign mint tokens transaction",
        Some(TxType::UnmintTokens) => "Sign unmint tokens transaction",
        Some(TxType::FreezeToken) => "Sign freeze tokens transaction",
        Some(TxType::UnfreezeToken) => "Sign unfreeze tokens transaction",
        Some(TxType::LockTokenSupply) => "Sign lock token supply transaction",
        Some(TxType::ChangeTokenAuthority) => "Sign change token authority transaction",
        Some(TxType::ChangeTokenMetadataUri) => "Sign change token metadata URI transaction",
        Some(TxType::CreateOrder) => "Sign create order transaction",
        Some(TxType::FillOrder) => "Sign fill order transaction",
        Some(TxType::FreezeOrder) => "Sign freeze order transaction",
        Some(TxType::ConcludeOrder) => "Sign conclude order transaction",
        Some(TxType::DataDeposit) => "Sign data deposit transaction",
    }
}

/// Displays a message for review and signing confirmation on the device.
///
/// This function handles both printable text and raw binary data by
/// displaying UTF-8 content directly and falling back to hex encoding for other data.
///
/// # Arguments
///
/// * `message`    - The message to be signed.
/// * `public_key` - The public key corresponding to the private key that will be used for signing.
/// * `coin_type`  - The coin type (mainnet, testnet etc).
/// * `addr_type`  - The address type (pk or pkh); this determines how `public_key` will be displayed.
///
/// # Returns
///
/// * `Ok(true)` if the user approves the signing.
/// * `Ok(false)` if the user rejects.
/// * `Err(StatusWord)` on error.
pub fn ui_display_message<const T: char>(
    message: &[u8],
    public_key: &ECPublicKey<65, T>,
    coin_type: mlcp::CoinType,
    addr_type: AddrType,
) -> Result<bool, StatusWord> {
    let pk = compress_public_key(public_key)?;

    let dest = match addr_type {
        AddrType::PublicKey => Destination::PublicKey(PublicKey::Secp256k1Schnorr(pk)),
        AddrType::PublicKeyHash => Destination::PublicKeyHash(to_public_key_hash(&pk)?),
    };
    let addr = to_address(&dest, coin_type)?;

    let message_str = make_displayable_str(message);

    let msg_fields = [
        Field {
            name: "Address",
            value: addr.as_str(),
        },
        Field {
            name: "Message",
            value: &message_str,
        },
    ];

    const MINTLAYER: NbglGlyph = load_glyph();

    // Create the NBGL review flow with titles appropriate for message signing.
    let review: NbglReview = NbglReview::new()
        .titles(
            // Title
            "Review message",
            // Subtitle; if non-empty, this will be shown on the second screen on nano devices
            // and below the title on the first screen on touch devices.
            "",
            // Final confirmation prompt
            "Sign message",
        )
        .tx_type(TransactionType::Message)
        .glyph(&MINTLAYER);

    // Show the review screen with the defined fields and return the user's choice.
    Ok(review.show(&msg_fields))
}

fn vrf_to_address(key: &VrfPublicKey, coin: mlcp::CoinType) -> Result<String, StatusWord> {
    bech32m_encode(coin.vrf_public_key_address_prefix(), &encode(key))
}

fn id_to_address(id: &H256, hrp: &str) -> Result<String, StatusWord> {
    bech32m_encode(hrp, &id.0)
}

fn token_id_to_address(id: &TokenId, coin: mlcp::CoinType) -> Result<String, StatusWord> {
    id_to_address(id.hash(), coin.token_id_address_prefix())
}

fn pool_id_to_address(id: &PoolId, coin: mlcp::CoinType) -> Result<String, StatusWord> {
    id_to_address(id.hash(), coin.pool_id_address_prefix())
}

fn delegation_id_to_address(id: &DelegationId, coin: mlcp::CoinType) -> Result<String, StatusWord> {
    id_to_address(id.hash(), coin.delegation_id_address_prefix())
}

fn order_id_to_address(id: &OrderId, coin: mlcp::CoinType) -> Result<String, StatusWord> {
    id_to_address(id.hash(), coin.order_id_address_prefix())
}

/// Format a coin amount as a decimal string.
fn format_coin_amount(amount: Amount, coin: mlcp::CoinType) -> String {
    let decimals = coin.coin_decimals() as usize;
    let mantissa = amount.into_atoms();

    // ceil(log10(u128::MAX)) + 1 for decimal point = 40
    // This is not the maximum possible length, but a reasonable expectation of it.
    let mut buffer = String::with_capacity(40);
    write!(&mut buffer, "{mantissa:0>width$}", width = decimals + 1).unwrap();
    buffer.insert(buffer.len() - decimals, '.');
    buffer
}

/// Same as format_coin_amount, but also append the coin ticker to the amount.
fn format_coin_amount_with_name(amount: Amount, coin: mlcp::CoinType) -> String {
    let mut amount = format_coin_amount(amount, coin);
    amount.push(' ');
    amount.push_str(coin.coin_ticker());
    amount
}

fn format_token_amount_with_name(
    token_id: &TokenId,
    amount: Amount,
    coin: mlcp::CoinType,
) -> Result<String, StatusWord> {
    let atoms = amount.into_atoms();
    let id_str = id_to_address(token_id.hash(), coin.token_id_address_prefix())?;

    Ok(format!("{atoms} atoms of {id_str}"))
}

fn format_atoms(amount: Amount) -> String {
    format!("{} atoms", amount.into_atoms())
}

fn format_value(value: &OutputValue, coin: mlcp::CoinType) -> Result<String, StatusWord> {
    match value {
        OutputValue::Coin(amount) => Ok(format_coin_amount_with_name(*amount, coin)),
        OutputValue::TokenV1(token_id, amount) => {
            format_token_amount_with_name(token_id, *amount, coin)
        }
    }
}

fn format_timestamp(seconds_u64: u64) -> Result<String, StatusWord> {
    let seconds_i64: i64 = seconds_u64
        .try_into()
        .map_err(|_| StatusWord::TxLockTimeInvalid)?;
    let datetime = Utc
        .timestamp_opt(seconds_i64, 0)
        .earliest()
        .ok_or(StatusWord::TxLockTimeInvalid)?;

    Ok(datetime.format("%Y-%m-%d %H:%M:%S").to_string())
}

fn format_lock(lock: &OutputTimeLock) -> Result<String, StatusWord> {
    let s = match lock {
        OutputTimeLock::UntilHeight(h) => format!("Lock until block height {}", h.0),
        OutputTimeLock::UntilTime(t) => format!("Lock until {}", format_timestamp(t.0.0)?),
        OutputTimeLock::ForBlockCount(b) => format!("Lock for {} blocks", b.0),
        OutputTimeLock::ForSeconds(s) => format!("Lock for {} seconds", s.0),
    };
    Ok(s)
}

/// Formats a transaction output into a FormattedOutput.
///
/// # Arguments
/// * `output` - A reference to the `TxOutput` enum variant to format.
/// * `coin` - The coin type (mainnet, testnet etc).
///
/// # Returns
/// A FormattedOutput containing the title and value of the output.
fn format_output(output: &TxOutput, coin: mlcp::CoinType) -> Result<FormattedOutput, StatusWord> {
    // Note: on nanox and nanosp the screen space is very limited. Moreover, if the name part of
    // a field doesn't fit into one line, it will be shrunk instead of being wrapped to the next
    // line, which will make it incomprehensible.
    // The limit is about 11-12 characters (e.g. "Chg token auth" is already too long and becomes
    // "Chg to...th").
    // So, choose names carefully and always check the screen snapshots after they're regenerated.
    // Same for inputs.

    let (name, value) = match output {
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
            if cfg!(any(target_os = "nanosplus", target_os = "nanox")) {
                ("LockThenXfer", address_short)
            } else {
                ("Lock then Transfer", address_short)
            }
        }

        TxOutput::Burn(value) => ("Burn", format_value(value, coin)?),

        TxOutput::CreateStakePool(pool_id, data) => {
            let address_short = format!(
                "Pool ID: {}\nStaker key: {}\nDecommission key: {}\nVRF public key: {}\nMargin ratio per thousand: {}\nCost per block: {}\nPledge: {}\n",
                pool_id_to_address(pool_id, coin)?,
                to_address(&data.staker, coin)?,
                to_address(&data.decommission_key, coin)?,
                vrf_to_address(&data.vrf_public_key, coin)?,
                data.margin_ratio_per_thousand.0,
                format_coin_amount_with_name(data.cost_per_block, coin),
                format_coin_amount_with_name(data.pledge, coin)
            );

            let name = if cfg!(any(target_os = "nanosplus", target_os = "nanox")) {
                "Create pool"
            } else {
                "Create staking pool"
            };
            (name, address_short)
        }

        TxOutput::ProduceBlockFromStake(destination, _pool_id) => {
            let name = if cfg!(any(target_os = "nanosplus", target_os = "nanox")) {
                "Prod block"
            } else {
                "Produce block from stake"
            };

            (
                name,
                format!("New staker key: {}", to_address(destination, coin)?),
            )
        }
        TxOutput::CreateDelegationId(destination, pool_id) => {
            let address_short = format!(
                "Address: {}\nPoolId: {}",
                to_address(destination, coin)?,
                pool_id_to_address(pool_id, coin)?
            );

            let name = if cfg!(any(target_os = "nanosplus", target_os = "nanox")) {
                "Create deleg"
            } else {
                "Create delegation"
            };

            (name, address_short)
        }

        TxOutput::DelegateStaking(amount, delegation_id) => {
            let name = if cfg!(any(target_os = "nanosplus", target_os = "nanox")) {
                "Deleg stake"
            } else {
                "Delegate staking"
            };

            (
                name,
                format!(
                    "\n{}\n{}",
                    delegation_id_to_address(delegation_id, coin)?,
                    format_value(&OutputValue::Coin(*amount), coin)?,
                ),
            )
        }
        TxOutput::IssueFungibleToken(x) => {
            let TokenIssuance::V1(data) = x;

            // Note: currently the consensus rules require that a token ticker can only be
            // ascii alphanumeric, so we could just reject non-ascii tickers here.
            // We use `make_displayable` here for simplicity and consistency.
            let ticker = make_displayable(&data.token_ticker);
            // Note: a metadata URI is allowed to have non-ascii chars, so `make_displayable`
            // is not redundant here.
            let metadata_uri = make_displayable(&data.metadata_uri);

            let total_supply_str = match data.total_supply {
                TokenTotalSupply::Unlimited => "UNLIMITED".to_string(),
                TokenTotalSupply::Lockable => "LOCKABLE".to_string(),
                TokenTotalSupply::Fixed(amount) => {
                    let formatted_amount = format_atoms(amount);
                    format!("FIXED {}", formatted_amount)
                }
            };
            let is_freezable = match data.is_freezable {
                IsTokenFreezable::Yes => "Yes",
                IsTokenFreezable::No => "No",
            };

            let address_short = format!(
                "Ticker: {}\nAuthority: {}\nMetadata URI: {}\nTotal token supply: {}\nNumber of decimals: {}\nIs freezable: {}",
                ticker,
                to_address(&data.authority, coin)?,
                metadata_uri,
                total_supply_str,
                data.number_of_decimals,
                is_freezable
            );
            let name = if cfg!(any(target_os = "nanosplus", target_os = "nanox")) {
                "Issue token"
            } else {
                "Issue fungible token"
            };

            (name, address_short)
        }

        TxOutput::IssueNft(_nft_id, data, destination) => {
            let NftIssuance::V0(data) = data;
            // Note:
            // 1. Consensus rules require that name, description and ticker are ascii alphanumeric.
            //    But same as in the IssueFungibleToken case, we use `make_displayable` for consistency
            //    and simplicity.
            // 2. The URIs are allowed to have non-ascii chars, so `make_displayable` is not redundant
            //    for them.
            // 3. There is no point in reviewing the NFT id - consensus rules require that the id is
            //    calculated from transaction inputs and if the host cheats or malfunctions, the
            //    transaction will become invalid (note that the id is only present in the output due
            //    to historical reasons and e.g. IssueFungibleToken doesn't contain the token id).
            let address_short = format!(
                "Name: {}\nDescription: {}\nCreator: {}\nTicker: {}\nAddress: {}\nIcon URI: {}\nAdditional metadata URI: {}\nMedia URI: {}, Media hash: {}",
                make_displayable(&data.name),
                make_displayable(&data.description),
                data.creator
                    .clone()
                    .map(|creator| to_address(&Destination::PublicKey(creator), coin))
                    .transpose()?
                    .unwrap_or_default(),
                make_displayable(&data.ticker),
                to_address(destination, coin)?,
                make_displayable(&data.icon_uri),
                make_displayable(&data.additional_metadata_uri),
                make_displayable(&data.media_uri),
                make_displayable(&data.media_hash),
            );

            ("Issue NFT", address_short)
        }

        TxOutput::DataDeposit(data) => ("Data deposit", const_hex::encode(data)),

        TxOutput::Htlc(value, data) => {
            let address_short = format!(
                "Secret hash: {:x}\nSpend key: {}\nRefund key: {}\nRefund time lock: {}\n{}",
                const_hex::display(data.secret_hash.0),
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

    Ok(FormattedOutput { name, value })
}

fn format_input(input: &InputCommand, coin: mlcp::CoinType) -> Result<FormattedOutput, StatusWord> {
    let (name, value) = match input {
        InputCommand::AccountSpending(cmd) => match cmd {
            AccountSpending::DelegationBalance(delegation_id, amount) => {
                let address_short = format!(
                    "Delegation ID: {}\nAmount: {}",
                    delegation_id_to_address(delegation_id, coin)?,
                    format_coin_amount_with_name(*amount, coin)
                );
                if cfg!(any(target_os = "nanosplus", target_os = "nanox")) {
                    ("Deleg wdrwl", address_short)
                } else {
                    ("Delegation withdrawal", address_short)
                }
            }
        },
        InputCommand::AccountCommand(cmd) => match cmd {
            AccountCommand::MintTokens(token_id, amount) => {
                let address_short = format!(
                    "Token ID: {}\nAmount: {}",
                    token_id_to_address(token_id, coin)?,
                    format_atoms(*amount)
                );
                ("Mint tokens", address_short)
            }
            AccountCommand::UnmintTokens(token_id) => {
                let address_short = format!("Token ID: {}", token_id_to_address(token_id, coin)?,);
                ("Unmint tokens", address_short)
            }
            AccountCommand::LockTokenSupply(token_id) => {
                let address_short = format!("Token ID: {}", token_id_to_address(token_id, coin)?);
                if cfg!(any(target_os = "nanosplus", target_os = "nanox")) {
                    ("Lock supply", address_short)
                } else {
                    ("Lock token supply", address_short)
                }
            }
            AccountCommand::FreezeToken(token_id, is_unfreezable) => {
                let address_short = format!(
                    "Token ID: {}\nIs unfreezable: {}",
                    token_id_to_address(token_id, coin)?,
                    if *is_unfreezable == IsTokenUnfreezable::Yes {
                        "Yes"
                    } else {
                        "No"
                    }
                );
                ("Freeze token", address_short)
            }
            AccountCommand::UnfreezeToken(token_id) => {
                let address_short = format!("Token ID: {}", token_id_to_address(token_id, coin)?,);
                if cfg!(any(target_os = "nanosplus", target_os = "nanox")) {
                    ("Unfrz token", address_short)
                } else {
                    ("Unfreeze token", address_short)
                }
            }
            AccountCommand::ChangeTokenAuthority(token_id, new_authority) => {
                let address_short = format!(
                    "Token ID: {}\nNew authority: {}",
                    token_id_to_address(token_id, coin)?,
                    to_address(new_authority, coin)?
                );
                if cfg!(any(target_os = "nanosplus", target_os = "nanox")) {
                    ("Chg tkn auth", address_short)
                } else {
                    ("Change token authority", address_short)
                }
            }
            AccountCommand::ChangeTokenMetadataUri(token_id, new_metadata_uri) => {
                let address_short = format!(
                    "Token ID: {}\nNew metadata URI: {}",
                    token_id_to_address(token_id, coin)?,
                    make_displayable(new_metadata_uri)
                );
                if cfg!(any(target_os = "nanosplus", target_os = "nanox")) {
                    ("Chg tkn meta", address_short)
                } else {
                    ("Change token metadata URI", address_short)
                }
            }
            AccountCommand::ConcludeOrder(_) | AccountCommand::FillOrder(_, _, _) => {
                return Err(StatusWord::OrdersV0NotSupported);
            }
        },
        InputCommand::OrderCommand(cmd, additional_info) => match cmd {
            OrderAccountCommand::FillOrder(order_id, fill_amount) => {
                let fill_value =
                    output_value_with_amount(&additional_info.initially_asked, *fill_amount);
                let address_short = format!(
                    "Order ID: {}\nFill amount: {}",
                    order_id_to_address(order_id, coin)?,
                    format_value(&fill_value, coin)?,
                );
                ("Fill order", address_short)
            }
            OrderAccountCommand::FreezeOrder(order_id) => {
                let address_short = format!("Order ID: {}", order_id_to_address(order_id, coin)?);
                ("Freeze order", address_short)
            }
            OrderAccountCommand::ConcludeOrder(order_id) => {
                let address_short = format!("Order ID: {}", order_id_to_address(order_id, coin)?);
                if cfg!(any(target_os = "nanosplus", target_os = "nanox")) {
                    ("Conclude ord", address_short)
                } else {
                    ("Conclude order", address_short)
                }
            }
        },
    };

    Ok(FormattedOutput { name, value })
}
