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

use mintlayer_messages::{parity_scale_codec, H256};

use ledger_device_sdk::hash::{blake2::Blake2b_512, HashInit};

/// The hasher that produces Mintlayer-specific hashes.
///
/// Note: we want to implement `parity_scale_codec::Output` for `Hasher`, which means that
/// its `update` method has to be infallible. But we don't want to `expect` on errors, so
/// on failure we just set `update_failed` to true, which will then cause `finalize` to fail.
pub struct Hasher {
    hasher: Blake2b_512,
    update_failed: bool,
}

impl Hasher {
    pub fn new() -> Self {
        Self {
            hasher: Blake2b_512::new(),
            update_failed: false,
        }
    }

    pub fn update(&mut self, input: &[u8]) {
        if self.update_failed {
            return;
        }

        if self.hasher.update(input).is_err() {
            self.update_failed = true;
        }
    }

    pub fn finalize(mut self) -> Result<H256, StatusWord> {
        if self.update_failed {
            return Err(StatusWord::HashFail);
        }

        let mut hash: [u8; 64] = [0u8; 64];
        self.hasher
            .finalize(&mut hash)
            .map_err(|_| StatusWord::HashFail)?;
        Ok(H256::from_slice(&hash[..32]))
    }

    pub fn hash(input: &[u8]) -> Result<H256, StatusWord> {
        let mut hasher = Self::new();
        hasher.update(input);
        hasher.finalize()
    }
}

impl parity_scale_codec::Output for Hasher {
    fn write(&mut self, bytes: &[u8]) {
        self.update(bytes)
    }
}
