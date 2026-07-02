/*****************************************************************************
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

use ledger_device_sdk::nbgl::{NbglGlyph, NbglHomeAndSettings};

use crate::app_ui::utils::load_glyph;

pub fn ui_menu_main() -> NbglHomeAndSettings {
    const MINTLAYER: NbglGlyph = load_glyph();

    // Display the home screen.
    NbglHomeAndSettings::new().glyph(&MINTLAYER).infos(
        "Mintlayer",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS"),
    )
}
