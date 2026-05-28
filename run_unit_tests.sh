#!/bin/bash

set -e
set -o nounset

# Run unit tests; the first argument must be the device model: nanox, nanosp, stax, flex or apex_p.

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)

cd "$SCRIPT_DIR"

MODEL=$1

if [[ "$MODEL" == "nanosp" ]]; then
    TARGET="nanosplus"
else
    TARGET="$MODEL"
fi

echo "*** Running unit tests on $MODEL ***"

PACKAGES=(mintlayer-app-core)

for package in "${PACKAGES[@]}"; do
    echo "*** Building unit tests for $package ***"

    output=$(cargo test -p "$package" --release --no-run --message-format=json --target="$TARGET")
    jq_selector='select(.reason == "compiler-artifact") | select(.profile.test == true) | select(.executable != null) | .executable'
    test_exe_path=$(jq -r "$jq_selector" <<< "$output")

    echo "*** Running unit tests for $package ***"
    speculos --display headless --model "$MODEL" "$test_exe_path"
done
