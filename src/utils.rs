use alloc::vec::Vec;

use ml_common::Destination;
use parity_scale_codec::Decode;

use crate::AppSW;

/// BIP32 path stored as an array of [`u32`].
#[derive(Default, Decode)]
pub struct Bip32Path(Vec<u32>);

impl AsRef<[u32]> for Bip32Path {
    fn as_ref(&self) -> &[u32] {
        &self.0
    }
}

#[repr(u8)]
#[derive(Decode, Clone, Copy)]
pub enum AddrType {
    PublicKey = 0,
    PublicKeyHash = 1,
}

impl TryFrom<u8> for AddrType {
    type Error = AppSW;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let addr_type = match value {
            0 => Self::PublicKey,
            1 => Self::PublicKeyHash,
            _ => return Err(AppSW::DeserializeFail),
        };

        Ok(addr_type)
    }
}

#[repr(u8)]
#[derive(Decode, Clone, Copy)]
pub enum CoinType {
    Mainnet = 0,
    Testnet = 1,
    Regtest = 2,
    Signet = 3,
}

impl TryFrom<u8> for CoinType {
    type Error = AppSW;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let coin = match value {
            0 => Self::Mainnet,
            1 => Self::Testnet,
            2 => Self::Regtest,
            3 => Self::Signet,
            _ => return Err(AppSW::DeserializeFail),
        };

        Ok(coin)
    }
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

    pub const fn coin_path(&self) -> u32 {
        let hardened_bit = 1 << 31;
        match self {
            Self::Mainnet => 19788 + hardened_bit,
            Self::Testnet | Self::Regtest | Self::Signet => 1 + hardened_bit,
        }
    }

    pub const fn coin_decimals(&self) -> u8 {
        11
    }

    pub const fn address_prefix(&self, destination_tag: &Destination) -> &'static str {
        match self {
            Self::Mainnet => match destination_tag {
                Destination::AnyoneCanSpend => "mxanyonecanspend",
                Destination::PublicKeyHash(_) => "mtc",
                Destination::PublicKey(_) => "mptc",
                Destination::ScriptHash(_) => "mstc",
                Destination::ClassicMultisig(_) => "mmtc",
            },
            Self::Testnet => match destination_tag {
                Destination::AnyoneCanSpend => "txanyonecanspend",
                Destination::PublicKeyHash(_) => "tmt",
                Destination::PublicKey(_) => "tpmt",
                Destination::ScriptHash(_) => "tstc",
                Destination::ClassicMultisig(_) => "tmtc",
            },
            Self::Regtest => match destination_tag {
                Destination::AnyoneCanSpend => "rxanyonecanspend",
                Destination::PublicKeyHash(_) => "rmt",
                Destination::PublicKey(_) => "rpmt",
                Destination::ScriptHash(_) => "rstc",
                Destination::ClassicMultisig(_) => "rmtc",
            },
            Self::Signet => match destination_tag {
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
