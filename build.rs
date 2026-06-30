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

use image::{ImageFormat, ImageReader, Pixel};

// FIXME: all image files currently contain the Rust logo; need to replace them with Mintlayer logo.

fn main() {
    println!("cargo:rerun-if-changed=script.ld");
    println!("cargo:rerun-if-changed=media/icons/mintlayer_14x14.gif");
    println!("cargo:rerun-if-changed=media/icons/mask_14x14.gif");

    let icons_path = std::path::PathBuf::from("media/icons");
    let mut gray = ImageReader::open(icons_path.join("mintlayer_14x14.gif"))
        .unwrap()
        .decode()
        .unwrap()
        .into_luma8();

    // Apply mask
    let mask = ImageReader::open(icons_path.join("mask_14x14.gif"))
        .unwrap()
        .decode()
        .unwrap()
        .into_luma8();

    for (x, y, mask_pixel) in mask.enumerate_pixels() {
        let mask_value = mask_pixel[0];
        let mut gray_pixel = *gray.get_pixel(x, y);
        if mask_value == 0 {
            gray_pixel = image::Luma([0]);
        } else {
            gray_pixel.invert();
        }
        gray.put_pixel(x, y, gray_pixel);
    }

    let glyph_path = std::path::PathBuf::from("media/glyphs");
    gray.save_with_format(glyph_path.join("home_nano_nbgl.png"), ImageFormat::Png)
        .unwrap();

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("Failed to execute git command");

    let git_hash = String::from_utf8(output.stdout).expect("Failed to convert git output to UTF-8");

    // Expose the Git hash as an environment variable
    // FIXME: this is unused. Either implement a custom command that would return this info
    // (e.g. in the form of a full semantic version), or remove this.
    println!("cargo:rustc-env=GIT_HASH={}", git_hash.trim());

    // Rerun the build script if .git/HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
}
