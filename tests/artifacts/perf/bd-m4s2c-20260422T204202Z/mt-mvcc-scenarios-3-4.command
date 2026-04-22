rch exec -- env CARGO_TARGET_DIR=${TMPDIR:-/tmp}/rch_target_fsqlite_cod4 cargo run --profile release-perf -p fsqlite-e2e --bin mt-mvcc-bench -- --rows-per-thread=500 --threads=1,2,4,8
