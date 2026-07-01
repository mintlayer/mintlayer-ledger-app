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

use alloc::{borrow::Cow, format};
use core::fmt;

use mintlayer_messages::StatusWord;

use crate::mlcp;

const DERIV_PATH_IDX_BIP44: usize = 0;
const DERIV_PATH_IDX_COIN_TYPE: usize = 1;
const DERIV_PATH_IDX_ACCOUNT_IDX: usize = 2;
const DERIV_PATH_IDX_ADDR_PURPOSE: usize = 3;
const DERIV_PATH_IDX_ADDR_IDX: usize = 4;

// Path should be at least [bip44, coin_type, account_index]
const DERIV_PATH_MIN_LEN: usize = 3;
// For tx signing the path should also contain the purpose and the index.
const DERIV_PATH_LEN_FOR_TX_SIGNING: usize = 5;

const DERIV_PATH_BIP44_ITEM: u32 = 44 + (1 << 31);

pub fn check_derivation_path(path: &[u32], coin_type: mlcp::CoinType) -> Result<(), StatusWord> {
    if path.len() < DERIV_PATH_MIN_LEN {
        return Err(StatusWord::InvalidPath);
    }

    if path[DERIV_PATH_IDX_BIP44] != DERIV_PATH_BIP44_ITEM {
        return Err(StatusWord::InvalidPath);
    }

    if path[DERIV_PATH_IDX_COIN_TYPE] != coin_type.bip44_coin_type() {
        return Err(StatusWord::InvalidPath);
    }

    Ok(())
}

pub fn check_derivation_path_for_tx_signing(
    path: &[u32],
    coin_type: mlcp::CoinType,
) -> Result<CompressedDerivationPathForTxSigning, StatusWord> {
    check_derivation_path(path, coin_type)?;

    if path.len() != DERIV_PATH_LEN_FOR_TX_SIGNING {
        return Err(StatusWord::InvalidPath);
    }

    Ok(CompressedDerivationPathForTxSigning {
        account_index: path[DERIV_PATH_IDX_ACCOUNT_IDX],
        addr_purpose: path[DERIV_PATH_IDX_ADDR_PURPOSE],
        addr_index: path[DERIV_PATH_IDX_ADDR_IDX],
    })
}

pub struct CompressedDerivationPathForTxSigning {
    pub account_index: u32,
    pub addr_purpose: u32,
    pub addr_index: u32,
}

impl CompressedDerivationPathForTxSigning {
    pub fn to_full_path(&self, coin_type: mlcp::CoinType) -> [u32; DERIV_PATH_LEN_FOR_TX_SIGNING] {
        [
            DERIV_PATH_BIP44_ITEM,
            coin_type.bip44_coin_type(),
            self.account_index,
            self.addr_purpose,
            self.addr_index,
        ]
    }
}

/// Cut an array of Copy types to a smaller size.
///
/// Fail at compile time if the destination size is bigger than the original.
pub fn cut_array<T, const ORIG_SIZE: usize, const DEST_SIZE: usize>(
    orig: &[T; ORIG_SIZE],
) -> [T; DEST_SIZE]
where
    T: Default + Copy,
{
    const { assert!(DEST_SIZE <= ORIG_SIZE) };

    let mut result = [T::default(); DEST_SIZE];
    result.copy_from_slice(&orig[..DEST_SIZE]);
    result
}

/// If `bytes` consists of only "displayable" ASCII chars, return the ASCII string directly
/// in the borrowed form.
/// Otherwise return hex-encode bytes as a newly allocated string.
pub fn make_displayable_str(bytes: &[u8]) -> Cow<'_, str> {
    if let Some(s) = as_displayable_chars(bytes) {
        Cow::Borrowed(s)
    } else {
        Cow::Owned(format!("{:#x}", const_hex::display(bytes)))
    }
}

/// This is similar to `make_displayable_str`, but instead of returning a string it returns
/// an object that will produce the string during formatting.
///
/// This should be preferred to `make_displayable_str` if the result has to be passed to
/// `format!` anyway, because it should result in fewer reallocations.
pub fn make_displayable<'a>(bytes: &'a [u8]) -> impl fmt::Display + 'a {
    if let Some(s) = as_displayable_chars(bytes) {
        DisplayableMessage::Str(s)
    } else {
        DisplayableMessage::Hex(bytes)
    }
}

enum DisplayableMessage<'a> {
    Str(&'a str),
    Hex(&'a [u8]),
}

impl<'a> fmt::Display for DisplayableMessage<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Str(s) => f.write_str(s),
            Self::Hex(bytes) => {
                write!(f, "{:#x}", const_hex::display(bytes))
            }
        }
    }
}

fn as_displayable_chars(bytes: &[u8]) -> Option<&'_ str> {
    let as_str = core::str::from_utf8(bytes).ok()?;
    as_str
        .bytes()
        .all(|b| (0x20..=0x7E).contains(&b))
        .then_some(as_str)
}

#[cfg(test)]
mod tests {
    use test_utils::prelude::*;

    use super::*;

    #[test_item]
    fn test_make_displayable() {
        // ASCII displayable string.
        {
            let data = "qwe123".as_bytes();
            let expected_res = "qwe123";

            let res = make_displayable_str(data);
            assert_eq!(res, expected_res);

            let res = format!("{}", make_displayable(data));
            assert_eq!(res, expected_res);
        }

        // ASCII non-displayable string.
        {
            let data = "qwe\t123".as_bytes();
            let expected_res = "0x71776509313233";

            let res = make_displayable_str(data);
            assert_eq!(res, expected_res);

            let res = format!("{}", make_displayable(data));
            assert_eq!(res, expected_res);
        }

        // Non-ASCII string.
        {
            let data = "今日は".as_bytes();
            let expected_res = "0xe4bb8ae697a5e381af";

            let res = make_displayable_str(data);
            assert_eq!(res, expected_res);

            let res = format!("{}", make_displayable(data));
            assert_eq!(res, expected_res);
        }

        // Not a string.
        {
            let data = [0xAA, 0xBB, 0xCC];
            let expected_res = "0xaabbcc";

            let res = make_displayable_str(&data);
            assert_eq!(res, expected_res);

            let res = format!("{}", make_displayable(&data));
            assert_eq!(res, expected_res);
        }
    }
}
