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

pub use mintlayer_core_primitives::{
    AccountCommand, AccountNonce, AccountOutPoint, AccountSpending, Amount, CoinType as PCoinType,
    Destination, H256, HashedTimelockContract, HtlcSecretHash, Id, IsTokenFreezable,
    IsTokenUnfreezable, NftIssuance, OrderAccountCommand, OrderData, OutPointSourceId,
    OutputTimeLock, OutputValue, PublicKey, PublicKeyHash, SchnorrkelPublicKey, Secp256k1PublicKey,
    SighashInputCommitment, StakePoolData, TokenIssuance, TokenTotalSupply, TxInput, TxOutput,
    UtxoOutPoint, VrfPublicKey,
};
use num_enum::{IntoPrimitive, TryFromPrimitive};
pub use parity_scale_codec::Encode;
use parity_scale_codec::{Decode, DecodeAll};

pub const APDU_CLASS: u8 = 0xE1;
pub const MAX_ADPU_DATA_LEN: usize = u8::MAX as usize;

// P2 is used to indicate APDU chunking.
// `P2_DONE` marks the final chunk, while `P2_MORE` indicates that more chunks follow.
pub const P2_DONE: u8 = 0x00;
pub const P2_MORE: u8 = 0x80;

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
    pub const PUB_KEY: u8 = 0x00;
    pub const SIGN_TX: u8 = 0x01;
    pub const SIGN_MSG: u8 = 0x02;
}

pub struct WrongP1P2;
fn wrong_p1p2(_: u8) -> WrongP1P2 {
    WrongP1P2
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[num_enum(error_type(name = WrongP1P2, constructor = wrong_p1p2))]
#[repr(u8)]
pub enum SignP1 {
    Start = 0,
    Next = 1,
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
    InputCommitment(SighashInputCommitment),
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
    pub inp: TxInputWithAdditionalInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct AdditionalOrderInfo {
    pub initially_asked: OutputValue,
    pub initially_given: OutputValue,
    pub ask_balance: Amount,
    pub give_balance: Amount,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum AdditionalUtxoInfo {
    Utxo(TxOutput),
    PoolData {
        utxo: TxOutput,
        staker_balance: Amount,
    },
}

impl From<AdditionalUtxoInfo> for SighashInputCommitment {
    fn from(value: AdditionalUtxoInfo) -> Self {
        match value {
            AdditionalUtxoInfo::Utxo(output) => SighashInputCommitment::Utxo(output),
            AdditionalUtxoInfo::PoolData {
                utxo,
                staker_balance,
            } => SighashInputCommitment::ProduceBlockFromStakeUtxo {
                utxo,
                staker_balance,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum TxInputWithAdditionalInfo {
    #[codec(index = 0)]
    Utxo(UtxoOutPoint, AdditionalUtxoInfo),

    #[codec(index = 1)]
    Account(AccountOutPoint),

    #[codec(index = 2)]
    AccountCommand(AccountNonce, AccountCommand),

    #[codec(index = 3)]
    OrderAccountCommand(OrderAccountCommand, AdditionalOrderInfo),
}

impl TxInputWithAdditionalInfo {
    pub fn into_input_and_commitment(self) -> (TxInput, SighashInputCommitment) {
        match self {
            TxInputWithAdditionalInfo::Utxo(utxo, info) => (TxInput::Utxo(utxo), info.into()),
            TxInputWithAdditionalInfo::Account(acc) => {
                (TxInput::Account(acc), SighashInputCommitment::None)
            }
            TxInputWithAdditionalInfo::AccountCommand(nonce, cmd) => (
                TxInput::AccountCommand(nonce, cmd),
                SighashInputCommitment::None,
            ),
            TxInputWithAdditionalInfo::OrderAccountCommand(cmd, info) => {
                let commitment = match &cmd {
                    OrderAccountCommand::FillOrder(_, _) => {
                        SighashInputCommitment::FillOrderAccountCommand {
                            initially_asked: info.initially_asked,
                            initially_given: info.initially_given,
                        }
                    }
                    OrderAccountCommand::ConcludeOrder(_) => {
                        SighashInputCommitment::ConcludeOrderAccountCommand {
                            initially_asked: info.initially_asked,
                            initially_given: info.initially_given,
                            ask_balance: info.ask_balance,
                            give_balance: info.give_balance,
                        }
                    }
                    OrderAccountCommand::FreezeOrder(_) => SighashInputCommitment::None,
                };
                (TxInput::OrderAccountCommand(cmd), commitment)
            }
        }
    }
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

impl From<CoinType> for PCoinType {
    fn from(value: CoinType) -> Self {
        match value {
            CoinType::Mainnet => Self::Mainnet,
            CoinType::Testnet => Self::Testnet,
            CoinType::Regtest => Self::Regtest,
            CoinType::Signet => Self::Signet,
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

#[derive(Encode, Decode)]
pub enum Prerelease {
    Alpha,
    Beta,
}

#[derive(Encode, Decode)]
pub struct GetVersionRespones {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
    pub prerelease_id: Option<Prerelease>,
    pub build_metadata: Vec<u8>,
}

#[derive(Encode, Decode)]
pub struct GetPublicKeyRespones {
    pub public_key: [u8; 65],
    pub chain_code: [u8; 32],
}

#[derive(Encode, Decode)]
pub struct Signature {
    pub signature: [u8; 64],
    pub multisig_idx: Option<u32>,
}

#[derive(Encode, Decode)]
pub struct MsgSignature {
    pub signature: [u8; 64],
}

pub fn encode<T: Encode>(t: T) -> Vec<u8> {
    t.encode()
}

pub fn encode_to<T: Encode>(t: T, buf: &mut Vec<u8>) {
    t.encode_to(buf)
}

pub fn decode_all<T: Decode>(mut bytes: &[u8]) -> Option<T> {
    T::decode_all(&mut bytes).ok()
}

pub fn encode_as_compact(num: u32) -> Vec<u8> {
    parity_scale_codec::Compact::<u32>::encode(&num.into())
}

/// This represents an APDU used in communication with Mintlayer Ledger app.
///
/// Note that the class byte is not present here, because it's the same for all our APDUs.
///
/// Also, we don't have the second parameter byte here either, because its meaning is the same
/// across all APDUs - it specifies whether this APDU represents the last chunk of the instruction.
pub struct Apdu<'a> {
    instruction_byte: u8,
    param1_byte: u8,
    command_data: &'a [u8],
    is_last_chunk: bool,
}

impl<'a> Apdu<'a> {
    /// Create an APDU with data; this will fail if the data length exceeds the allowed maximum.
    pub fn new_with_data(
        instruction_byte: u8,
        param1_byte: u8,
        command_data: &'a [u8],
    ) -> Option<Self> {
        (command_data.len() <= MAX_ADPU_DATA_LEN).then(|| Self {
            instruction_byte,
            param1_byte,
            command_data,
            is_last_chunk: true,
        })
    }

    /// Creates a Vec of APDUs by chunking the data to MAX_ADPU_DATA_LEN
    pub fn new_chunks(instruction_byte: u8, param1_byte: u8, data: &'a [u8]) -> Vec<Self> {
        let mut adpus = Vec::new();
        let mut chunks_iter = data.chunks(MAX_ADPU_DATA_LEN).peekable();
        while let Some(chunk) = chunks_iter.next() {
            let apdu = Self {
                instruction_byte,
                param1_byte,
                command_data: chunk,
                is_last_chunk: chunks_iter.peek().is_none(),
            };
            adpus.push(apdu);
        }
        adpus
    }

    pub fn is_last(&self) -> bool {
        self.is_last_chunk
    }

    /// The number of bytes that will be written by `write_bytes`.
    ///
    /// This can be used to reserve the required capacity in the destination collection
    /// (note that `Extend::extend_reserve is` still nightly-only, so we can't use it).
    pub fn bytes_count(&self) -> usize {
        // class, instruction, param1 and param2 bytes, then 1 byte for data length, then the
        // data itself.
        5 + self.command_data.len()
    }

    pub fn write_bytes(&self, collection: &mut impl core::iter::Extend<u8>) {
        let param2_byte = if self.is_last_chunk { P2_DONE } else { P2_MORE };

        collection.extend([
            APDU_CLASS,
            self.instruction_byte,
            self.param1_byte,
            param2_byte,
        ]);
        // Should be true by construction
        assert!(self.command_data.len() <= u8::MAX as usize);
        collection.extend(core::iter::once(self.command_data.len() as u8));
        collection.extend(self.command_data.iter().copied());
    }
}
