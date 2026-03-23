#!/usr/bin/env bash
# bd-db300.1.7.1: Capture authoritative c1 hot-path artifact packs.
#
# Captures release-perf artifact packs for the worst low-concurrency cells:
# - commutative_inserts_disjoint_keys c1 (0.205x — worst)
# - hot_page_contention c1 (0.633x)
# - mixed_read_write c1 (3.519x — already winning, included for comparison)
#
# Runs both FrankenSQLite (MVCC + single-writer) and C SQLite as control.
#
# Usage:
#   ./scripts/capture_c1_evidence_pack.sh [--output-dir DIR]
#
# Requirements:
#   - Build with: cargo build --profile release-perf -p fsqlite-e2e
#   - Or: CARGO_TARGET_DIR=/tmp/c1-evidence cargo build --profile release-perf -p fsqlite-e2e

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

OUTPUT_DIR="${1:-${PROJECT_ROOT}/artifacts/c1_evidence_pack_$(date +%Y%m%d_%H%M%S)}"
mkdir -p "$OUTPUT_DIR"

# Record build metadata.
cat > "$OUTPUT_DIR/build_metadata.json" << METADATA
{
  "date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "profile": "release-perf",
  "hostname": "$(hostname)",
  "rustc_version": "$(rustc --version)",
  "cargo_target_dir": "${CARGO_TARGET_DIR:-default}",
  "git_sha": "$(git -C "$PROJECT_ROOT" rev-parse HEAD 2>/dev/null || echo 'unknown')",
  "git_dirty": "$(git -C "$PROJECT_ROOT" status --porcelain 2>/dev/null | wc -l | tr -d ' ')",
  "cpu_model": "$(grep 'model name' /proc/cpuinfo 2>/dev/null | head -1 | cut -d: -f2 | xargs || echo 'unknown')",
  "cpu_cores": "$(nproc 2>/dev/null || echo 'unknown')"
}
METADATA

echo "=== bd-db300.1.7.1: c1 Evidence Pack ==="
echo "Output: $OUTPUT_DIR"
echo "Profile: release-perf"
echo ""

BINARY="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}/release-perf/realdb-e2e"
if [ ! -f "$BINARY" ]; then
    echo "Building realdb-e2e with release-perf profile..."
    cd "$PROJECT_ROOT"
    cargo build --profile release-perf -p fsqlite-e2e --bin realdb-e2e
fi

# Canonical workloads for c1 evidence.
WORKLOADS="commutative_inserts_disjoint_keys,hot_page_contention,mixed_read_write"
CONCURRENCY="1"
REPEAT="3"
# Canonical fixtures from beads_benchmark_campaign.v1.json:
# frankensqlite (~11MB), frankentui (~18MB), frankensearch
# Use first fixture by default for focused c1 evidence.
DB_FIXTURE="${DB_FIXTURE:-frankensqlite}"

echo "--- C SQLite control (c1, fixture=$DB_FIXTURE) ---"
"$BINARY" bench \
    --db "$DB_FIXTURE" \
    --preset "$WORKLOADS" \
    --concurrency "$CONCURRENCY" \
    --engine sqlite3 \
    --repeat "$REPEAT" \
    --output-jsonl "$OUTPUT_DIR/c1_sqlite3.jsonl" \
    --pretty 2>&1 | tee "$OUTPUT_DIR/c1_sqlite3_stdout.log"

echo ""
echo "--- FrankenSQLite MVCC (c1, fixture=$DB_FIXTURE) ---"
"$BINARY" bench \
    --db "$DB_FIXTURE" \
    --preset "$WORKLOADS" \
    --concurrency "$CONCURRENCY" \
    --engine fsqlite \
    --mvcc \
    --repeat "$REPEAT" \
    --output-jsonl "$OUTPUT_DIR/c1_fsqlite_mvcc.jsonl" \
    --pretty 2>&1 | tee "$OUTPUT_DIR/c1_fsqlite_mvcc_stdout.log"

echo ""
echo "--- FrankenSQLite single-writer (c1, fixture=$DB_FIXTURE) ---"
"$BINARY" bench \
    --db "$DB_FIXTURE" \
    --preset "$WORKLOADS" \
    --concurrency "$CONCURRENCY" \
    --engine fsqlite \
    --no-mvcc \
    --repeat "$REPEAT" \
    --output-jsonl "$OUTPUT_DIR/c1_fsqlite_single.jsonl" \
    --pretty 2>&1 | tee "$OUTPUT_DIR/c1_fsqlite_single_stdout.log"

echo ""
echo "--- Hot-path profile (c1, worst cell: commutative_inserts_disjoint_keys) ---"
"$BINARY" hot-profile \
    --db "$DB_FIXTURE" \
    --preset "commutative_inserts_disjoint_keys" \
    --concurrency 1 \
    --mvcc \
    --pretty 2>&1 | tee "$OUTPUT_DIR/c1_hotprofile_commutative.log"

echo ""
echo "=== Evidence pack complete: $OUTPUT_DIR ==="
echo "Files:"
ls -la "$OUTPUT_DIR/"
echo ""
echo "To analyze:"
echo "  cat $OUTPUT_DIR/c1_sqlite3.jsonl | python3 -m json.tool"
echo "  cat $OUTPUT_DIR/c1_fsqlite_mvcc.jsonl | python3 -m json.tool"
echo "  diff $OUTPUT_DIR/c1_sqlite3.jsonl $OUTPUT_DIR/c1_fsqlite_mvcc.jsonl"
