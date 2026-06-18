/*****************************************************************************
 *
 *   Mintlayer Ledger App.
 *   (c) 2023 Ledger SAS.
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
// The following is needed to be able to generate a test executable that can be run on speculos.
// 1. Disable the generation of `fn main`.
#![cfg_attr(test, no_main)]
// 2. "custom_test_frameworks" must be enabled to be able to specify the custom runner and use
// the `#[test_case]` attribute (used internally by `testmacro::test_item`).
#![feature(custom_test_frameworks)]
// 3. Specify the custom test runner. All test cases collected by `#[test_case]` will be passed
// to this function. In particular, `sdk_test_runner` will loop over the array of test cases and:
// a) fix references stored inside the test case via pic_rs/pic;
// b) invoke the closure associated with the test case.
#![test_runner(ledger_device_sdk::testing::sdk_test_runner)]
// 4. This will put `fn test_main` at the test crate's root, which will call the runner that we've
// specified above; we'll call it from our `sample_main`.
#![reexport_test_harness_main = "test_main"]

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
    pub mod utils;
}

mod errors;
#[cfg(test)]
mod testing;

// Required for using String, Vec, format!...
extern crate alloc;
use alloc::vec::Vec;

use ledger_device_sdk::{
    io::{ApduHeader, Comm, Reply},
    nbgl::{init_comm, NbglHomeAndSettings, NbglReviewStatus, NbglStreamingReview, StatusType},
};

use app_ui::menu::ui_menu_main;
use errors::sdk_err_to_status;
use handlers::{
    get_public_key::handle_get_public_key,
    sign_message::{handle_sign_message, setup_sign_message, SignMessageContext},
    sign_tx::{handle_sign_tx, setup_sign_tx, TxParsingContext},
};
use mintlayer_messages::{
    decode_all, encode, GetPubKeyP1, Ins, PingP1, Response, SignMsgP1, SignTxP1, StatusWord,
    APDU_CLASS, MAX_ADPU_DATA_LEN, P2_DONE, P2_MORE,
};

pub const MAX_BUFFER_LEN: usize = 4 * MAX_ADPU_DATA_LEN;

/// Represents a fully assembled Low-Level Instruction.
/// Contains the aggregated data from one or more APDUs (if P2 indicated more data).
pub struct RawInstruction {
    pub ins: u8,
    pub p1: u8,
    pub data: Vec<u8>,
}

pub enum ReceiveInstructionResult {
    ExpectingNextChunk,
    Instruction(RawInstruction),
}

/// State machine to handle APDU packet chaining (P2_MORE / P2_DONE).
pub struct ApduTransport {
    buffer: Vec<u8>,
    current_ins: Option<u8>,
    current_p1: Option<u8>,
}

impl Default for ApduTransport {
    fn default() -> Self {
        Self {
            buffer: Vec::with_capacity(u8::MAX as usize), // Pre-alloc for at least one standard APDU
            current_ins: None,
            current_p1: None,
        }
    }
}

impl ApduTransport {
    /// Reads the next APDU from `comm`.
    ///
    /// - If `P2 == P2_MORE`, it accumulates the data and returns `Ok(None)`.
    ///   It also sends a `StatusWord::Ok` to the host to request the next chunk.
    /// - If `P2 == P2_DONE`, it finishes accumulation and returns `Ok(Some(RawInstruction))`.
    pub fn receive(&mut self, comm: &mut Comm) -> Result<ReceiveInstructionResult, StatusWord> {
        let header: ApduHeader = comm.next_command();
        let data = comm.get_data().map_err(sdk_err_to_status)?;

        // Validation: If we are in the middle of a stream, INS and P1 must match
        if let (Some(curr_ins), Some(curr_p1)) = (self.current_ins, self.current_p1) {
            if header.ins != curr_ins {
                self.reset();
                return Err(StatusWord::WrongInstruction);
            }
            if header.p1 != curr_p1 {
                self.reset();
                return Err(StatusWord::WrongP1P2);
            }
        } else {
            // New command sequence starting
            self.current_ins = Some(header.ins);
            self.current_p1 = Some(header.p1);
        }

        if self.buffer.len() + data.len() > MAX_BUFFER_LEN {
            self.reset();
            return Err(StatusWord::MaxBufferLenExceeded);
        }

        self.buffer.extend_from_slice(data);

        match header.p2 {
            P2_MORE => Ok(ReceiveInstructionResult::ExpectingNextChunk),
            P2_DONE => {
                // Construct the full instruction
                let raw = RawInstruction {
                    ins: header.ins,
                    p1: header.p1,
                    data: core::mem::take(&mut self.buffer),
                };
                self.reset();
                Ok(ReceiveInstructionResult::Instruction(raw))
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
    GetPubKey { p1: GetPubKeyP1, data: Vec<u8> },
    SignTx { p1: SignTxP1, data: Vec<u8> },
    SignMessage { p1: SignMsgP1, data: Vec<u8> },
    Ping,
}

impl TryFrom<RawInstruction> for Command {
    type Error = StatusWord;

    fn try_from(raw: RawInstruction) -> Result<Self, Self::Error> {
        match raw.ins {
            Ins::GET_PUB_KEY => {
                let p1: GetPubKeyP1 = raw.p1.try_into()?;
                Ok(Command::GetPubKey { p1, data: raw.data })
            }
            Ins::SIGN_TX => {
                let p1: SignTxP1 = raw.p1.try_into()?;
                Ok(Command::SignTx { p1, data: raw.data })
            }
            Ins::SIGN_MSG => {
                let p1: SignMsgP1 = raw.p1.try_into()?;
                Ok(Command::SignMessage { p1, data: raw.data })
            }
            Ins::PING => {
                let _p1: PingP1 = raw.p1.try_into()?;
                Ok(Command::Ping)
            }
            _ => Err(StatusWord::InsNotSupported),
        }
    }
}

fn show_status_and_home_if_needed(cmd: &Command, ctx: &mut AppContext, status: StatusWord) {
    let (show_status, status_type) = match (cmd, status) {
        (Command::GetPubKey { p1, .. }, StatusWord::Deny | StatusWord::Ok) if p1.display() => {
            (true, StatusType::Address)
        }
        (Command::SignTx { .. }, StatusWord::Deny | StatusWord::Ok) if ctx.finished() => {
            (true, StatusType::Transaction)
        }
        (Command::SignMessage { .. }, StatusWord::Deny | StatusWord::Ok) if ctx.finished() => {
            (true, StatusType::Message)
        }
        (Command::Ping, StatusWord::Ok) => {
            ctx.home.show_and_return();
            return;
        }
        (_, _) => (false, StatusType::Transaction), // Default fallback
    };

    if show_status {
        let success = status == StatusWord::Ok;
        NbglReviewStatus::new()
            .status_type(status_type)
            .show(success);

        // call home.show_and_return() to show home and setting screen
        ctx.home.show_and_return();
    }
}

pub enum DataContext {
    TxContext(TxParsingContext, NbglStreamingReview),
    SignMessageContext(SignMessageContext),
}

struct AppContext {
    pub data_context: Option<DataContext>,
    pub home: NbglHomeAndSettings,
}

impl AppContext {
    fn new() -> Self {
        Self {
            data_context: None,
            home: Default::default(),
        }
    }

    fn finished(&self) -> bool {
        self.data_context.as_ref().is_some_and(|ctx| match ctx {
            DataContext::SignMessageContext(ctx) => ctx.finished(),
            DataContext::TxContext(ctx, _) => ctx.finished(),
        })
    }
}

pub fn mintlayer_main() {
    let mut comm = Comm::new().set_expected_cla(APDU_CLASS);

    let mut tx_ctx = AppContext::new();

    // Initialize reference to Comm instance for NBGL API calls.
    init_comm(&mut comm);
    tx_ctx.home = ui_menu_main();
    tx_ctx.home.show_and_return();

    let mut transport = ApduTransport::default();

    loop {
        let raw_instruction = match transport.receive(&mut comm) {
            Ok(ReceiveInstructionResult::Instruction(raw)) => raw,
            Ok(ReceiveInstructionResult::ExpectingNextChunk) => {
                // Signal host that we received the chunk and are waiting for more
                comm.append(&encode(Response::ExpectingNextChunk));
                comm.reply(Reply(StatusWord::Ok as u16));
                continue; // Waiting for more chunks, loop around
            }
            Err(sw) => {
                comm.reply(Reply(sw as u16));
                continue;
            }
        };

        let command = match Command::try_from(raw_instruction) {
            Ok(cmd) => cmd,
            Err(sw) => {
                comm.reply(Reply(sw as u16));
                continue;
            }
        };

        let status = match handle_command(&command, &mut tx_ctx) {
            Ok(response) => {
                comm.append(&encode(response));
                comm.reply_ok();
                StatusWord::Ok
            }
            Err(sw) => {
                comm.reply(Reply(sw as u16));
                sw
            }
        };

        show_status_and_home_if_needed(&command, &mut tx_ctx, status);
    }
}

fn handle_command(cmd: &Command, ctx: &mut AppContext) -> Result<Response, StatusWord> {
    match cmd {
        Command::GetPubKey { p1, data } => {
            let req = decode_all(data).ok_or(StatusWord::DeserializeFail)?;
            handle_get_public_key(req, p1.display()).map(Response::PublicKey)
        }
        Command::SignTx { p1, data } => match p1 {
            SignTxP1::Start => {
                let req = decode_all(data).ok_or(StatusWord::DeserializeFail)?;
                ctx.data_context = Some(setup_sign_tx(req)?);
                Ok(Response::TxSetup)
            }
            SignTxP1::Next => {
                let (mut tx_ctx, mut review) = match ctx.data_context.take() {
                    Some(DataContext::TxContext(c, r)) => (c, r),
                    _ => return Err(StatusWord::WrongContext),
                };

                let req = decode_all(data).ok_or(StatusWord::DeserializeFail)?;

                tx_ctx.show_spinner();

                match handle_sign_tx(req, tx_ctx, &mut review) {
                    Ok((response, new_ctx)) => {
                        ctx.data_context = Some(DataContext::TxContext(new_ctx, review));
                        Ok(response)
                    }
                    Err(sw) => {
                        ctx.data_context = None;
                        Err(sw)
                    }
                }
            }
        },
        Command::SignMessage { p1, data } => match p1 {
            SignMsgP1::Start => {
                let req = decode_all(data).ok_or(StatusWord::DeserializeFail)?;
                ctx.data_context = Some(setup_sign_message(req));
                Ok(Response::MessageSetup)
            }
            SignMsgP1::Next => {
                let msg_ctx = match ctx.data_context.as_mut() {
                    Some(DataContext::SignMessageContext(ctx)) => ctx,
                    _ => return Err(StatusWord::WrongContext),
                };
                handle_sign_message(data, msg_ctx).map(Response::MessageSignature)
            }
        },
        Command::Ping => Ok(Response::Pong),
    }
}
