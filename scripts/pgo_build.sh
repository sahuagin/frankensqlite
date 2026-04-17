#!/usr/bin/env bash
# Build `comprehensive-bench` (and release-perf libraries) with Profile-Guided Optimization.
#
# Pipeline:
#   1. Instrumented build with `-Cprofile-generate=<dir>`.
#   2. Run `comprehensive-bench --quick` to collect representative profiles.
#   3. Merge `.profraw` files with `llvm-profdata merge`.
#   4. Final build with `-Cprofile-use=<merged.profdata>`.
#   5. Optional BOLT post-link optimization if `llvm-bolt` is available.
#
# Outputs into `/tmp/frankensqlite-pgo/` by default; override with $PGO_DIR.
#
# Usage:
#   scripts/pgo_build.sh                    # full pipeline
#   scripts/pgo_build.sh --skip-training    # reuse existing profiles
#   scripts/pgo_build.sh --bolt             # also run BOLT after PGO
#
# NOTE: PGO doubles link time. The first run takes ~8 minutes on this hardware.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${WORKSPACE_ROOT}"

PGO_DIR="${PGO_DIR:-/tmp/frankensqlite-pgo}"
PROFRAW_DIR="${PGO_DIR}/profraw"
PROFDATA="${PGO_DIR}/merged.profdata"
CARGO_TARGET="${CARGO_TARGET_DIR:-/tmp/cargo-target}"

SKIP_TRAINING=0
RUN_BOLT=0
for arg in "$@"; do
    case "$arg" in
        --skip-training) SKIP_TRAINING=1 ;;
        --bolt) RUN_BOLT=1 ;;
        -h|--help)
            sed -n '2,/^set -euo/p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *)
            echo "unknown arg: $arg" >&2
            exit 2
            ;;
    esac
done

# Locate llvm-profdata that matches this rustc's LLVM version.
RUSTC_SYSROOT="$(rustc --print sysroot)"
LLVM_PROFDATA="${RUSTC_SYSROOT}/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-profdata"
if [[ ! -x "${LLVM_PROFDATA}" ]]; then
    LLVM_PROFDATA="$(command -v llvm-profdata || true)"
fi
if [[ -z "${LLVM_PROFDATA}" ]]; then
    echo "error: llvm-profdata not found. Install with: rustup component add llvm-tools-preview" >&2
    exit 1
fi

mkdir -p "${PROFRAW_DIR}"

BIN_OUT="${CARGO_TARGET}/release-perf/comprehensive-bench"

if [[ "${SKIP_TRAINING}" -eq 0 ]]; then
    echo "[1/4] Instrumented build (-Cprofile-generate)..."
    rm -rf "${PROFRAW_DIR}" && mkdir -p "${PROFRAW_DIR}"
    RUSTFLAGS="-Cprofile-generate=${PROFRAW_DIR}" \
        cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench

    echo "[2/4] Training run (comprehensive-bench --quick)..."
    LLVM_PROFILE_FILE="${PROFRAW_DIR}/default_%m_%p.profraw" \
        "${BIN_OUT}" --quick > "${PGO_DIR}/training_run.log" 2>&1
    echo "      training profiles:"
    ls -lh "${PROFRAW_DIR}" | tail -5

    echo "[3/4] Merging profdata..."
    "${LLVM_PROFDATA}" merge -output="${PROFDATA}" "${PROFRAW_DIR}"/*.profraw
    echo "      merged: ${PROFDATA} ($(stat -c '%s' "${PROFDATA}") bytes)"
else
    if [[ ! -f "${PROFDATA}" ]]; then
        echo "error: --skip-training given but ${PROFDATA} does not exist" >&2
        exit 1
    fi
    echo "[1-3/4] Skipped (reusing ${PROFDATA})"
fi

echo "[4/4] PGO-optimized build (-Cprofile-use)..."
# -Cllvm-args=-pgo-warn-missing-function is noisy but useful for diagnosis.
RUSTFLAGS="-Cprofile-use=${PROFDATA}" \
    cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench

if [[ "${RUN_BOLT}" -eq 1 ]]; then
    if ! command -v llvm-bolt >/dev/null 2>&1; then
        echo "warning: llvm-bolt not found on PATH; skipping BOLT stage" >&2
    else
        echo "[BOLT] Post-link optimization..."
        BOLT_PROFILE="${PGO_DIR}/bolt.fdata"
        BOLT_BIN="${BIN_OUT}.bolt"
        perf record -e cycles:u -j any,u -o "${PGO_DIR}/perf.data" -- \
            "${BIN_OUT}" --quick > /dev/null 2>&1
        perf2bolt -p "${PGO_DIR}/perf.data" -o "${BOLT_PROFILE}" "${BIN_OUT}"
        llvm-bolt "${BIN_OUT}" -o "${BOLT_BIN}" -data="${BOLT_PROFILE}" \
            -reorder-blocks=ext-tsp -reorder-functions=hfsort+ -split-functions -icf=1
        echo "      BOLT output: ${BOLT_BIN}"
    fi
fi

echo
echo "Done. Final binary: ${BIN_OUT}"
echo "To compare against non-PGO baseline, rebuild without RUSTFLAGS and diff timings."
