#!/usr/bin/env bash

set -eux

cd "$(dirname "${BASH_SOURCE}")/.."
pwd
mkdir -p "logs"

SUFFIX="$(printf '%05d' "$1")"

for i in `seq 1 100`; do
    seed="$i$SUFFIX"
    mkdir -p "logs/$seed"
    bash -c "time timeout 300 cargo run --bin run_solve_no_marks <<< 'local random 24 $seed'" \
        > "logs/$seed/stdout" 2> "logs/$seed/stderr"
done
