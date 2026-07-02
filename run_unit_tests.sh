#!/bin/bash

set -e
set -o nounset

# Run unit tests.
# The first argument must be the device model: nanox, nanosp, stax, flex or apex_p.
# The second argument is optional (defaults to all) and specifies the crate whose tests must be run.

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)

cd "$SCRIPT_DIR"

MODEL=$1
PACKAGE=${2:-all}

if [[ "$MODEL" == "nanosp" ]]; then
    TARGET="nanosplus"
else
    TARGET="$MODEL"
fi

echo "*** Running unit tests on $MODEL ***"

if [[ "$PACKAGE" == "all" ]]; then
    PACKAGES=(mintlayer-app-core mintlayer-messages)
else
    PACKAGES=("$PACKAGE")
fi

for package in "${PACKAGES[@]}"; do
    echo "*** Building unit tests for $package ***"

    # Build the test with normal output and without capturing it, so that build errors, if any,
    # are visible.
    cargo test -p "$package" --release --no-run --target="$TARGET"

    # Build the test again using `--message-format=json`, to capture the name of the test executable.
    output=$(cargo test -p "$package" --release --no-run --message-format=json --target="$TARGET")
    jq_selector='select(.reason == "compiler-artifact") | select(.profile.test == true) | select(.executable != null) | .executable'
    test_exe_path=$(jq -r "$jq_selector" <<< "$output")

    echo "*** Running unit tests for $package ***"
    speculos --display headless --model "$MODEL" "$test_exe_path"
done
