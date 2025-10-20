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

#![no_std]

// Required for using String, Vec, format!...
extern crate alloc;

use alloc::vec::Vec;

pub use ml_common::{
    AccountCommand, AccountOutPoint, AccountSpending, Amount, Destination, H256,
    HashedTimelockContract, HtlcSecretHash, IsTokenFreezable, IsTokenUnfreezable, NftIssuance,
    OrderAccountCommand, OrderData, OutPointSourceId, OutputTimeLock, OutputValue, PublicKeyHash,
    PublicKeyHolder, SighashInputCommitment, StakePoolData, TokenIssuance, TokenTotalSupply,
    TxInput, TxOutput, UtxoOutPoint, VRFPublicKeyHolder,
};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use parity_scale_codec::{Decode, DecodeAll, Encode};

pub const APDU_CLASS: u8 = 0xE2;

// P2 for last APDU to receive.
pub const P2_DONE: u8 = 0x00;
// P2 for more APDU to receive.
pub const P2_SIGN_MORE: u8 = 0x80;
// P1 for first APDU number.
pub const P1_SIGN_START: u8 = 0x00;
// P1 for next APDU number.
pub const P1_SIGN_NEXT: u8 = 0x01;
// P1 for maximum APDU number.
pub const P1_SIGN_MAX_CHUNKS: u8 = 0x04;
// P1 for the GET VERSION INS
pub const P1_GET_VERSION: u8 = 0x00;
// P1 for the APP NAME INS
pub const P1_APP_NAME: u8 = 0x00;

#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[num_enum(error_type(name = WrongP1P2, constructor = wrong_p1p2))]
#[repr(u8)]
pub enum PubKeyP1 {
    NoDisplayAddress = 0,
    DisplayAddress = 1,
}

impl PubKeyP1 {
    pub fn display(&self) -> bool {
        *self == Self::DisplayAddress
    }
}

pub struct Ins {}

impl Ins {
    pub const GET_VERSION: u8 = 0x00;
    pub const APP_NAME: u8 = 0x01;
    pub const PUB_KEY: u8 = 0x02;
    pub const SIGN_TX: u8 = 0x03;
    pub const SIGN_MSG: u8 = 0x04;
}

pub struct WrongP1P2;
fn wrong_p1p2(_: u8) -> WrongP1P2 {
    WrongP1P2
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[num_enum(error_type(name = WrongP1P2, constructor = wrong_p1p2))]
#[repr(u8)]
pub enum P1SignTx {
    Metadata = 0,
    Input = 1,
    InputAdditionalInfo = 2,
    Output = 3,
    NextSignature = 4,
}

#[derive(Encode, Decode)]
pub struct PublicKeyReq {
    pub coin_type: CoinType,
    pub path: Bip32Path,
}

#[derive(Encode, Decode)]
pub struct SignMessageReq {
    pub coin: CoinType,
    pub addr_type: AddrType,
    pub path: Bip32Path,
}

#[derive(Encode, Decode)]
pub enum SignTxReq {
    Input(TxInputReq),
    InputAdditionalInfo(InputAdditionalInfoReq),
    Output(TxOutputReq),
    NextSignature,
}

#[derive(Encode, Decode)]
pub struct TxMetadataReq {
    pub coin: CoinType,
    pub version: u8,
    pub num_inputs: u32,
    pub num_outputs: u32,
}

#[derive(Encode, Decode)]
pub struct TxInputReq {
    pub addresses: Vec<InputAddressPath>,
    pub inp: TxInput,
}

#[derive(Encode, Decode)]
pub enum InputAdditionalInfoReq {
    None,
    Utxo {
        utxo: TxOutput,
    },
    PoolInfo {
        utxo: TxOutput,
        staker_balance: Amount,
    },
    OrderInfo {
        initially_asked: OutputValue,
        initially_given: OutputValue,
        ask_balance: Amount,
        give_balance: Amount,
    },
}

#[derive(Encode, Decode)]
pub struct TxOutputReq {
    pub out: TxOutput,
}

#[derive(Encode, Decode, Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive)]
#[repr(u8)]
pub enum CoinType {
    Mainnet = 0,
    Testnet = 1,
    Regtest = 2,
    Signet = 3,
}

impl CoinType {
    pub const fn coin_ticker(&self) -> &'static str {
        match self {
            Self::Mainnet => "ML",
            Self::Testnet => "TML",
            Self::Regtest => "RML",
            Self::Signet => "SML",
        }
    }

    pub const fn bip44_coin_type(&self) -> u32 {
        let hardened_bit = 1 << 31;
        match self {
            Self::Mainnet => 19788 + hardened_bit,
            Self::Testnet | Self::Regtest | Self::Signet => 1 + hardened_bit,
        }
    }

    pub const fn coin_decimals(&self) -> u8 {
        11
    }

    pub const fn address_prefix(&self, destination: &Destination) -> &'static str {
        match self {
            Self::Mainnet => match destination {
                Destination::AnyoneCanSpend => "mxanyonecanspend",
                Destination::PublicKeyHash(_) => "mtc",
                Destination::PublicKey(_) => "mptc",
                Destination::ScriptHash(_) => "mstc",
                Destination::ClassicMultisig(_) => "mmtc",
            },
            Self::Testnet => match destination {
                Destination::AnyoneCanSpend => "txanyonecanspend",
                Destination::PublicKeyHash(_) => "tmt",
                Destination::PublicKey(_) => "tpmt",
                Destination::ScriptHash(_) => "tstc",
                Destination::ClassicMultisig(_) => "tmtc",
            },
            Self::Regtest => match destination {
                Destination::AnyoneCanSpend => "rxanyonecanspend",
                Destination::PublicKeyHash(_) => "rmt",
                Destination::PublicKey(_) => "rpmt",
                Destination::ScriptHash(_) => "rstc",
                Destination::ClassicMultisig(_) => "rmtc",
            },
            Self::Signet => match destination {
                Destination::AnyoneCanSpend => "sxanyonecanspend",
                Destination::PublicKeyHash(_) => "smt",
                Destination::PublicKey(_) => "spmt",
                Destination::ScriptHash(_) => "sstc",
                Destination::ClassicMultisig(_) => "smtc",
            },
        }
    }

    pub const fn pool_id_address_prefix(&self) -> &'static str {
        match self {
            Self::Mainnet => "mpool",
            Self::Testnet => "tpool",
            Self::Regtest => "rpool",
            Self::Signet => "spool",
        }
    }

    pub const fn delegation_id_address_prefix(&self) -> &'static str {
        match self {
            Self::Mainnet => "mdelg",
            Self::Testnet => "tdelg",
            Self::Regtest => "rdelg",
            Self::Signet => "sdelg",
        }
    }

    pub const fn token_id_address_prefix(&self) -> &'static str {
        match self {
            Self::Mainnet => "mmltk",
            Self::Testnet => "tmltk",
            Self::Regtest => "rmltk",
            Self::Signet => "smltk",
        }
    }

    pub const fn order_id_address_prefix(&self) -> &'static str {
        match self {
            Self::Mainnet => "mordr",
            Self::Testnet => "tordr",
            Self::Regtest => "rordr",
            Self::Signet => "sordr",
        }
    }

    pub const fn vrf_public_key_address_prefix(&self) -> &'static str {
        match self {
            Self::Mainnet => "mvrfpk",
            Self::Testnet => "tvrfpk",
            Self::Regtest => "rvrfpk",
            Self::Signet => "svrfpk",
        }
    }
}

#[repr(u8)]
#[derive(Encode, Decode, Clone, Copy, IntoPrimitive)]
pub enum AddrType {
    PublicKey = 0,
    PublicKeyHash = 1,
}

/// BIP32 path stored as an array of [`u32`].
#[derive(Default, Encode, Decode, Clone)]
pub struct Bip32Path(pub Vec<u32>);

impl AsRef<[u32]> for Bip32Path {
    fn as_ref(&self) -> &[u32] {
        &self.0
    }
}

/// Address path to be signed for an input
#[derive(Encode, Decode)]
pub struct InputAddressPath {
    pub path: Bip32Path,
    pub multisig_idx: Option<u32>,
}

pub fn encode<T: Encode>(t: T) -> Vec<u8> {
    t.encode()
}

pub fn decode_all<T: Decode>(mut bytes: &[u8]) -> Option<T> {
    T::decode_all(&mut bytes).ok()
}
