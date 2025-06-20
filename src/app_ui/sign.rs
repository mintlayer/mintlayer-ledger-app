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
use crate::handlers::sign_tx::{Tx, TxContext};
use crate::AppSW;

use crate::settings::Settings;
use include_gif::include_gif;
use ledger_device_sdk::hash::{sha2::Sha2_256, HashInit, blake2::Blake2b_512};
use ledger_device_sdk::nbgl::{Field, NbglGlyph, NbglReview};

use alloc::{format, string::ToString};
use alloc::vec::Vec;

/// Displays a transaction and returns true if user approved it.
///
/// This method can return [`AppSW::TxDisplayFail`] error if the coin name length is too long.
///
/// # Arguments
///
/// * `tx` - Transaction to be displayed for validation
pub fn ui_display_tx(tx: &TxContext) -> Result<bool, AppSW> {
    //let value_str = format!("{} {}", "coin", "123");
    //let to_str = format!("0x{}", hex::encode(b"asd").to_uppercase());

    // Define transaction review fields
    let my_fields = [
        /*Field {
            name: "Amount",
            value: value_str.as_str(),
        },
        Field {
            name: "Destination",
            value: to_str.as_str(),
        },*/
        Field {
            name: "Memo",
            value: "momo",
        },
    ];

    // Create transaction review

    // Load glyph from 64x64 4bpp gif file with include_gif macro. Creates an NBGL compatible glyph.
    //#[cfg(any(target_os = "stax", target_os = "flex"))]
    //const FERRIS: NbglGlyph = NbglGlyph::from_include(include_gif!("crab_64x64.gif", NBGL));
    //#[cfg(any(target_os = "nanosplus", target_os = "nanox"))]
    //const FERRIS: NbglGlyph = NbglGlyph::from_include(include_gif!("crab_16x16.gif", NBGL));

    // Create NBGL review. Maximum number of fields and string buffer length can be customised
    // with constant generic parameters of NbglReview. Default values are 32 and 1024 respectively.
    let review: NbglReview = NbglReview::new()
        .titles(
            "Review transaction\nto send CRAB",
            "",
            "Sign transaction\nto send CRAB",
        );
        //.glyph(&FERRIS);

    // If first setting switch is disabled do not display the transaction memo
    //let settings: Settings = Default::default();
    //if settings.get_element(0) == 0 {
    //    Ok(review.show(&my_fields[0..2]))
    //} else {
        Ok(review.show(&my_fields))
    //}
}

/// Displays a message for review and signing confirmation on the device.
///
/// This function handles both printable text and raw binary data by
/// displaying UTF-8 content directly and falling back to hex encoding for other data.
/// It also shows a SHA-256 hash of the message for verification.
///
/// # Arguments
///
/// * `message` - A byte slice (`&[u8]`) containing the message to be signed.
///
/// # Returns
///
/// * `Ok(true)` if the user approves the signing.
/// * `Ok(false)` if the user rejects.
/// * `Err(AppSW)` on error.
pub fn ui_display_message(message: &[u8]) -> Result<bool, AppSW> {
    // Attempt to display the message as a UTF-8 string. If it's not valid
    // UTF-8, display it as a hex string. This requires creating an owned
    // String to ensure the borrow for `Field` is valid.
    let message_str = match core::str::from_utf8(message) {
        Ok(s) => s.to_string(),
        Err(_) => format!("0x{}", hex::encode(message)),
    };

    const MESSAGE_MAGIC_PREFIX: &str = "===MINTLAYER MESSAGE BEGIN===\n";
    const MESSAGE_MAGIC_SUFFIX: &str = "\n===MINTLAYER MESSAGE END===";

    let mut blake2b256 = Blake2b_512::new();
    let mut message_hash: [u8; 64] = [0u8; 64];
    let mut message_hash2: [u8; 64] = [0u8; 64];

    let message = MESSAGE_MAGIC_PREFIX
        .as_bytes()
        .iter()
        .chain(message.iter())
        .chain(MESSAGE_MAGIC_SUFFIX.as_bytes().iter())
        .copied()
        .collect::<Vec<_>>();

        // TODO: handle hash error
    blake2b256.hash(&message, &mut message_hash).map_err(|_| AppSW::Foo1)?;
    let mut message_hash_32: [u8; 32] = [0u8; 32];
    message_hash_32.copy_from_slice(&message_hash[0..32]);


    let hash_str = format!("0x{}", hex::encode(message_hash_32));

    let mut blake2b256 = Blake2b_512::new();
    blake2b256.hash(&message_hash_32, &mut message_hash2).map_err(|_| AppSW::Foo1)?;

    // It's a good practice to show the hash of the message, so the user can
    // verify it on a trusted screen if the message is too long to be
    // displayed entirely on the Ledger device.
    //let mut sha2 = Sha2_256::new();
    //let mut hash_output: [u8; 32] = [0; 32];
    //sha2.hash(message, &mut hash_output)
    //    .map_err(|_| AppSW::Foo1)?;

    let mut message_hash2_32: [u8; 32] = [0u8; 32];
    message_hash2_32.copy_from_slice(&message_hash2[0..32]);
    let hash_str2 = format!("0x{}", hex::encode(message_hash2_32));

    let hex_message = hex::encode(message);
    // Define the fields for the review screen.
    let my_fields = [
        Field {
            name: "Message",
            value: message_str.as_str(),
        },
        Field {
            name: "Message hex",
            value: hex_message.as_str(),
        },
        Field {
            name: "Message Hash1",
            value: hash_str.as_str(),
        },
        Field {
            name: "Message Hash2",
            value: hash_str2.as_str(),
        },
    ];

    // Load a generic icon for signing. You should replace this with your app's icon.
    // The `include_gif!` macro selects the correct size based on the target device.
    #[cfg(any(target_os = "stax", target_os = "flex"))]
    const SIGN_ICON: NbglGlyph = NbglGlyph::from_include(include_gif!("crab_64x64.gif", NBGL));
    #[cfg(any(target_os = "nanosplus", target_os = "nanox"))]
    const SIGN_ICON: NbglGlyph = NbglGlyph::from_include(include_gif!("crab_16x16.gif", NBGL));

    // Create the NBGL review flow with titles appropriate for message signing.
    let review: NbglReview = NbglReview::new()
        .titles(
            "Review message",   // Initial title
            "Cannot be undone", // Warning on the second screen
            "Sign message",     // Final confirmation prompt
        )
        .glyph(&SIGN_ICON);

    // Show the review screen with the defined fields and return the user's choice.
    Ok(review.show(&my_fields))
}
