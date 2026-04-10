#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

CARGO_PROFILE="${CARGO_PROFILE:-release-perf}"
DB_ID="${DB_ID:-frankensqlite}"
WORKLOAD="${WORKLOAD:-mixed_read_write}"
CONCURRENCY="${CONCURRENCY:-4}"
SEED="${SEED:-42}"
SCALE="${SCALE:-50}"
GOLDEN_DIR="${GOLDEN_DIR:-${WORKSPACE_ROOT}/sample_sqlite_db_files/golden}"
WORKING_BASE="${WORKING_BASE:-${WORKSPACE_ROOT}/sample_sqlite_db_files/working}"
MVCC="${MVCC:-1}"
RUN_INTEGRITY_CHECK="${RUN_INTEGRITY_CHECK:-0}"
PRETTY="${PRETTY:-0}"
EMIT_INLINE_BUNDLE="${EMIT_INLINE_BUNDLE:-0}"
OUTPUT_DIR="${OUTPUT_DIR:-}"

passthrough_args=()

compute_sha256() {
    local path="$1"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "${path}" | awk '{print $1}'
        return 0
    fi
    if command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "${path}" | awk '{print $1}'
        return 0
    fi
    return 1
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --golden-dir)
            if [[ $# -lt 2 ]]; then
                echo "error: --golden-dir requires a directory path" >&2
                exit 2
            fi
            GOLDEN_DIR="$2"
            shift 2
            ;;
        --working-base)
            if [[ $# -lt 2 ]]; then
                echo "error: --working-base requires a directory path" >&2
                exit 2
            fi
            WORKING_BASE="$2"
            shift 2
            ;;
        --db)
            if [[ $# -lt 2 ]]; then
                echo "error: --db requires a fixture id" >&2
                exit 2
            fi
            DB_ID="$2"
            shift 2
            ;;
        --workload|--preset)
            if [[ $# -lt 2 ]]; then
                echo "error: $1 requires a preset name" >&2
                exit 2
            fi
            WORKLOAD="$2"
            shift 2
            ;;
        --concurrency)
            if [[ $# -lt 2 ]]; then
                echo "error: --concurrency requires an integer" >&2
                exit 2
            fi
            CONCURRENCY="$2"
            shift 2
            ;;
        --seed)
            if [[ $# -lt 2 ]]; then
                echo "error: --seed requires an integer" >&2
                exit 2
            fi
            SEED="$2"
            shift 2
            ;;
        --scale)
            if [[ $# -lt 2 ]]; then
                echo "error: --scale requires an integer" >&2
                exit 2
            fi
            SCALE="$2"
            shift 2
            ;;
        --output-dir)
            if [[ $# -lt 2 ]]; then
                echo "error: --output-dir requires a directory path" >&2
                exit 2
            fi
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --mvcc)
            MVCC=1
            shift
            ;;
        --no-mvcc)
            MVCC=0
            shift
            ;;
        --integrity-check)
            RUN_INTEGRITY_CHECK=1
            shift
            ;;
        --pretty)
            PRETTY=1
            shift
            ;;
        --emit-inline-bundle)
            EMIT_INLINE_BUNDLE=1
            shift
            ;;
        *)
            passthrough_args+=("$1")
            shift
            ;;
    esac
done

mode_tag="mvcc"
mode_flag="--mvcc"
if [[ "${MVCC}" == "0" ]]; then
    mode_tag="single_writer"
    mode_flag="--no-mvcc"
fi

if [[ -z "${OUTPUT_DIR}" ]]; then
    OUTPUT_DIR="${WORKSPACE_ROOT}/artifacts/hot-profile/${DB_ID}__${WORKLOAD}__c${CONCURRENCY}__s${SEED}__${mode_tag}"
fi

if [[ -n "${BEAD_ID:-}" && -z "${FSQLITE_HOT_PATH_BEAD_ID:-}" ]]; then
    export FSQLITE_HOT_PATH_BEAD_ID="${BEAD_ID}"
fi
export FSQLITE_HOT_PATH_CARGO_PROFILE="${FSQLITE_HOT_PATH_CARGO_PROFILE:-${CARGO_PROFILE}}"
export FSQLITE_HOT_PATH_WORKSPACE_ROOT="${FSQLITE_HOT_PATH_WORKSPACE_ROOT:-${WORKSPACE_ROOT}}"
if [[ -z "${FSQLITE_HOT_PATH_SOURCE_REVISION:-}" ]] && git -C "${WORKSPACE_ROOT}" rev-parse HEAD >/dev/null 2>&1; then
    export FSQLITE_HOT_PATH_SOURCE_REVISION="$(git -C "${WORKSPACE_ROOT}" rev-parse HEAD)"
fi
if [[ -z "${FSQLITE_HOT_PATH_BEADS_DATA_HASH:-}" && -f "${WORKSPACE_ROOT}/.beads/issues.jsonl" ]]; then
    if beads_hash="$(compute_sha256 "${WORKSPACE_ROOT}/.beads/issues.jsonl")"; then
        export FSQLITE_HOT_PATH_BEADS_DATA_HASH="${beads_hash}"
    fi
fi

cmd=(
    cargo
    run
    --profile
    "${CARGO_PROFILE}"
    -p
    fsqlite-e2e
    --bin
    realdb-e2e
    --
    hot-profile
    --golden-dir
    "${GOLDEN_DIR}"
    --working-base
    "${WORKING_BASE}"
    --db
    "${DB_ID}"
    --workload
    "${WORKLOAD}"
    --concurrency
    "${CONCURRENCY}"
    --seed
    "${SEED}"
    --scale
    "${SCALE}"
    --output-dir
    "${OUTPUT_DIR}"
    "${mode_flag}"
)

if [[ "${RUN_INTEGRITY_CHECK}" == "1" ]]; then
    cmd+=(--integrity-check)
fi
if [[ "${PRETTY}" == "1" ]]; then
    cmd+=(--pretty)
fi
if [[ "${EMIT_INLINE_BUNDLE}" == "1" ]]; then
    cmd+=(--emit-inline-bundle)
fi
cmd+=("${passthrough_args[@]}")

cd "${WORKSPACE_ROOT}"
"${cmd[@]}"
