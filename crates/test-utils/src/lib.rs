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

//! Every crate that needs unit tests that can be run on Speculos must have the following:
//! 1. It must be a lib crate.
//! 2. At the top of its `lib.rs` specify this:
//!    1. Disable the generation of `fn main`.
//!
//!       #![cfg_attr(test, no_main)]
//!
//!    2. "custom_test_frameworks" must be enabled to be able to specify the custom runner and use
//!       the `#[test_case]` attribute (used internally by `testmacro::test_item`).
//!
//!       #![feature(custom_test_frameworks)]
//!
//!    3. Specify the custom test runner. All test cases collected by `#[test_case]` will be passed
//!       to this function. In particular, `sdk_test_runner` will loop over the array of test cases
//!       and:
//!       a) fix references stored inside the test case via pic_rs/pic;
//!       b) invoke the closure associated with the test case.
//!
//!       #![test_runner(ledger_device_sdk::testing::sdk_test_runner)]
//!
//!
//!    4. The following will put `fn test_main` at the test crate's root, which will call the runner
//!       that we've specified above. The crate's `sample_main` will have to call `test_main`.
//!
//!       #![reexport_test_harness_main = "test_main"]
//! 3. Under `#[cfg(test)]` call:
//!    1. test_utils::impl_panic_handler!();
//!    2. test_utils::impl_main!();

#![no_std]

extern crate alloc;

use alloc::{borrow::ToOwned as _, format};

pub mod prelude {
    // testmacro::test_item expects `TestType` to be imported.
    pub use ledger_device_sdk::testing::TestType;

    pub use testmacro::test_item;
}

#[macro_export]
macro_rules! impl_panic_handler {
    () => {
        #[panic_handler]
        fn panic_handler(info: &core::panic::PanicInfo) -> ! {
            test_utils::handle_panic(info)
        }
    };
}

#[allow(clippy::crate_in_macro_def)]
#[macro_export]
macro_rules! impl_main {
    () => {
        #[unsafe(no_mangle)]
        extern "C" fn sample_main() {
            crate::test_main();
            ledger_device_sdk::exit_app(0);
        }
    };
}

pub fn handle_panic(info: &core::panic::PanicInfo) -> ! {
    ledger_device_sdk::error!(
        "Panic occurred at {}: {}",
        info.location().map_or_else(
            || "???".to_owned(),
            |loc| format!("{}:{}", loc.file(), loc.line())
        ),
        info.message(),
    );

    ledger_device_sdk::exit_app(1);
}
