#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DOC_PATH="${ROOT_DIR}/docs/design/per-core-wal-buffer-architecture.md"
WAL_PATH="${ROOT_DIR}/crates/fsqlite-wal/src/parallel_wal.rs"
PAGER_PATH="${ROOT_DIR}/crates/fsqlite-pager/src/pager.rs"

require_pattern() {
    local pattern="$1"
    local path="$2"
    if ! rg -q --fixed-strings "${pattern}" "${path}"; then
        printf 'missing pattern %q in %s\n' "${pattern}" "${path}" >&2
        exit 1
    fi
}

require_pattern 'Per-Core Parallel WAL Design Contract (`bd-3wop3.1.1`)' "${DOC_PATH}"
require_pattern 'Commit Certificate Record' "${DOC_PATH}"
require_pattern 'Exact irreducible ordered residue' "${DOC_PATH}"
require_pattern 'Safe-Mode Fallback and Operator Control Surface' "${DOC_PATH}"
require_pattern 'Decision-Plane Contract' "${DOC_PATH}"
require_pattern 'Invariants Ledger' "${DOC_PATH}"
require_pattern 'Logging Contract Schema' "${DOC_PATH}"
require_pattern 'scripts/verify_d1_parallel_wal_design_contract.sh' "${DOC_PATH}"

require_pattern 'pub enum ParallelWalOperatingMode' "${WAL_PATH}"
require_pattern 'pub enum ParallelWalFallbackReason' "${WAL_PATH}"
require_pattern 'pub struct ParallelWalControlSurface' "${WAL_PATH}"
require_pattern 'pub struct ParallelWalCommitCertificate' "${WAL_PATH}"
require_pattern 'pub struct ParallelWalTraceRecord' "${WAL_PATH}"
require_pattern 'pub struct ParallelWalDecisionRecord' "${WAL_PATH}"

require_pattern 'pub struct ParallelWalPublicationIntent' "${PAGER_PATH}"
require_pattern 'PagerPublishedSnapshot' "${PAGER_PATH}"

printf '[PASS] D1 parallel WAL design contract artifacts are present.\n'
