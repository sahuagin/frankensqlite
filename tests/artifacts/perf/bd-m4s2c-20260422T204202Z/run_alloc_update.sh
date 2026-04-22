#!/usr/bin/env bash
set -euo pipefail

OUT="tests/artifacts/perf/bd-m4s2c-20260422T204202Z"
TARGET="/home/ubuntu/rch_target_fsqlite_cod4"

env CARGO_TARGET_DIR="$TARGET" cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete
heaptrack --record-only \
  -o "$OUT/alloc-update-100-of-1000.heaptrack" \
  "$TARGET/release-perf/perf-update-delete" 1000 20 update
