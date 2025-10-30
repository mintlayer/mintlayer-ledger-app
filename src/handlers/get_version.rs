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

use crate::StatusWord;
use core::str::FromStr;
use ledger_device_sdk::io;

use messages::{encode, GetVersionRespones};

pub fn handle_get_version(comm: &mut io::Comm) -> Result<(), StatusWord> {
    if let Some((major, minor, patch)) = parse_version_string(env!("CARGO_PKG_VERSION")) {
        let response = GetVersionRespones {
            major,
            minor,
            patch,
            prerelease_id: None,
            build_metadata: env!("GIT_HASH").as_bytes().to_vec(),
        };

        comm.append(&encode(response));
        Ok(())
    } else {
        Err(StatusWord::VersionParsingFail)
    }
}

fn parse_version_string(input: &str) -> Option<(u8, u8, u8)> {
    // Split the input string by '.'.
    // Input should be of the form "major.minor.patch",
    // where "major", "minor", and "patch" are integers.
    let mut parts = input.split('.');
    let major = u8::from_str(parts.next()?).ok()?;
    let minor = u8::from_str(parts.next()?).ok()?;
    let patch = u8::from_str(parts.next()?).ok()?;
    Some((major, minor, patch))
}
