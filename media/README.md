## This directory contains images used by the app.

Some of the images are called icons - they are used as Ledger dashboard icons for the app.
This is specified in the root Cargo.toml, which is parsed by the Ledger SDK build script.

The others are called glyphs and they are used inside the app itself (embedded in the Rust code via `load_glyph`).

Note that `icons/mintlayer_14x14.gif` serves both as an icon and as a glyph.

⚠️ After an image is modified, do a clean build, otherwise it may not be picked up.

ℹ️ For the glyphs, it's possible to force Rust code rebuild via `cargo:rerun-if-changed` in `build.rs`
(in `crates/app-core`), which would cause the images to be picked up. But it doesn't seem to be possible
to force a rerun of the SDK build script. So we don't do it for the glyphs either.

### The purpose and required format for each image:
| File                       | Purpose                                 | Required format              |
| -------------------------- | --------------------------------------- | ---------------------------- |
| icons/mintlayer_14x14.gif  | app icon and in-app glyph for Nano X/S+ | 14x14, 1-bit black/white     |
| icons/mintlayer_32x32.gif  | app icon for Stax                       | 32x32, grayscale <=16 colors |
| icons/mintlayer_32x32.png  | app icon for Apex P                     | 32x32, 1-bit black/white     |
| icons/mintlayer_40x40.gif  | app icon for Flex                       | 40x40, grayscale <=16 colors |
| glyphs/mintlayer_48x48.png | in-app glyph for Apex P                 | 48x48, 1-bit black/white     |
| glyphs/mintlayer_64x64.gif | in-app glyph for Stax/Flex              | 64x64, grayscale <=16 colors |
