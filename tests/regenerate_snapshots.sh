#!/bin/bash

set -e
set -o nounset

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
SNAPSHOTS_DIR=$SCRIPT_DIR/snapshots
ROOT_DIR=$(cd "$SCRIPT_DIR/.." && pwd)

MODELS=(nanox nanosp stax flex apex_p)

cd "$ROOT_DIR"

echo "*** Removing old snapshots ***"
rm -rf "$SNAPSHOTS_DIR"

for model in "${MODELS[@]}"; do
    if [[ "$model" == "nanosp" ]]; then
        TARGET="nanosplus"
    else
        TARGET="$model"
    fi

    echo "*** Building the app for $model ***"
    cargo ledger build "$TARGET"

    echo "*** Regenerating snapshots for $model ***"
    pytest tests/ --tb=short -v --device "$model"  --golden_run
done
