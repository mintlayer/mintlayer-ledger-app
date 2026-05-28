#!/bin/bash

set -e
set -o nounset

# Run some extra checks (for now its mostly clippy).

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)

cd "$SCRIPT_DIR"

echo "Running cargo fmt"
cargo fmt --check -- --config newline_style=Unix

# It doesn't matter which one we use, but we need to specify one.
CLIPPY_TARGET_ARG=--target=apex_p

echo "Running clippy (any code)"
cargo clippy "$CLIPPY_TARGET_ARG" --all-features --workspace --bins --lib --tests  -- \
    -D warnings \
    -D clippy::implicit_saturating_sub \
    -D clippy::implicit_clone \
    -D clippy::map_unwrap_or \
    -D clippy::unnested_or_patterns \
    -D clippy::mut_mut \
    -D clippy::todo \
    -A clippy::let-and-return \
    -A clippy::unnecessary-lazy-evaluations \
    -A clippy::boxed-local

echo "Running clippy (production code)"
# TODO: consider also enabling `unwrap_used` and `items_after_statements`.
cargo clippy "$CLIPPY_TARGET_ARG" --all-features --workspace --bins --lib -- \
    -A clippy::all \
    -D clippy::float_arithmetic \
    -D clippy::dbg_macro \
    -D clippy::fallible_impl_from \
    -D clippy::string_slice

echo "All checks passed"
