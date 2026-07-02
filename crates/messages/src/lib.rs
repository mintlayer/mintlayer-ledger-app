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
// See the comment in `crates/test-utils/src/lib.rs` for the meaning of these attributes.
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(ledger_device_sdk::testing::sdk_test_runner)]
#![reexport_test_harness_main = "test_main"]

#[cfg(test)]
test_utils::impl_panic_handler!();
#[cfg(test)]
test_utils::impl_main!();

#[cfg(test)]
mod tests;

// TODO: need tests that ensure encoding stability - encode a certain message or a message part and
// expect concrete bytes, decode it back, expect the same object.
// See https://github.com/mintlayer/mintlayer-ledger-app/issues/16.

// TODO: types from mintlayer core primitives should probably not be used as part of the protocol
// (but note that this will increase the size of the binary slightly - a test attempt at using distinct
// types increased the binary from ~100Kb to ~106Kb).
// Alternatively, we may want to keep some basic mlcp types in the protocol (the primitives that
// won't ever change). Though note that we'll have to have a separate type for Destination if we
// want to be able to detect change addresses, so all types that contain Destination will have to
// be distinct as well.
// See https://github.com/mintlayer/mintlayer-ledger-app/issues/18.
// Also see the TODO in `tests/application_client/__init__.py`.

// Required for using String, Vec, format!...
extern crate alloc;

use alloc::{boxed::Box, vec::Vec};
use core::iter::ExactSizeIterator;

use derive_more::Display;
use mintlayer_core_primitives as mlcp;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use parity_scale_codec::{Decode, DecodeAll};

pub use parity_scale_codec::{self, Encode};

pub use mlcp::{
    AccountCommand, AccountNonce, AccountOutPoint, AccountSpending, Amount, BlockHeight,
    BlockTimestamp, BlocksCount, DelegationId, Destination, GenBlockId, H256,
    HashedTimelockContract, IsTokenFreezable, IsTokenUnfreezable, NftIssuance, OrderAccountCommand,
    OrderData, OrderId, OutPointSourceId, OutputTimeLock, OutputValue, PerThousand, PoolId,
    PublicKey, PublicKeyHash, ScriptId, SecondsCount, Secp256k1PublicKey, SighashInputCommitment,
    StakePoolData, TokenId, TokenIssuance, TokenTotalSupply, TransactionId, TxInput, TxOutput,
    UtxoOutPoint, VrfPublicKey,
};

pub const APDU_CLASS: u8 = 0xE1;
pub const MAX_APDU_DATA_LEN: usize = u8::MAX as usize;

// P2 is used to indicate APDU chunking.
// `P2_DONE` marks the final chunk, while `P2_MORE` indicates that more chunks follow.
pub const P2_DONE: u8 = 0x00;
pub const P2_MORE: u8 = 0x80;

fn wrong_p1p2(_: u8) -> StatusWord {
    StatusWord::WrongP1P2
}

pub struct Ins {}

impl Ins {
    pub const GET_PUB_KEY: u8 = 0x00;
    pub const SIGN_TX: u8 = 0x01;
    pub const SIGN_MSG: u8 = 0x02;
    pub const PING: u8 = 0x03;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[num_enum(error_type(name = StatusWord, constructor = wrong_p1p2))]
#[repr(u8)]
pub enum GetPubKeyP1 {
    NoDisplayAddress = 0,
    DisplayAddress = 1,
}

impl GetPubKeyP1 {
    pub fn display(&self) -> bool {
        *self == Self::DisplayAddress
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[num_enum(error_type(name = StatusWord, constructor = wrong_p1p2))]
#[repr(u8)]
pub enum SignTxP1 {
    Start = 0,
    Next = 1,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[num_enum(error_type(name = StatusWord, constructor = wrong_p1p2))]
#[repr(u8)]
pub enum SignMsgP1 {
    Start = 0,
    Next = 1,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[num_enum(error_type(name = StatusWord, constructor = wrong_p1p2))]
#[repr(u8)]
pub enum PingP1 {
    // Ping doesn't have parameters, so its P1 must always be zero.
    Dummy = 0,
}

#[derive(Encode, Decode)]
pub struct GetPubKeyReq {
    pub coin_type: CoinType,
    pub path: Bip32Path,
}

#[derive(Encode, Decode)]
pub struct SignMessageStartReq {
    pub coin: CoinType,
    pub addr_type: AddrType,
    pub path: Bip32Path,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Encode, Decode)]
#[repr(u8)]
pub enum TransactionVersion {
    #[codec(index = 0)]
    V1,
}

#[derive(Encode, Decode)]
pub struct SignTxStartReq {
    pub coin: CoinType,
    pub version: TransactionVersion,
    pub num_inputs: u32,
    pub num_outputs: u32,
}

#[derive(Encode, Decode)]
pub enum SignTxNextReq {
    #[codec(index = 0)]
    ProcessInput(Box<TxInputData>),

    #[codec(index = 1)]
    ProcessInputCommitment(Box<TxInputCommitmentData>),

    #[codec(index = 2)]
    ProcessOutput(Box<TxOutputData>),

    #[codec(index = 3)]
    ReturnNextSignature,
}

// Note:
// 1) `addresses` can contain multiple entries in the case of multisig or no entries at all in
//    the case of a non-signable pseudo-input (at this moment FillOrder is the only possible
//    pseudo-input).
// 2) Derivation paths in `addresses` are not checked against the actual destinations that the
//    consensus requires the input to be signed against (note that in the non-utxo cases it's
//    not really possible to verify their consistency, because we don't commit to the actual
//    destination in those cases).
//    This is not a problem in general, but note that since we always use SigHashType::ALL in this
//    app, each input signature is a signature over the entire tx. So e.g. if inputs 0 and 1 require
//    keys A and B respectively, a malfunctioning host may request input 0 to be signed with key B
//    and input 1 with key A; in such a case the signature 0 will be valid for input 1 and vice versa.
//    This does not allow the host to change the reviewed transaction, but it means that the app must
//    not promise the user that a particular input was signed by the key specified in `addresses`.
#[derive(Encode, Decode)]
pub struct TxInputData {
    pub addresses: Vec<InputAddressPath>,
    pub input: TxInputWithAdditionalInfo,
}

#[derive(Encode, Decode)]
pub struct TxInputCommitmentData {
    pub commitment: SighashInputCommitment,
}

// TODO:
// 1) In order to be able to detect change outputs, there should be a way of specifying the destination
//    via a derivation path.
//    Note: the contents of Destination::PublicKeyHash and Destination::PublicKey should probably be
//    enums of the form `enum PublicKeyHash { Own(derivation path), Foreign(actual hash) }`.
// 2) Possible ways of handling change outputs (simplified version of what the cardano app seems to do):
//  * a) require that all derivation paths (in inputs and in outputs) belong to the same account,
//       fail if they don't;
//    b) if an output is a simple Transfer to a change address in the account, omit it from review;
//       if it's something more complicated, don't omit it from review, but mark is as change output.
//  * Same as above, but only track (without failing) whether all inputs are signed with keys from the
//    same account; if so and an output is a simple Transfer to a change address in that same account,
//    omit the output from review. If the output references a change address but multiple accounts
//    are referenced by inputs, or if the output is not a simple Transfer, then don't omit it,
//    but mark it as change.
// See https://github.com/mintlayer/mintlayer-ledger-app/issues/17.
#[derive(Encode, Decode)]
pub struct TxOutputData {
    pub output: TxOutput,
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
    #[codec(index = 0)]
    Utxo(TxOutput),

    #[codec(index = 1)]
    UtxoWithPoolData {
        utxo: TxOutput,
        staker_balance: Amount,
    },
}

impl From<AdditionalUtxoInfo> for SighashInputCommitment {
    fn from(value: AdditionalUtxoInfo) -> Self {
        match value {
            AdditionalUtxoInfo::Utxo(output) => SighashInputCommitment::Utxo(output),
            AdditionalUtxoInfo::UtxoWithPoolData {
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
pub struct UncompressedSecp256k1PublicKey(pub [u8; 65]);

#[derive(Encode, Decode)]
pub struct ChainCode(pub [u8; 32]);

#[derive(Encode, Decode)]
pub struct PublicKeyResponse {
    pub public_key: UncompressedSecp256k1PublicKey,
    pub chain_code: ChainCode,
}

#[derive(Encode, Decode)]
pub struct Signature(pub [u8; 64]);

#[derive(Encode, Decode)]
pub struct TxInputSignatureResponse {
    pub signature: Signature,
    pub input_idx: u32,
    pub multisig_idx: Option<u32>,
    pub has_next: bool,
}

#[derive(Encode, Decode)]
pub struct MsgSignatureResponse {
    pub signature: Signature,
}

#[derive(Encode, Decode)]
pub enum Response {
    #[codec(index = 0)]
    ExpectingNextChunk,
    #[codec(index = 1)]
    PublicKey(PublicKeyResponse),
    #[codec(index = 2)]
    TxSetup,
    #[codec(index = 3)]
    TxNext,
    #[codec(index = 4)]
    TxInputSignature(TxInputSignatureResponse),
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

pub fn encode_to<T, O>(t: T, output: &mut O)
where
    T: Encode,
    O: parity_scale_codec::Output,
{
    t.encode_to(output)
}

pub fn encode_as_compact<N>(num: N) -> Vec<u8>
where
    // Note: without the Num bound, if N is a reference, the compilation would fail with
    // "overflow evaluating the requirement `for<'b> CompactRef<'b, _>: Encode`".
    // With the bound, the error is much clearer.
    N: num_traits::Num,
    N: parity_scale_codec::HasCompact,
    <N as parity_scale_codec::HasCompact>::Type: Encode,
{
    <N as parity_scale_codec::HasCompact>::Type::from(num).encode()
}

pub fn encode_as_compact_to<N, O>(num: N, output: &mut O)
where
    // Same note as in encode_as_compact.
    N: num_traits::Num,
    N: parity_scale_codec::HasCompact,
    <N as parity_scale_codec::HasCompact>::Type: Encode,
    O: parity_scale_codec::Output,
{
    <N as parity_scale_codec::HasCompact>::Type::from(num).encode_to(output)
}

pub fn decode_all<T: Decode>(mut bytes: &[u8]) -> Option<T> {
    T::decode_all(&mut bytes).ok()
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
        (command_data.len() <= MAX_APDU_DATA_LEN).then_some(Self {
            instruction_byte,
            param1_byte,
            command_data,
            is_last_chunk: true,
        })
    }

    /// Returns an ExactSizeIterator of APDUs by chunking the data to MAX_APDU_DATA_LEN.
    pub fn new_chunks(
        instruction_byte: u8,
        param1_byte: u8,
        data: &'a [u8],
    ) -> impl ExactSizeIterator<Item = Self> {
        // Note: the standard `chunks` method returns zero-length iterator if the data has zero
        // length, but we need to return 1 APDU with zero-length data in such a case.
        let chunk_count = data.len().div_ceil(MAX_APDU_DATA_LEN).max(1);
        let chunk_iter = (0..chunk_count).map(|i| {
            let start = i * MAX_APDU_DATA_LEN;
            let end = (start + MAX_APDU_DATA_LEN).min(data.len());
            &data[start..end]
        });

        chunk_iter.enumerate().map(move |(chunk_idx, chunk)| Self {
            instruction_byte,
            param1_byte,
            command_data: chunk,
            is_last_chunk: chunk_idx == chunk_count - 1,
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
    #[display("Hashing failed")]
    HashFail = 0xB003,
    #[display("Address encoding failed")]
    AddressEncodingFail = 0xB004,
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
    #[display("Invalid data")]
    InvalidData = 0xB00D,
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
    #[display("Invalid timestamp")]
    InvalidTimestamp = 0xB014,
    #[display("Signature for FillOrder input requested")]
    FillOrderSigRequested = 0xB015,
    #[display("Transaction has zero inputs")]
    TxWithZeroInputs = 0xB016,

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
