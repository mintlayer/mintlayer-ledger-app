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

use crate::StatusWord;

use mintlayer_messages::mlcp::H256;

use ledger_device_sdk::hash::{blake2::Blake2b_512, HashInit};

pub fn mintlayer_hash(data: &[u8]) -> Result<H256, StatusWord> {
    let mut hasher = Blake2b_512::new();
    let mut message_hash: [u8; 64] = [0u8; 64];
    hasher
        .hash(data, &mut message_hash)
        .map_err(|_| StatusWord::TxHashFail)?;

    Ok(H256::from_slice(&message_hash[..32]))
}
