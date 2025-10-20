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
    pub mod get_version;
    pub mod sign_message;
    pub mod sign_tx;
}

mod settings;

use ledger_device_sdk::{
    ecc::CxError,
    io::{ApduHeader, Comm, Reply, StatusWords},
    nbgl::{init_comm, NbglHomeAndSettings, NbglReviewStatus, StatusType},
};
use parity_scale_codec::DecodeAll;

use app_ui::menu::ui_menu_main;
use handlers::{
    get_public_key::handler_get_public_key,
    get_version::handler_get_version,
    sign_message::{handler_sign_message, setup_sign_message, SignMessageContext},
    sign_tx::{setup_sign_tx, Review, TxContext},
};
use messages::{
    Ins, P1SignTx, PubKeyP1, PublicKeyReq, SignMessageReq, SignTxReq, TxMetadataReq, WrongP1P2,
    APDU_CLASS, P1_APP_NAME, P1_GET_VERSION, P1_SIGN_MAX_CHUNKS, P1_SIGN_NEXT, P1_SIGN_START,
    P2_DONE, P2_SIGN_MORE,
};

use crate::handlers::sign_tx::handler_sign_tx;

ledger_device_sdk::set_panic!(ledger_device_sdk::exiting_panic);
// Required for using String, Vec, format!...
extern crate alloc;

impl From<WrongP1P2> for AppSW {
    fn from(_: WrongP1P2) -> Self {
        Self::WrongP1P2
    }
}

// Application status words.
#[repr(u16)]
#[derive(Clone, Copy, PartialEq)]
pub enum AppSW {
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

impl From<CxError> for AppSW {
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
    SignTx { p1: P1SignTx, more: bool },
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
            (Ins::GET_VERSION, P1_GET_VERSION, P2_DONE) => Ok(Instruction::GetVersion),
            (Ins::APP_NAME, P1_APP_NAME, P2_DONE) => Ok(Instruction::GetAppName),
            (Ins::PUB_KEY, p1, P2_DONE) => {
                let p1: PubKeyP1 = p1.try_into()?;
                Ok(Instruction::GetPubkey {
                    display: p1.display(),
                })
            }
            (Ins::SIGN_TX, p1, P2_SIGN_MORE | P2_DONE) => Ok(Instruction::SignTx {
                p1: p1.try_into()?,
                more: value.p2 == P2_SIGN_MORE,
            }),
            (Ins::SIGN_MSG, P1_SIGN_START, P2_DONE)
            | (Ins::SIGN_MSG, P1_SIGN_NEXT..=P1_SIGN_MAX_CHUNKS, P2_DONE | P2_SIGN_MORE) => {
                Ok(Instruction::SignMessage {
                    chunk: value.p1,
                    more: value.p2 == P2_SIGN_MORE,
                })
            }
            (
                Ins::GET_VERSION | Ins::APP_NAME | Ins::PUB_KEY | Ins::SIGN_TX | Ins::SIGN_MSG,
                _,
                _,
            ) => Err(AppSW::WrongP1P2),
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
    // Create the communication manager, and configure it to accept only APDU from the APDU_CLASS.
    // If any APDU with a wrong class value is received, comm will respond automatically with
    // BadCla status word.
    let mut comm = Comm::new().set_expected_cla(APDU_CLASS);

    let mut tx_ctx = Context::new();

    // Initialize reference to Comm instance for NBGL API calls.
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
    let mut data = comm.get_data().map_err(|_| AppSW::WrongApduLength)?;
    match ins {
        Instruction::GetAppName => {
            comm.append(env!("CARGO_PKG_NAME").as_bytes());
            Ok(())
        }
        Instruction::GetVersion => handler_get_version(comm),
        Instruction::GetPubkey { display } => {
            let req = PublicKeyReq::decode_all(&mut data).map_err(|_| AppSW::DeserializeFail)?;
            handler_get_public_key(comm, req, *display)
        }
        Instruction::SignTx { p1, more } => {
            if *p1 == P1SignTx::Metadata {
                let req =
                    TxMetadataReq::decode_all(&mut data).map_err(|_| AppSW::DeserializeFail)?;
                setup_sign_tx(req, &mut ctx.data)
            } else {
                let mut data = comm.get_data().map_err(|_| AppSW::WrongApduLength)?;

                let (ctx, review) = match &mut ctx.data {
                    DataContext::TxContext(ctx, review) => (ctx, review),
                    _ => return Err(AppSW::WrongContext),
                };

                ctx.show_spinner();
                ctx.extend(data)?;

                if *more {
                    return Ok(());
                }

                let req = SignTxReq::decode_all(&mut data).map_err(|_| AppSW::DeserializeFail)?;
                handler_sign_tx(comm, req, ctx, review)
            }
        }
        Instruction::SignMessage { chunk, more } => {
            if *chunk == 0 {
                let req =
                    SignMessageReq::decode_all(&mut data).map_err(|_| AppSW::DeserializeFail)?;
                setup_sign_message(req, &mut ctx.data)
            } else {
                handler_sign_message(comm, *more, &mut ctx.data)
            }
        }
    }
}
