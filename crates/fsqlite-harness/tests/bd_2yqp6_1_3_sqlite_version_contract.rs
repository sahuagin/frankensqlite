//! Drift-gate tests for sqlite_version_contract.toml (bd-2yqp6.1.3).
//!
//! Enforces a single canonical SQLite target version across runtime, harness,
//! and docs, and verifies parity reports carry an explicit contract reference.

#![allow(clippy::struct_field_names)]

use std::fs;
use std::path::{Path, PathBuf};

use fsqlite_harness::differential_v2::{
    EngineIdentity, ExecutionEnvelope, NormalizedValue, Outcome, SqlExecutor, run_differential,
};
use serde::Deserialize;

const BEAD_ID: &str = "bd-2yqp6.1.3";

#[derive(Debug, Deserialize)]
struct VersionContractDocument {
    meta: VersionContractMeta,
    contract: VersionContract,
    references: VersionReferences,
}

#[derive(Debug, Deserialize)]
struct VersionContractMeta {
    schema_version: String,
    bead_id: String,
    track_id: String,
    generated_at: String,
    contract_owner: String,
}

#[derive(Debug, Deserialize)]
struct VersionContract {
    sqlite_target: String,
    runtime_pragma_sqlite_version: String,
    contract_reference_path: String,
}

#[derive(Debug, Deserialize)]
struct VersionReferences {
    runtime_source: String,
    surface_matrix: String,
    feature_ledger: String,
    parity_report_module: String,
    readme: String,
}

#[derive(Debug, Deserialize)]
struct SurfaceMatrix {
    meta: SurfaceMeta,
}

#[derive(Debug, Deserialize)]
struct SurfaceMeta {
    sqlite_target: String,
    sqlite_version_contract: String,
}

#[derive(Debug, Deserialize)]
struct LedgerDocument {
    meta: LedgerMeta,
}

#[derive(Debug, Deserialize)]
struct LedgerMeta {
    sqlite_target: String,
    sqlite_version_contract: String,
}

struct StaticExecutor {
    identity: EngineIdentity,
}

impl SqlExecutor for StaticExecutor {
    fn execute(&self, _sql: &str) -> Result<usize, String> {
        Ok(0)
    }

    fn query(&self, _sql: &str) -> Result<Vec<Vec<NormalizedValue>>, String> {
        Ok(vec![vec![NormalizedValue::Integer(1)]])
    }

    fn engine_identity(&self) -> EngineIdentity {
        self.identity
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn resolve_reference_path(path: &str) -> PathBuf {
    let rel = Path::new(path);
    if rel.components().count() == 1 && path.ends_with(".toml") {
        workspace_root().join("docs/contracts").join(rel)
    } else {
        workspace_root().join(rel)
    }
}

fn read_text(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    })
}

fn load_version_contract() -> VersionContractDocument {
    let path = workspace_root().join("docs/contracts/sqlite_version_contract.toml");
    toml::from_str(&read_text(&path)).unwrap_or_else(|error| {
        panic!("failed to parse {}: {error}", path.display());
    })
}

#[test]
fn contract_meta_and_schema_are_valid() {
    let doc = load_version_contract();
    assert_eq!(doc.meta.schema_version, "1.0.0");
    assert_eq!(doc.meta.bead_id, BEAD_ID);
    assert_eq!(doc.meta.track_id, "bd-2yqp6.1");
    assert!(!doc.meta.generated_at.trim().is_empty());
    assert!(!doc.meta.contract_owner.trim().is_empty());
    assert_eq!(doc.contract.sqlite_target, "3.52.0");
    assert_eq!(doc.contract.runtime_pragma_sqlite_version, "3.52.0");
    assert_eq!(
        doc.contract.contract_reference_path,
        "docs/contracts/sqlite_version_contract.toml"
    );
}

#[test]
fn matrix_and_ledger_are_pinned_to_contract_target() {
    let contract = load_version_contract();

    let matrix_path = resolve_reference_path(&contract.references.surface_matrix);
    let matrix: SurfaceMatrix = toml::from_str(&read_text(&matrix_path)).unwrap_or_else(|error| {
        panic!("failed to parse {}: {error}", matrix_path.display());
    });
    assert_eq!(matrix.meta.sqlite_target, contract.contract.sqlite_target);
    assert_eq!(
        matrix.meta.sqlite_version_contract,
        contract.contract.contract_reference_path
    );

    let ledger_path = resolve_reference_path(&contract.references.feature_ledger);
    let ledger: LedgerDocument = toml::from_str(&read_text(&ledger_path)).unwrap_or_else(|error| {
        panic!("failed to parse {}: {error}", ledger_path.display());
    });
    assert_eq!(ledger.meta.sqlite_target, contract.contract.sqlite_target);
    assert_eq!(
        ledger.meta.sqlite_version_contract,
        contract.contract.contract_reference_path
    );
}

#[test]
fn runtime_pragma_sqlite_version_matches_contract() {
    let contract = load_version_contract();
    let path = workspace_root().join(&contract.references.runtime_source);
    let source = read_text(&path);
    assert!(
        source.contains("FRANKENSQLITE_SQLITE_VERSION"),
        "runtime source should route sqlite_version through the canonical version constant"
    );
    assert_eq!(
        fsqlite_types::FRANKENSQLITE_SQLITE_VERSION,
        contract.contract.runtime_pragma_sqlite_version
    );
}

#[test]
fn parity_report_embeds_contract_reference() {
    let contract = load_version_contract();

    let envelope = ExecutionEnvelope::builder(3520)
        .engines(
            "0.1.0-test",
            format!("{}-test", contract.contract.sqlite_target),
        )
        .workload(["SELECT 1".to_owned()])
        .build();
    let subject = StaticExecutor {
        identity: EngineIdentity::FrankenSqlite,
    };
    let oracle = StaticExecutor {
        identity: EngineIdentity::CSqliteOracle,
    };

    let result = run_differential(&envelope, &subject, &oracle);
    assert_eq!(result.outcome, Outcome::Pass);
    assert_eq!(
        result.target_sqlite_version,
        contract.contract.sqlite_target
    );
    assert_eq!(
        result.sqlite_version_contract,
        contract.contract.contract_reference_path
    );

    let parity_module = workspace_root().join(&contract.references.parity_report_module);
    let parity_source = read_text(&parity_module);
    assert!(
        parity_source.contains("SQLITE_VERSION_CONTRACT_PATH"),
        "missing parity report contract-path constant in {}",
        parity_module.display()
    );
}

#[test]
fn readme_documents_contract_reference() {
    let contract = load_version_contract();
    let readme_path = workspace_root().join(&contract.references.readme);
    let readme = read_text(&readme_path);

    assert!(
        readme.contains(&contract.contract.sqlite_target),
        "README missing sqlite target {}",
        contract.contract.sqlite_target
    );
    assert!(
        readme.contains(&contract.contract.contract_reference_path),
        "README missing contract path {}",
        contract.contract.contract_reference_path
    );
}

#[test]
fn canonical_doc_lists_track_a_artifacts_and_validation_gates() {
    let contract = load_version_contract();
    let doc_path = workspace_root().join("docs/canonical_parity_contract.md");
    let doc = read_text(&doc_path);

    for artifact in [
        contract.contract.contract_reference_path.as_str(),
        contract.references.surface_matrix.as_str(),
        contract.references.feature_ledger.as_str(),
        "parity_score_contract.toml",
    ] {
        assert!(
            doc.contains(artifact),
            "canonical parity doc missing artifact reference {artifact}"
        );
    }

    for validation_gate in [
        "bd_2yqp6_1_1_supported_surface_matrix.rs",
        "bd_2yqp6_1_2_feature_universe_ledger.rs",
        "bd_2yqp6_1_3_sqlite_version_contract.rs",
        "bd_2yqp6_1_4_parity_score_contract.rs",
    ] {
        assert!(
            doc.contains(validation_gate),
            "canonical parity doc missing validation gate {validation_gate}"
        );
    }
}
