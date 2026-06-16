/*****************************************************************************
 *
 *   Mintlayer Ledger App.
 *   (c) 2023 Ledger SAS.
 *   (c) 2026 RBB S.r.l.
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

use alloc::{borrow::ToOwned as _, format};

pub mod prelude {
    // testmacro::test_item expects `TestType` to be imported.
    pub use ledger_device_sdk::testing::TestType;

    pub use testmacro::test_item;
}

#[no_mangle]
extern "C" fn sample_main() {
    crate::test_main();
    ledger_device_sdk::exit_app(0);
}

#[panic_handler]
fn handle_panic(info: &core::panic::PanicInfo) -> ! {
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
