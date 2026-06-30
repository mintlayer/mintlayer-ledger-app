#!/bin/bash

# Run some extra checks.

set -e
set -o nounset

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
PYTHON=$(which python || which python3)

cd "$SCRIPT_DIR"

echo "Running codecheck.py"
"$PYTHON" "tools/codecheck.py"

# Notes about clippy:
# 1. Ledger's guideline enforcer also runs it. But at the moment of writing this it doesn't check
#    tests, see https://github.com/LedgerHQ/ledger-app-workflows/blob/master/scripts/check_all.sh.
#    Besides, we want to enable some additional checks, similar to what we do in Mintlayer Core,
#    so we do a separate clippy run here.
# 2. The guideline enforcer runs it for all existing device models, but in this additional run this
#    is redundant, so we use one arbitrarily chosen model.
# 3. Unlike in Mintlayer Core, we can't disable certain annoying and mostly useless checks (such as
#    let-and-return), because the guideline enforcer will run them anyway.

echo "Running cargo fmt"
cargo fmt --check -- --config newline_style=Unix

CLIPPY_TARGET_ARG=--target=apex_p

echo "Running clippy (any code)"
cargo clippy "$CLIPPY_TARGET_ARG" --all-features --workspace --bins --lib --tests  -- \
    -D warnings \
    -D clippy::implicit_saturating_sub \
    -D clippy::implicit_clone \
    -D clippy::map_unwrap_or \
    -D clippy::unnested_or_patterns \
    -D clippy::mut_mut \
    -D clippy::todo

echo "Running clippy (production code)"
# TODO: consider also enabling `unwrap_used` and `items_after_statements`.
cargo clippy "$CLIPPY_TARGET_ARG" --all-features --workspace --bins --lib -- \
    -A clippy::all \
    -D clippy::float_arithmetic \
    -D clippy::dbg_macro \
    -D clippy::fallible_impl_from \
    -D clippy::string_slice

echo "All checks passed"
