#!/usr/bin/env bash
# Profile the single-txn 10K INSERT hot path with perf + flamegraph.
#
# Produces:
#   /tmp/frankensqlite-perf/perf.data
#   /tmp/frankensqlite-perf/stacks.folded
#   /tmp/frankensqlite-perf/flamegraph.svg
#   /tmp/frankensqlite-perf/top.txt       (perf report --stdio top 30 functions)
#
# Requires linux-perf (`perf` on PATH) and flamegraph.pl either at /opt/FlameGraph/
# or installed via cargo install inferno (inferno-flamegraph).
#
# Usage:
#   scripts/perf_record_insert.sh                # default: 10K rows tiny_1col
#   scripts/perf_record_insert.sh medium_6col    # or any RecordSize name

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${WORKSPACE_ROOT}"

OUT_DIR="${OUT_DIR:-/tmp/frankensqlite-perf}"
CARGO_TARGET="${CARGO_TARGET_DIR:-/tmp/cargo-target}"
BIN="${CARGO_TARGET}/release-perf/comprehensive-bench"
FILTER="${1:-tiny_1col}"

mkdir -p "${OUT_DIR}"

if [[ ! -x "${BIN}" ]]; then
    echo "Building comprehensive-bench release-perf..."
    cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
fi

# Give perf a stable frame pointer so call stacks resolve. release-perf already
# has debug=false but LTO keeps symbol names. `--call-graph dwarf` is slower
# but produces better stacks than `fp`.
echo "[1/3] Recording perf data on single-txn INSERT 10K ${FILTER}..."
perf record -g --call-graph dwarf,8192 -F 999 \
    -o "${OUT_DIR}/perf.data" \
    -- "${BIN}" --quick --filter "${FILTER}" > "${OUT_DIR}/bench.log" 2>&1

echo "[2/3] Folding stacks..."
if [[ -x /opt/FlameGraph/stackcollapse-perf.pl ]]; then
    perf script -i "${OUT_DIR}/perf.data" | /opt/FlameGraph/stackcollapse-perf.pl > "${OUT_DIR}/stacks.folded"
    /opt/FlameGraph/flamegraph.pl "${OUT_DIR}/stacks.folded" > "${OUT_DIR}/flamegraph.svg"
elif command -v inferno-collapse-perf >/dev/null 2>&1; then
    perf script -i "${OUT_DIR}/perf.data" | inferno-collapse-perf > "${OUT_DIR}/stacks.folded"
    inferno-flamegraph < "${OUT_DIR}/stacks.folded" > "${OUT_DIR}/flamegraph.svg"
else
    echo "warning: neither /opt/FlameGraph nor inferno-flamegraph found; skipping SVG"
    perf script -i "${OUT_DIR}/perf.data" > "${OUT_DIR}/stacks.raw"
fi

echo "[3/3] Top 30 functions..."
perf report -i "${OUT_DIR}/perf.data" --stdio --no-children --sort=overhead,symbol \
    | head -60 > "${OUT_DIR}/top.txt"

echo
echo "Artifacts in ${OUT_DIR}:"
ls -lh "${OUT_DIR}"
