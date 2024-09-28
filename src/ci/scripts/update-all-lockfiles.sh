#!/bin/bash
# Update multiple lockfiles in the repository and record the changes to
# `update_output.json`.

set -euxo pipefail

# Map `name=path`
lockfiles=(
    "root=."
    "library=library"
    "rustbook=src/tools/rustbook"
)

echo "{}" > update_output.json

for item in "${lockfiles[@]}"; do
    name=$(echo "$item" | cut -d= -f1)
    path=$(echo "$item" | cut -d= -f2)
    manifest="$path/Cargo.toml"
    lockfile="$path/Cargo.lock"

    echo -e "$name dependencies:" >> cargo_update.log

    # Remove first line that always just says "Updating crates.io index"
    cargo update --manifest-path "$manifest" 2>&1 |
        sed '/crates.io index/d' |
        tee -a cargo_update.log

    jq -n \
        --arg path "$path" \
        --arg lockfile_path "$lockfile" \
        --rawfile lockfile "$lockfile" \
        --rawfile log cargo_update.log \
        '{ $path, $lockfile_path, $lockfile, $log }' \
        > single_update_output.json


    jq -n \
      --arg name "$name" \
      --slurpfile output update_output.json \
      --slurpfile value single_update_output.json \
      '$output[0] + { $name: $value[0] }' \
      > tmp.json

    # No inplace editing with jq...
    mv tmp.json update_output.json

    rm cargo_update.log
    rm single_update_output.json
done
