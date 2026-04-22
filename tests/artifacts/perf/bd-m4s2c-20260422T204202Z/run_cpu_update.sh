#!/usr/bin/env bash
set -euo pipefail

OUT="tests/artifacts/perf/bd-m4s2c-20260422T204202Z"
TARGET="/home/ubuntu/rch_target_fsqlite_cod4"

env CARGO_TARGET_DIR="$TARGET" cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete
samply record --save-only \
  -o "$OUT/cpu-update-100-of-1000.json.gz" \
  -- "$TARGET/release-perf/perf-update-delete" 1000 20 update
