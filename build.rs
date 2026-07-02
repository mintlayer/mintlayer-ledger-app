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

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=script.ld");

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("Failed to execute git command");

    let git_hash = String::from_utf8(output.stdout).expect("Failed to convert git output to UTF-8");

    // Expose the Git hash as an environment variable
    // TODO: this is unused. Either implement a custom command that would return this info
    // (e.g. in the form of a full semantic version), or remove this.
    // See https://github.com/mintlayer/mintlayer-ledger-app/issues/11.
    println!("cargo:rustc-env=GIT_HASH={}", git_hash.trim());

    // Rerun the build script if .git/HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
}
