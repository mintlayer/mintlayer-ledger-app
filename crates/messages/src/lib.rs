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

#![no_std]

// Required for using String, Vec, format!...
extern crate alloc;

use alloc::{boxed::Box, vec::Vec};
use core::iter::ExactSizeIterator;

use derive_more::Display;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use parity_scale_codec::{Decode, DecodeAll};

pub use mintlayer_core_primitives as mlcp;
pub use parity_scale_codec::Encode;

pub const APDU_CLASS: u8 = 0xE1;
pub const MAX_ADPU_DATA_LEN: usize = u8::MAX as usize;

// P2 is used to indicate APDU chunking.
// `P2_DONE` marks the final chunk, while `P2_MORE` indicates that more chunks follow.
pub const P2_DONE: u8 = 0x00;
pub const P2_MORE: u8 = 0x80;

fn wrong_p1p2(_: u8) -> StatusWord {
    StatusWord::WrongP1P2
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[num_enum(error_type(name = StatusWord, constructor = wrong_p1p2))]
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
    pub const PING: u8 = 0x03;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[num_enum(error_type(name = StatusWord, constructor = wrong_p1p2))]
#[repr(u8)]
pub enum SignP1 {
    Start = 0,
    Next = 1,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[num_enum(error_type(name = StatusWord, constructor = wrong_p1p2))]
#[repr(u8)]
pub enum PingP1 {
    Start = 0,
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
    Input(Box<TxInputReq>),
    InputCommitment(Box<mlcp::SighashInputCommitment>),
    Output(Box<TxOutputReq>),
    NextSignature,
}

#[derive(Encode, Decode)]
pub struct TxMetadataV1Req {
    pub num_inputs: u32,
    pub num_outputs: u32,
}

#[derive(Encode, Decode)]
pub enum TxMetadataVersionReq {
    V1(TxMetadataV1Req),
}

#[derive(Encode, Decode)]
pub struct TxMetadataReq {
    pub coin: CoinType,
    pub version: TxMetadataVersionReq,
}

#[derive(Encode, Decode)]
pub struct TxInputReq {
    pub addresses: Vec<InputAddressPath>,
    pub inp: TxInputWithAdditionalInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct AdditionalOrderInfo {
    pub initially_asked: mlcp::OutputValue,
    pub initially_given: mlcp::OutputValue,
    pub ask_balance: mlcp::Amount,
    pub give_balance: mlcp::Amount,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum AdditionalUtxoInfo {
    Utxo(mlcp::TxOutput),
    UtxoWithPoolData {
        utxo: mlcp::TxOutput,
        staker_balance: mlcp::Amount,
    },
}

impl From<AdditionalUtxoInfo> for mlcp::SighashInputCommitment {
    fn from(value: AdditionalUtxoInfo) -> Self {
        match value {
            AdditionalUtxoInfo::Utxo(output) => mlcp::SighashInputCommitment::Utxo(output),
            AdditionalUtxoInfo::UtxoWithPoolData {
                utxo,
                staker_balance,
            } => mlcp::SighashInputCommitment::ProduceBlockFromStakeUtxo {
                utxo,
                staker_balance,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum TxInputWithAdditionalInfo {
    #[codec(index = 0)]
    Utxo(mlcp::UtxoOutPoint, AdditionalUtxoInfo),

    #[codec(index = 1)]
    Account(mlcp::AccountOutPoint),

    #[codec(index = 2)]
    AccountCommand(mlcp::AccountNonce, mlcp::AccountCommand),

    #[codec(index = 3)]
    OrderAccountCommand(mlcp::OrderAccountCommand, AdditionalOrderInfo),
}

impl TxInputWithAdditionalInfo {
    pub fn into_input_and_commitment(self) -> (mlcp::TxInput, mlcp::SighashInputCommitment) {
        match self {
            TxInputWithAdditionalInfo::Utxo(utxo, info) => (mlcp::TxInput::Utxo(utxo), info.into()),
            TxInputWithAdditionalInfo::Account(acc) => (
                mlcp::TxInput::Account(acc),
                mlcp::SighashInputCommitment::None,
            ),
            TxInputWithAdditionalInfo::AccountCommand(nonce, cmd) => (
                mlcp::TxInput::AccountCommand(nonce, cmd),
                mlcp::SighashInputCommitment::None,
            ),
            TxInputWithAdditionalInfo::OrderAccountCommand(cmd, info) => {
                let commitment = match &cmd {
                    mlcp::OrderAccountCommand::FillOrder(_, _) => {
                        mlcp::SighashInputCommitment::FillOrderAccountCommand {
                            initially_asked: info.initially_asked,
                            initially_given: info.initially_given,
                        }
                    }
                    mlcp::OrderAccountCommand::ConcludeOrder(_) => {
                        mlcp::SighashInputCommitment::ConcludeOrderAccountCommand {
                            initially_asked: info.initially_asked,
                            initially_given: info.initially_given,
                            ask_balance: info.ask_balance,
                            give_balance: info.give_balance,
                        }
                    }
                    mlcp::OrderAccountCommand::FreezeOrder(_) => mlcp::SighashInputCommitment::None,
                };
                (mlcp::TxInput::OrderAccountCommand(cmd), commitment)
            }
        }
    }
}

#[derive(Encode, Decode)]
pub struct TxOutputReq {
    pub out: mlcp::TxOutput,
}

#[derive(Encode, Decode, Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive)]
#[repr(u8)]
pub enum CoinType {
    Mainnet = 0,
    Testnet = 1,
    Regtest = 2,
    Signet = 3,
}

impl From<CoinType> for mlcp::CoinType {
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
pub struct PublicKey(pub [u8; 65]);

#[derive(Encode, Decode)]
pub struct ChainCode(pub [u8; 32]);

#[derive(Encode, Decode)]
pub struct GetPublicKeyResponse {
    pub public_key: PublicKey,
    pub chain_code: ChainCode,
}

#[derive(Encode, Decode)]
pub struct SignatureResponse(pub [u8; 64]);

#[derive(Encode, Decode)]
pub struct TxInputSignatureResponse {
    pub signature: SignatureResponse,
    pub input_idx: u32,
    pub multisig_idx: Option<u32>,
    pub has_next: bool,
}

#[derive(Encode, Decode)]
pub struct MsgSignatureResponse {
    pub signature: SignatureResponse,
}

#[derive(Encode, Decode)]
pub enum Response {
    #[codec(index = 0)]
    ExpectingNextChunk,
    #[codec(index = 1)]
    PublicKey(GetPublicKeyResponse),
    #[codec(index = 2)]
    TxSetup,
    #[codec(index = 3)]
    TxNext,
    #[codec(index = 4)]
    TxSignature(TxInputSignatureResponse),
    #[codec(index = 5)]
    MessageSetup,
    #[codec(index = 6)]
    MessageSignature(MsgSignatureResponse),
    #[codec(index = 7)]
    Pong,
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
        (command_data.len() <= MAX_ADPU_DATA_LEN).then_some(Self {
            instruction_byte,
            param1_byte,
            command_data,
            is_last_chunk: true,
        })
    }

    /// Returns an ExactSizeIterator of APDUs by chunking the data to MAX_ADPU_DATA_LEN
    pub fn new_chunks(
        instruction_byte: u8,
        param1_byte: u8,
        data: &'a [u8],
    ) -> impl ExactSizeIterator<Item = Self> {
        let chunk_iter = data.chunks(MAX_ADPU_DATA_LEN);
        let last_chunk_idx = chunk_iter.len() - 1;

        chunk_iter.enumerate().map(move |(chunk_idx, chunk)| Self {
            instruction_byte,
            param1_byte,
            command_data: chunk,
            is_last_chunk: chunk_idx == last_chunk_idx,
        })
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

/// Application status words.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Display, IntoPrimitive, TryFromPrimitive)]
pub enum StatusWord {
    // Standard Ledger APDU Codes
    #[display("Success")]
    Ok = 0x9000,
    #[display("Nothing received")]
    NothingReceived = 0x6982,
    #[display("User cancelled")]
    Deny = 0x6985,
    #[display("CLA not supported")]
    ClaNotSupported = 0x6E00,
    #[display("Instruction not supported")]
    InsNotSupported = 0x6E01,
    #[display("Wrong P1/P2 parameters")]
    WrongP1P2 = 0x6E02,
    #[display("Wrong APDU length")]
    WrongApduLength = 0x6E03,
    #[display("Unknown")]
    Unknown = 0x6D00,
    #[display("Panic")]
    Panic = 0xE000,
    #[display("Device locked")]
    DeviceLocked = 0x5515,

    // App Specific Errors (0xB...)
    #[display("Transaction display failed")]
    TxDisplayFail = 0xB000,
    #[display("Transaction lock time value is invalid")]
    TxLockTimeInvalid = 0xB001,
    #[display("Transaction wrong length")]
    TxWrongLength = 0xB002,
    #[display("Transaction hashing failed")]
    TxHashFail = 0xB003,
    #[display("Transaction address failed")]
    TxAddressFail = 0xB004,
    #[display("Different instruction than expected")]
    WrongInstruction = 0xB005,
    #[display("Key derivation failed")]
    KeyDeriveFail = 0xB006,
    #[display("Orders V0 not supported")]
    OrdersV0NotSupported = 0xB007,
    #[display("Wrong context")]
    WrongContext = 0xB008,
    #[display("Deserialization failed")]
    DeserializeFail = 0xB009,
    #[display("Invalid input UTXO")]
    TxInvalidInputUtxo = 0xB00A,
    #[display("Numeric operation failed")]
    TxNumericOperationFail = 0xB00B,
    #[display("Tx fee underflow")]
    TxFeeUnderflow = 0xB00C,
    #[display("Invalid input path")]
    TxInvalidInputPath = 0xB00D,
    #[display("Nothing to sign")]
    NothingToSign = 0xB00E,
    #[display("Transaction already finished")]
    TxAlreadyFinished = 0xB00F,
    #[display("Invalid path")]
    InvalidPath = 0xB010,
    #[display("Invalid uncompressed public key")]
    InvalidUncompressedPublicKey = 0xB011,
    #[display("Max buffer length exceeded")]
    MaxBufferLenExceeded = 0xB012,
    #[display("Different input commitment hash")]
    DifferentInputCommitmentHash = 0xB013,
    #[display("Invalid Timestamp")]
    InvalidTimestamp = 0xB014,

    // Ecc Errors
    #[display("ECC Carry")]
    EccCarry = 0xB100,
    #[display("ECC Locked")]
    EccLocked = 0xB101,
    #[display("ECC Unlocked")]
    EccUnlocked = 0xB102,
    #[display("ECC Not Locked")]
    EccNotLocked = 0xB103,
    #[display("ECC Not Unlocked")]
    EccNotUnlocked = 0xB104,
    #[display("ECC Internal Error")]
    EccInternalError = 0xB105,
    #[display("ECC Invalid Parameter Size")]
    EccInvalidParameterSize = 0xB106,
    #[display("ECC Invalid Parameter Value")]
    EccInvalidParameterValue = 0xB107,
    #[display("ECC Invalid Parameter")]
    EccInvalidParameter = 0xB108,
    #[display("ECC Not Invertible")]
    EccNotInvertible = 0xB109,
    #[display("ECC Overflow")]
    EccOverflow = 0xB10A,
    #[display("ECC Memory Full")]
    EccMemoryFull = 0xB10B,
    #[display("ECC No Residue")]
    EccNoResidue = 0xB10C,
    #[display("ECC Point At Infinity")]
    EccPointAtInfinity = 0xB10D,
    #[display("ECC Invalid Point")]
    EccInvalidPoint = 0xB10E,
    #[display("ECC Invalid Curve")]
    EccInvalidCurve = 0xB10F,
    #[display("ECC Generic Error")]
    EccGenericError = 0xB110,
}
