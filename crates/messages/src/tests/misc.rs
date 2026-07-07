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

use alloc::vec::Vec;

use test_utils::prelude::*;

use crate::Apdu;

#[test_item]
fn test_apdu_chunking() {
    let byte_iter = || core::iter::repeat(0..=9).flatten();

    let ins = 0xaa;
    let p1 = 0xbb;

    // One chunk
    {
        let data = byte_iter().take(100).collect::<Vec<_>>();
        let chunks = Apdu::new_chunks(ins, p1, &data).collect::<Vec<_>>();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].instruction_byte, ins);
        assert_eq!(chunks[0].param1_byte, p1);
        assert_eq!(chunks[0].command_data, &data);
        assert!(chunks[0].is_last_chunk);
    }

    // One chunk, data is zero length
    {
        let chunks = Apdu::new_chunks(ins, p1, &[]).collect::<Vec<_>>();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].instruction_byte, ins);
        assert_eq!(chunks[0].param1_byte, p1);
        assert_eq!(chunks[0].command_data, &[]);
        assert!(chunks[0].is_last_chunk);
    }

    // Multiple chunks
    {
        let data = byte_iter().take(600).collect::<Vec<_>>();
        let chunks = Apdu::new_chunks(ins, p1, &data).collect::<Vec<_>>();

        assert_eq!(chunks.len(), 3);

        assert_eq!(chunks[0].instruction_byte, ins);
        assert_eq!(chunks[0].param1_byte, p1);
        assert_eq!(chunks[0].command_data, &data[0..255]);
        assert!(!chunks[0].is_last_chunk);

        assert_eq!(chunks[1].instruction_byte, ins);
        assert_eq!(chunks[1].param1_byte, p1);
        assert_eq!(chunks[1].command_data, &data[255..510]);
        assert!(!chunks[1].is_last_chunk);

        assert_eq!(chunks[2].instruction_byte, ins);
        assert_eq!(chunks[2].param1_byte, p1);
        assert_eq!(chunks[2].command_data, &data[510..]);
        assert!(chunks[2].is_last_chunk);
    }
}
