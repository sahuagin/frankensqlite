//! Contract tests for the T6.7.8.1 WAL generation and authoritative lookup doc.
//!
//! This pins the design layer to the existing source symbols, verifier scripts,
//! and runtime diagnostics so the contract cannot drift into vague prose.

use std::fs;
use std::path::{Path, PathBuf};

const BEAD_ID: &str = "bd-1dp9.6.7.8.1";
const DOC_PATH: &str = "docs/design/wal-generation-index-authoritative-lookup-contract.md";
const WAL_PATH: &str = "crates/fsqlite-wal/src/wal.rs";
const WAL_ADAPTER_PATH: &str = "crates/fsqlite-core/src/wal_adapter.rs";
const INDEX_SCRIPT_PATH: &str = "scripts/verify_t6_7_wal_index.sh";
const PUBLICATION_SCRIPT_PATH: &str = "scripts/verify_t6_7_wal_publication_plane.sh";

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root should canonicalize")
}

fn read_text(rel_path: &str) -> String {
    let path = workspace_root().join(rel_path);
    fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("failed to read {} at {}: {error}", rel_path, path.display());
    })
}

#[test]
fn wal_index_contract_doc_is_pinned_to_bead_and_repo_surfaces() {
    let doc = read_text(DOC_PATH);

    assert!(
        doc.contains("# WAL Generation and Authoritative Lookup Contract"),
        "design doc must expose the contract title"
    );
    assert!(
        doc.contains(BEAD_ID),
        "design doc must be explicitly pinned to the bead id"
    );
    for required_ref in [
        WAL_PATH,
        WAL_ADAPTER_PATH,
        INDEX_SCRIPT_PATH,
        PUBLICATION_SCRIPT_PATH,
    ] {
        assert!(
            doc.contains(required_ref),
            "design doc must reference {required_ref}"
        );
    }
}

#[test]
fn wal_index_contract_doc_declares_generation_lookup_and_recovery_invariants() {
    let doc = read_text(DOC_PATH);

    for required in [
        "WalGenerationIdentity",
        "checkpoint_seq",
        "salt1",
        "salt2",
        "WalPublishedSnapshot",
        "publication_seq",
        "WalPageLookupResolution",
        "authoritative_index",
        "partial_index_fallback",
        "reverse scan",
        "steady-state",
        "reset",
        "truncate",
        "ABA",
        "torn tail",
        "corruption",
        "WalCorrupt",
        "lookup_mode",
        "fallback_reason",
        "snapshot_age",
    ] {
        assert!(
            doc.contains(required),
            "design doc must explicitly mention `{required}`"
        );
    }
}

#[test]
fn wal_index_contract_scripts_pin_runtime_tests_and_trace_fields() {
    let index_script = read_text(INDEX_SCRIPT_PATH);
    let publication_script = read_text(PUBLICATION_SCRIPT_PATH);

    for required_test in [
        "test_page_index_invalidated_on_wal_reset",
        "test_page_index_invalidated_on_same_salt_generation_change",
        "test_lookup_contract_distinguishes_authoritative_and_fallback_paths",
        "test_partial_index_falls_back_to_linear_scan",
        "test_refresh_after_reset_with_same_salts_detects_new_generation",
    ] {
        let present =
            index_script.contains(required_test) || publication_script.contains(required_test);
        assert!(
            present,
            "verifier scripts must reference runtime test `{required_test}`"
        );
    }

    for required_field in [
        "wal_generation",
        "wal_salt1",
        "wal_salt2",
        "publication_seq",
        "frame_delta_count",
        "latest_frame_entries",
        "snapshot_age",
        "lookup_mode",
        "fallback_reason",
        "wal_checkpoint_seq",
    ] {
        let present =
            index_script.contains(required_field) || publication_script.contains(required_field);
        assert!(
            present,
            "verifier scripts must pin trace field `{required_field}`"
        );
    }
}

#[test]
fn wal_index_contract_matches_live_source_symbols() {
    let wal_source = read_text(WAL_PATH);
    let adapter_source = read_text(WAL_ADAPTER_PATH);

    for required in [
        "pub struct WalGenerationIdentity",
        "checkpoint_seq",
        "salts: WalSalts",
        "header_generation_changed_rebuild",
        "truncated_tail_stop",
        "test_refresh_after_reset_with_same_salts_detects_new_generation",
    ] {
        assert!(
            wal_source.contains(required),
            "wal.rs must still contain `{required}` for the contract to remain valid"
        );
    }

    for required in [
        "enum WalPageLookupResolution",
        "AuthoritativeHit",
        "AuthoritativeMiss",
        "PartialIndexFallbackHit",
        "PartialIndexFallbackMiss",
        "struct WalPublishedSnapshot",
        "index_is_partial",
        "scan_backwards_for_page",
        "WalCorrupt",
        "test_lookup_contract_distinguishes_authoritative_and_fallback_paths",
        "test_partial_index_falls_back_to_linear_scan",
    ] {
        assert!(
            adapter_source.contains(required),
            "wal_adapter.rs must still contain `{required}` for the contract to remain valid"
        );
    }
}
