/*****************************************************************************
 *   Mintlayer Ledger App.
 *   (c) 2023 Ledger SAS.
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
#![no_main]

mod app_ui {
    pub mod address;
    pub mod menu;
    pub mod sign;
    pub mod utils;
}
mod handlers {
    pub mod get_public_key;
    pub mod sign_message;
    pub mod sign_tx;
}

mod settings;

// Required for using String, Vec, format!...
extern crate alloc;
use alloc::vec::Vec;

use ledger_device_sdk::{
    ecc::CxError,
    io::{ApduHeader, Comm, Reply, StatusWords},
    nbgl::{init_comm, NbglHomeAndSettings, NbglReviewStatus, StatusType},
};

use app_ui::menu::ui_menu_main;
use handlers::{
    get_public_key::handle_get_public_key,
    sign_message::{handle_sign_message, setup_sign_message, SignMessageContext},
    sign_tx::{setup_sign_tx, Review, TxContext},
};
use messages::{
    decode_all, Ins, P1SignTx, PubKeyP1, WrongP1P2, APDU_CLASS, MAX_ADPU_DATA_LEN, P1_SIGN_START,
    P2_DONE, P2_MORE,
};

use crate::handlers::sign_tx::handle_sign_tx;

ledger_device_sdk::set_panic!(ledger_device_sdk::exiting_panic);

pub const MAX_BUFFER_LEN: usize = 4 * MAX_ADPU_DATA_LEN;

impl From<WrongP1P2> for StatusWord {
    fn from(_: WrongP1P2) -> Self {
        Self::WrongP1P2
    }
}

// Application status words.
#[repr(u16)]
#[derive(Clone, Copy, PartialEq)]
pub enum StatusWord {
    Ok = StatusWords::Ok as u16,
    Deny = StatusWords::UserCancelled as u16,
    ClaNotSupported = StatusWords::BadCla as u16,
    WrongP1P2 = StatusWords::BadP1P2 as u16,
    InsNotSupported = StatusWords::BadIns as u16,
    WrongApduLength = StatusWords::BadLen as u16,

    TxDisplayFail = 0xB000,
    AddrDisplayFail = 0xB001,
    TxWrongLength = 0xB002,
    TxParsingFail = 0xB003,
    TxHashFail = 0xB004,
    TxAddressFail = 0xB005,
    TxSignFail = 0xB006,
    KeyDeriveFail = 0xB007,
    VersionParsingFail = 0xB008,
    WrongContext = 0xB009,
    DeserializeFail = 0xB00A,
    TxInvalidInputUtxo = 0xB00B,
    TxNumericOperationFail = 0xB00C,
    TxUnsupportedInput = 0xB00D,
    TxInvalidTokenV0 = 0xB00E,
    TxInvalidInputPath = 0xB00F,
    NothingToSign = 0xB010,
    TxAlreadyFinished = 0xB011,
    InvalidPath = 0xB012,
    InvalidUncompressedPublicKey = 0xB013,
    MaxBufferLenExceeded = 0xB014,

    // The following errors come from ecc::CxError
    EccCarry = 0xB100,
    EccLocked = 0xB101,
    EccUnlocked = 0xB102,
    EccNotLocked = 0xB103,
    EccNotUnlocked = 0xB104,
    EccInternalError = 0xB105,
    EccInvalidParameterSize = 0xB106,
    EccInvalidParameterValue = 0xB107,
    EccInvalidParameter = 0xB108,
    EccNotInvertible = 0xB109,
    EccOverflow = 0xB10A,
    EccMemoryFull = 0xB10B,
    EccNoResidue = 0xB10C,
    EccPointAtInfinity = 0xB10D,
    EccInvalidPoint = 0xB10E,
    EccInvalidCurve = 0xB10F,
    EccGenericError = 0xB110,
}

impl From<CxError> for StatusWord {
    fn from(value: CxError) -> Self {
        match value {
            CxError::Carry => Self::EccCarry,
            CxError::Locked => Self::EccLocked,
            CxError::Unlocked => Self::EccUnlocked,
            CxError::NotLocked => Self::EccNotLocked,
            CxError::NotUnlocked => Self::EccNotUnlocked,
            CxError::InternalError => Self::EccInternalError,
            CxError::InvalidParameterSize => Self::EccInvalidParameterSize,
            CxError::InvalidParameterValue => Self::EccInvalidParameterValue,
            CxError::InvalidParameter => Self::EccInvalidParameter,
            CxError::NotInvertible => Self::EccNotInvertible,
            CxError::Overflow => Self::EccOverflow,
            CxError::MemoryFull => Self::EccMemoryFull,
            CxError::NoResidue => Self::EccNoResidue,
            CxError::PointAtInfinity => Self::EccPointAtInfinity,
            CxError::InvalidPoint => Self::EccInvalidPoint,
            CxError::InvalidCurve => Self::EccInvalidCurve,
            CxError::GenericError => Self::EccGenericError,
        }
    }
}

impl From<StatusWord> for Reply {
    fn from(sw: StatusWord) -> Reply {
        Reply(sw as u16)
    }
}

/// Represents a fully assembled Low-Level Instruction.
/// Contains the aggregated data from one or more APDUs (if P2 indicated more data).
pub struct RawInstruction {
    pub ins: u8,
    pub p1: u8,
    pub data: Vec<u8>,
}

/// State machine to handle APDU packet chaining (P2_MORE / P2_DONE).
pub struct ApduTransport {
    buffer: Vec<u8>,
    current_ins: Option<u8>,
    current_p1: Option<u8>,
}

impl ApduTransport {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(255), // Pre-alloc for at least one standard APDU
            current_ins: None,
            current_p1: None,
        }
    }

    /// Reads the next APDU from `comm`.
    ///
    /// - If `P2 == P2_MORE`, it accumulates the data and returns `Ok(None)`.
    ///   It also sends a `StatusWord::Ok` to the host to request the next chunk.
    /// - If `P2 == P2_DONE`, it finishes accumulation and returns `Ok(Some(RawInstruction))`.
    pub fn receive(&mut self, comm: &mut Comm) -> Result<Option<RawInstruction>, StatusWord> {
        let header: ApduHeader = comm.next_command();
        let data = comm.get_data().map_err(|_| StatusWord::WrongApduLength)?;

        // Validation: If we are in the middle of a stream, INS and P1 must match
        if let (Some(curr_ins), Some(curr_p1)) = (self.current_ins, self.current_p1) {
            if header.ins != curr_ins || header.p1 != curr_p1 {
                self.reset();
                return Err(StatusWord::WrongP1P2);
            }
        } else {
            // New command sequence starting
            self.current_ins = Some(header.ins);
            self.current_p1 = Some(header.p1);
        }

        if self.buffer.len() + data.len() > MAX_BUFFER_LEN {
            return Err(StatusWord::MaxBufferLenExceeded);
        }

        self.buffer.extend_from_slice(data);

        match header.p2 {
            P2_MORE => {
                // Signal host that we received the chunk and are waiting for more
                comm.reply(StatusWord::Ok);
                Ok(None)
            }
            P2_DONE => {
                // Construct the full instruction
                let raw = RawInstruction {
                    ins: header.ins,
                    p1: header.p1,
                    data: core::mem::take(&mut self.buffer),
                };
                self.reset();
                Ok(Some(raw))
            }
            _ => {
                self.reset();
                Err(StatusWord::WrongP1P2)
            }
        }
    }

    fn reset(&mut self) {
        self.buffer.clear();
        self.current_ins = None;
        self.current_p1 = None;
    }
}

pub enum Command {
    GetPubkey { display: bool, data: Vec<u8> },
    SignTx { p1: P1SignTx, data: Vec<u8> },
    SignMessage { p1: u8, data: Vec<u8> },
}

impl TryFrom<RawInstruction> for Command {
    type Error = StatusWord;

    fn try_from(raw: RawInstruction) -> Result<Self, Self::Error> {
        match raw.ins {
            Ins::PUB_KEY => {
                let p1: PubKeyP1 = raw.p1.try_into()?;
                Ok(Command::GetPubkey {
                    display: p1.display(),
                    data: raw.data,
                })
            }
            Ins::SIGN_TX => {
                let p1: P1SignTx = raw.p1.try_into()?;
                Ok(Command::SignTx { p1, data: raw.data })
            }
            Ins::SIGN_MSG => {
                let p1 = raw.p1;
                Ok(Command::SignMessage { p1, data: raw.data })
            }
            _ => Err(StatusWord::InsNotSupported),
        }
    }
}

fn show_status_and_home_if_needed(cmd: &Command, ctx: &mut Context, status: &StatusWord) {
    let (show_status, status_type) = match (cmd, status) {
        (Command::GetPubkey { display: true, .. }, StatusWord::Deny | StatusWord::Ok) => {
            (true, StatusType::Address)
        }
        (Command::SignTx { .. }, StatusWord::Deny | StatusWord::Ok) if ctx.finished() => {
            (true, StatusType::Transaction)
        }
        (Command::SignMessage { .. }, StatusWord::Deny | StatusWord::Ok) if ctx.finished() => {
            (true, StatusType::Message)
        }
        (_, _) => (false, StatusType::Transaction), // Default fallback
    };

    if show_status {
        let success = *status == StatusWord::Ok;
        NbglReviewStatus::new()
            .status_type(status_type)
            .show(success);

        // call home.show_and_return() to show home and setting screen
        ctx.home.show_and_return();
    }
}

pub enum DataContext {
    Empty,
    TxContext(TxContext, Review),
    SignMessageContext(SignMessageContext),
}

struct Context {
    pub data: DataContext,
    pub home: NbglHomeAndSettings,
}

impl Context {
    fn new() -> Self {
        Self {
            data: DataContext::Empty,
            home: Default::default(),
        }
    }

    fn finished(&self) -> bool {
        match &self.data {
            DataContext::Empty => false,
            DataContext::SignMessageContext(ctx) => ctx.finished(),
            DataContext::TxContext(ctx, _) => ctx.finished(),
        }
    }
}

#[no_mangle]
extern "C" fn sample_main() {
    let mut comm = Comm::new().set_expected_cla(APDU_CLASS);

    let mut tx_ctx = Context::new();

    // Initialize reference to Comm instance for NBGL API calls.
    init_comm(&mut comm);
    tx_ctx.home = ui_menu_main(&mut comm);
    tx_ctx.home.show_and_return();

    let mut transport = ApduTransport::new();

    loop {
        let raw_instruction = match transport.receive(&mut comm) {
            Ok(Some(raw)) => raw,
            Ok(None) => continue, // Waiting for more chunks, loop around
            Err(sw) => {
                comm.reply(sw);
                continue;
            }
        };

        let command = match Command::try_from(raw_instruction) {
            Ok(cmd) => cmd,
            Err(sw) => {
                comm.reply(sw);
                continue;
            }
        };

        let status = match handle_command(&mut comm, &command, &mut tx_ctx) {
            Ok(()) => {
                comm.reply_ok();
                StatusWord::Ok
            }
            Err(sw) => {
                comm.reply(sw);
                sw
            }
        };

        show_status_and_home_if_needed(&command, &mut tx_ctx, &status);
    }
}

fn handle_command(comm: &mut Comm, cmd: &Command, ctx: &mut Context) -> Result<(), StatusWord> {
    match cmd {
        Command::GetPubkey { display, data } => {
            let req = decode_all(data).ok_or(StatusWord::DeserializeFail)?;
            handle_get_public_key(comm, req, *display)
        }
        Command::SignTx { p1, data } => {
            if *p1 == P1SignTx::Metadata {
                let req = decode_all(data).ok_or(StatusWord::DeserializeFail)?;
                setup_sign_tx(req, &mut ctx.data)
            } else {
                let (tx_ctx, review) = match &mut ctx.data {
                    DataContext::TxContext(c, r) => (c, r),
                    _ => return Err(StatusWord::WrongContext),
                };

                tx_ctx.show_spinner();

                let req = decode_all(data).ok_or(StatusWord::DeserializeFail)?;
                handle_sign_tx(comm, req, tx_ctx, review)
            }
        }
        Command::SignMessage { p1, data } => {
            if *p1 == P1_SIGN_START {
                let req = decode_all(&data).ok_or(StatusWord::DeserializeFail)?;
                setup_sign_message(req, &mut ctx.data)
            } else {
                handle_sign_message(comm, false, &mut ctx.data)
            }
        }
    }
}
