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

#![no_std]
#![no_main]

mod utils;
mod app_ui {
    pub mod address;
    pub mod menu;
    pub mod sign;
}
mod handlers {
    pub mod get_public_key;
    pub mod get_version;
    pub mod sign_message;
    pub mod sign_tx;
}

mod settings;

use app_ui::menu::ui_menu_main;
use handlers::{
    get_public_key::handler_get_public_key,
    get_version::handler_get_version,
    sign_tx::{handler_sign_tx, TxContext},
};
use ledger_device_sdk::ecc::CxError;
use ledger_device_sdk::io::{ApduHeader, Comm, Reply, StatusWords};

ledger_device_sdk::set_panic!(ledger_device_sdk::exiting_panic);

// Required for using String, Vec, format!...
extern crate alloc;

use ledger_device_sdk::nbgl::{init_comm, NbglHomeAndSettings, NbglReviewStatus, StatusType};

use crate::handlers::sign_message::{handler_sign_message, SignMessageContext};

// P2 for last APDU to receive.
const P2_SIGN_TX_LAST: u8 = 0x00;
// P2 for more APDU to receive.
const P2_SIGN_TX_MORE: u8 = 0x80;
// P1 for first APDU number.
const P1_SIGN_TX_START: u8 = 0x00;
// P1 for maximum APDU number.
const P1_SIGN_TX_MAX: u8 = 0x04;

#[derive(Clone, Copy, PartialEq)]
pub enum P1SignTx {
    Metadata,
    Input,
    InputCommitement,
    Output,
    NextSignature,
}

impl TryFrom<u8> for P1SignTx {
    type Error = AppSW;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let x = match value {
            0 => Self::Metadata,
            1 => Self::Input,
            2 => Self::InputCommitement,
            3 => Self::Output,
            4 => Self::NextSignature,
            _ => return Err(AppSW::WrongP1P2)
        };

        Ok(x)
    }
}

// Application status words.
#[repr(u16)]
#[derive(Clone, Copy, PartialEq)]
pub enum AppSW {
    Deny = 0x6985,
    WrongP1P2 = 0x6A86,
    InsNotSupported = 0x6D00,
    ClaNotSupported = 0x6E00,
    TxDisplayFail = 0xB001,
    AddrDisplayFail = 0xB002,
    TxWrongLength = 0xB004,
    TxParsingFail = 0xB005,
    TxHashFail = 0xB006,
    TxAddressFail = 0xB007,
    TxSignFail = 0xB008,
    KeyDeriveFail = 0xB009,
    VersionParsingFail = 0xB00A,
    WrongContext = 0xB00B,
    TxDeserializeFail = 0xB00C,
    TxInvalidInputUtxo = 0xB00D,
    TxNumericOperationFail = 0xB00E,
    TxUnsupportedInput = 0xB00F,
    TxInvalidTokenV0 = 0xB010,

    WrongApduLength = StatusWords::BadLen as u16,
    Ok = 0x9000,

    Carry = 0xFF15,
    Locked,
    Unlocked,
    NotLocked,
    NotUnlocked,
    InternalError,
    InvalidParameterSize,
    InvalidParameterValue,
    InvalidParameter,
    NotInvertible,
    Overflow,
    MemoryFull,
    NoResidue,
    PointAtInfinity,
    InvalidPoint,
    InvalidCurve,
    GenericError,
}

impl From<CxError> for AppSW {
    fn from(value: CxError) -> Self {
        match value {
            CxError::Carry => Self::Carry,
            CxError::Locked => Self::Locked,
            CxError::Unlocked => Self::Unlocked,
            CxError::NotLocked => Self::NotLocked,
            CxError::NotUnlocked => Self::NotUnlocked,
            CxError::InternalError => Self::InternalError,
            CxError::InvalidParameterSize => Self::InvalidParameterSize,
            CxError::InvalidParameterValue => Self::InvalidParameterValue,
            CxError::InvalidParameter => Self::InvalidParameter,
            CxError::NotInvertible => Self::NotInvertible,
            CxError::Overflow => Self::Overflow,
            CxError::MemoryFull => Self::MemoryFull,
            CxError::NoResidue => Self::NoResidue,
            CxError::PointAtInfinity => Self::PointAtInfinity,
            CxError::InvalidPoint => Self::InvalidPoint,
            CxError::InvalidCurve => Self::InvalidCurve,
            CxError::GenericError => Self::GenericError,
            // A catch-all arm to handle any other variants CxError might have.
            // This makes the conversion robust.
            //_ => Self::GenericError,
        }
    }
}

impl From<AppSW> for Reply {
    fn from(sw: AppSW) -> Reply {
        Reply(sw as u16)
    }
}

/// Possible input commands received through APDUs.
pub enum Instruction {
    GetVersion,
    GetAppName,
    GetPubkey { display: bool },
    SignTx { p1: P1SignTx, more: u8 },
    SignMessage { chunk: u8, more: bool },
}

impl TryFrom<ApduHeader> for Instruction {
    type Error = AppSW;

    /// APDU parsing logic.
    ///
    /// Parses INS, P1 and P2 bytes to build an [`Instruction`]. P1 and P2 are translated to
    /// strongly typed variables depending on the APDU instruction code. Invalid INS, P1 or P2
    /// values result in errors with a status word, which are automatically sent to the host by the
    /// SDK.
    ///
    /// This design allows a clear separation of the APDU parsing logic and commands handling.
    ///
    /// Note that CLA is not checked here. Instead the method [`Comm::set_expected_cla`] is used in
    /// [`sample_main`] to have this verification automatically performed by the SDK.
    fn try_from(value: ApduHeader) -> Result<Self, Self::Error> {
        match (value.ins, value.p1, value.p2) {
            (3, 0, 0) => Ok(Instruction::GetVersion),
            (4, 0, 0) => Ok(Instruction::GetAppName),
            (5, 0 | 1, 0) => Ok(Instruction::GetPubkey {
                display: value.p1 != 0,
            }),
            (6, P1_SIGN_TX_START, P2_SIGN_TX_MORE)
            | (6, 1..=P1_SIGN_TX_MAX, 1 | 2 | P2_SIGN_TX_LAST | P2_SIGN_TX_MORE) => {
                Ok(Instruction::SignTx {
                    p1: value.p1.try_into()?,
                    more: value.p2,
                })
            }
            (7, P1_SIGN_TX_START, P2_SIGN_TX_MORE)
            | (7, 1..=P1_SIGN_TX_MAX, P2_SIGN_TX_LAST | P2_SIGN_TX_MORE) => {
                Ok(Instruction::SignMessage {
                    chunk: value.p1,
                    more: value.p2 == P2_SIGN_TX_MORE,
                })
            }
            (3..=7, _, _) => Err(AppSW::WrongP1P2),
            (_, _, _) => Err(AppSW::InsNotSupported),
        }
    }
}

fn show_status_and_home_if_needed(ins: &Instruction, ctx: &mut Context, status: &AppSW) {
    let (show_status, status_type) = match (ins, status) {
        (Instruction::GetPubkey { display: true }, AppSW::Deny | AppSW::Ok) => {
            (true, StatusType::Address)
        }
        (Instruction::SignTx { .. }, AppSW::Deny | AppSW::Ok) if ctx.finished() => {
            (true, StatusType::Transaction)
        }
        (Instruction::SignMessage { .. }, AppSW::Deny | AppSW::Ok) if ctx.finished() => {
            (true, StatusType::Message)
        }
        (_, _) => (false, StatusType::Transaction),
    };

    if show_status {
        let success = *status == AppSW::Ok;
        NbglReviewStatus::new()
            .status_type(status_type)
            .show(success);

        // call home.show_and_return() to show home and setting screen
        ctx.home.show_and_return();
    }
}

pub enum DataContext {
    Empty,
    TxContext(TxContext),
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
            DataContext::TxContext(ctx) => ctx.finished(),
        }
    }
}

#[no_mangle]
extern "C" fn sample_main() {
    // Create the communication manager, and configure it to accept only APDU from the 0xe0 class.
    // If any APDU with a wrong class value is received, comm will respond automatically with
    // BadCla status word.
    let mut comm = Comm::new().set_expected_cla(0xe0);

    let mut tx_ctx = Context::new();

    // Initialize reference to Comm instance for NBGL
    // API calls.
    init_comm(&mut comm);
    tx_ctx.home = ui_menu_main(&mut comm);
    tx_ctx.home.show_and_return();

    loop {
        let ins: Instruction = comm.next_command();

        let _status = match handle_apdu(&mut comm, &ins, &mut tx_ctx) {
            Ok(()) => {
                comm.reply_ok();
                AppSW::Ok
            }
            Err(sw) => {
                comm.reply(sw);
                sw
            }
        };
        show_status_and_home_if_needed(&ins, &mut tx_ctx, &_status);
    }
}

fn handle_apdu(comm: &mut Comm, ins: &Instruction, ctx: &mut Context) -> Result<(), AppSW> {
    match ins {
        Instruction::GetAppName => {
            comm.append(env!("CARGO_PKG_NAME").as_bytes());
            Ok(())
        }
        Instruction::GetVersion => handler_get_version(comm),
        Instruction::GetPubkey { display } => handler_get_public_key(comm, *display),
        Instruction::SignTx { p1, more } => handler_sign_tx(comm, *p1, *more, &mut ctx.data),
        Instruction::SignMessage { chunk, more } => {
            handler_sign_message(comm, *chunk, *more, &mut ctx.data)
        }
    }
}
