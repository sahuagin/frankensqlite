use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};

const E2E_CARGO_TOML_REL_PATH: &str = "crates/fsqlite-e2e/Cargo.toml";
const E2E_CRATE_REL_PATH: &str = "crates/fsqlite-e2e";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum BenchmarkComparisonMode {
    Parity,
    Control,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BenchmarkMetaAnnotation {
    engine: String,
    lifecycle: String,
    storage: String,
    concurrency: String,
    comparison: BenchmarkComparisonMode,
    line_no: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct BenchmarkDimensions {
    lifecycle: String,
    storage: String,
    concurrency: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BenchmarkMetaRecord {
    file_relpath: String,
    benchmark_group: String,
    engine: String,
    dimensions: BenchmarkDimensions,
    comparison: BenchmarkComparisonMode,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct EngineMetaSummary {
    comparisons: BTreeSet<BenchmarkComparisonMode>,
    dimensions: BTreeSet<BenchmarkDimensions>,
}

#[derive(Debug, Clone, Default)]
struct BenchTargetDeclaration {
    name: Option<String>,
    path: Option<String>,
    section_line_no: usize,
}

pub fn validate_benchmark_lifecycle_parity(workspace_root: &Path) -> Vec<String> {
    let bench_relpaths = match discover_benchmark_source_files(workspace_root) {
        Ok(bench_relpaths) => bench_relpaths,
        Err(violations) => return violations,
    };

    let mut violations = Vec::new();
    let mut records = Vec::new();

    for relpath in bench_relpaths {
        let path = workspace_root.join(&relpath);
        let Ok(content) = std::fs::read_to_string(&path) else {
            violations.push(format!("{relpath}: could not read benchmark source file"));
            continue;
        };

        match parse_benchmark_meta_records(&relpath, &content) {
            Ok(file_records) => records.extend(file_records),
            Err(file_violations) => violations.extend(file_violations),
        }
    }

    violations.extend(validate_benchmark_metadata_records(records));
    violations
}

fn discover_benchmark_source_files(workspace_root: &Path) -> Result<Vec<String>, Vec<String>> {
    let manifest_path = workspace_root.join(E2E_CARGO_TOML_REL_PATH);
    let content = std::fs::read_to_string(&manifest_path).map_err(|_| {
        vec![format!(
            "{E2E_CARGO_TOML_REL_PATH}: could not read e2e Cargo manifest"
        )]
    })?;
    parse_benchmark_source_paths(E2E_CARGO_TOML_REL_PATH, &content)
}

fn parse_benchmark_source_paths(
    manifest_relpath: &str,
    content: &str,
) -> Result<Vec<String>, Vec<String>> {
    let mut violations = Vec::new();
    let mut bench_targets = Vec::new();
    let mut current_bench: Option<BenchTargetDeclaration> = None;

    for (index, raw_line) in content.lines().enumerate() {
        let line_no = index + 1;
        let trimmed = strip_toml_comment(raw_line).trim();

        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with('[') {
            if let Some(bench) = current_bench.take() {
                match finalize_bench_target(manifest_relpath, bench) {
                    Ok(relpath) => bench_targets.push(relpath),
                    Err(violation) => violations.push(violation),
                }
            }

            current_bench = (trimmed == "[[bench]]").then_some(BenchTargetDeclaration {
                name: None,
                path: None,
                section_line_no: line_no,
            });
            continue;
        }

        let Some(bench) = current_bench.as_mut() else {
            continue;
        };

        if let Some(name) = parse_toml_string_assignment(trimmed, "name") {
            bench.name = Some(name);
            continue;
        }

        if let Some(path) = parse_toml_string_assignment(trimmed, "path") {
            bench.path = Some(path);
        }
    }

    if let Some(bench) = current_bench.take() {
        match finalize_bench_target(manifest_relpath, bench) {
            Ok(relpath) => bench_targets.push(relpath),
            Err(violation) => violations.push(violation),
        }
    }

    if !violations.is_empty() {
        return Err(violations);
    }

    bench_targets.sort_unstable();
    bench_targets.dedup();
    Ok(bench_targets)
}

fn finalize_bench_target(
    manifest_relpath: &str,
    bench: BenchTargetDeclaration,
) -> Result<String, String> {
    let name = bench.name.ok_or_else(|| {
        format!(
            "{manifest_relpath}:{}: [[bench]] missing `name` field",
            bench.section_line_no
        )
    })?;
    let relative_path = bench.path.unwrap_or_else(|| format!("benches/{name}.rs"));
    let relpath = PathBuf::from(E2E_CRATE_REL_PATH).join(relative_path);
    Ok(relpath.to_string_lossy().into_owned())
}

fn strip_toml_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut escape = false;

    for (index, ch) in line.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        match ch {
            '\\' if in_string => escape = true,
            '"' => in_string = !in_string,
            '#' if !in_string => return &line[..index],
            _ => {}
        }
    }

    line
}

fn parse_toml_string_assignment(line: &str, key: &str) -> Option<String> {
    let remainder = line.strip_prefix(key)?.trim_start();
    let remainder = remainder.strip_prefix('=')?.trim_start();
    let remainder = remainder.strip_prefix('"')?;
    let end = remainder.find('"')?;
    Some(remainder[..end].to_owned())
}

fn validate_benchmark_metadata_records(records: Vec<BenchmarkMetaRecord>) -> Vec<String> {
    let mut grouped: BTreeMap<(String, String), BTreeMap<String, EngineMetaSummary>> =
        BTreeMap::new();

    for record in records {
        let engine_summary = grouped
            .entry((record.file_relpath.clone(), record.benchmark_group.clone()))
            .or_default()
            .entry(record.engine.clone())
            .or_default();
        engine_summary.comparisons.insert(record.comparison);
        engine_summary.dimensions.insert(record.dimensions);
    }

    let mut violations = Vec::new();

    for ((file_relpath, benchmark_group), engines) in grouped {
        let Some(csqlite) = engines.get("csqlite") else {
            violations.push(format!(
                "{file_relpath}:{benchmark_group}: missing csqlite BENCH-META entry"
            ));
            continue;
        };
        let Some(frankensqlite) = engines.get("frankensqlite") else {
            violations.push(format!(
                "{file_relpath}:{benchmark_group}: missing frankensqlite BENCH-META entry"
            ));
            continue;
        };

        if csqlite.comparisons.len() != 1 {
            violations.push(format!(
                "{file_relpath}:{benchmark_group}: csqlite BENCH-META mixes comparison modes: {}",
                describe_comparison_set(&csqlite.comparisons),
            ));
            continue;
        }

        if frankensqlite.comparisons.len() != 1 {
            violations.push(format!(
                "{file_relpath}:{benchmark_group}: frankensqlite BENCH-META mixes comparison modes: {}",
                describe_comparison_set(&frankensqlite.comparisons),
            ));
            continue;
        }

        if csqlite.comparisons != frankensqlite.comparisons {
            violations.push(format!(
                "{file_relpath}:{benchmark_group}: comparison-mode mismatch: csqlite={} frankensqlite={}",
                describe_comparison_set(&csqlite.comparisons),
                describe_comparison_set(&frankensqlite.comparisons),
            ));
            continue;
        }

        let csqlite_lifecycles = collect_lifecycles(&csqlite.dimensions);
        let frankensqlite_lifecycles = collect_lifecycles(&frankensqlite.dimensions);
        if csqlite_lifecycles != frankensqlite_lifecycles {
            violations.push(format!(
                "{file_relpath}:{benchmark_group}: lifecycle parity mismatch: csqlite={} frankensqlite={}",
                describe_lifecycle_set(&csqlite_lifecycles),
                describe_lifecycle_set(&frankensqlite_lifecycles),
            ));
            continue;
        }

        // Explicit arms (no `_ => {}` catch-all) preserve exhaustiveness
        // so future additions to BenchmarkComparisonMode force updates here.
        #[allow(clippy::collapsible_match)]
        match csqlite.comparisons.first() {
            Some(BenchmarkComparisonMode::Parity) => {
                if csqlite.dimensions != frankensqlite.dimensions {
                    violations.push(format!(
                        "{file_relpath}:{benchmark_group}: lifecycle/storage/concurrency parity mismatch: csqlite={} frankensqlite={}",
                        describe_dimension_set(&csqlite.dimensions),
                        describe_dimension_set(&frankensqlite.dimensions),
                    ));
                }
            }
            Some(BenchmarkComparisonMode::Control) => {
                if csqlite.dimensions == frankensqlite.dimensions {
                    violations.push(format!(
                        "{file_relpath}:{benchmark_group}: comparison=control is unnecessary because lifecycle/storage/concurrency already match ({})",
                        describe_dimension_set(&csqlite.dimensions),
                    ));
                }
            }
            None => {}
        }
    }

    violations
}

fn collect_lifecycles(dimensions: &BTreeSet<BenchmarkDimensions>) -> BTreeSet<String> {
    dimensions
        .iter()
        .map(|dimension| dimension.lifecycle.clone())
        .collect()
}

fn describe_comparison_set(entries: &BTreeSet<BenchmarkComparisonMode>) -> String {
    let mut description = String::new();
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            description.push_str(", ");
        }
        let _ = write!(
            description,
            "{}",
            match entry {
                BenchmarkComparisonMode::Parity => "parity",
                BenchmarkComparisonMode::Control => "control",
            }
        );
    }
    description
}

fn describe_lifecycle_set(entries: &BTreeSet<String>) -> String {
    entries.iter().cloned().collect::<Vec<_>>().join(", ")
}

fn describe_dimension_set(entries: &BTreeSet<BenchmarkDimensions>) -> String {
    let mut description = String::new();
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            description.push_str(", ");
        }
        let _ = write!(
            description,
            "{}/{}/{}",
            entry.lifecycle, entry.storage, entry.concurrency
        );
    }
    description
}

fn parse_benchmark_meta_records(
    relpath: &str,
    content: &str,
) -> Result<Vec<BenchmarkMetaRecord>, Vec<String>> {
    let lines: Vec<&str> = content.lines().collect();
    let mut index = 0_usize;
    let mut pending_meta = Vec::new();
    let mut records = Vec::new();
    let mut violations = Vec::new();

    while index < lines.len() {
        let trimmed = lines[index].trim();

        if let Some(raw_meta) = trimmed.strip_prefix("// BENCH-META:") {
            match parse_benchmark_meta_annotation(relpath, index + 1, raw_meta) {
                Ok(meta) => pending_meta.push(meta),
                Err(violation) => violations.push(violation),
            }
            index += 1;
            continue;
        }

        if let Some(function_name) = parse_benchmark_function_name(trimmed) {
            let function_line = index + 1;
            let mut body = String::from(lines[index]);
            body.push('\n');
            let mut brace_depth = brace_delta(lines[index]);
            index += 1;

            while index < lines.len() && brace_depth > 0 {
                body.push_str(lines[index]);
                body.push('\n');
                brace_depth += brace_delta(lines[index]);
                index += 1;
            }

            let has_benchmark_group = body.contains("benchmark_group(");
            if has_benchmark_group {
                if pending_meta.is_empty() {
                    violations.push(format!(
                        "{relpath}:{function_line}: benchmark function `{function_name}` missing BENCH-META block"
                    ));
                } else {
                    let benchmark_group =
                        parse_benchmark_group_name(&body).unwrap_or_else(|| function_name.clone());
                    for meta in std::mem::take(&mut pending_meta) {
                        let engine =
                            match normalize_bench_engine(relpath, meta.line_no, &meta.engine) {
                                Ok(engine) => Some(engine),
                                Err(violation) => {
                                    violations.push(violation);
                                    None
                                }
                            };
                        let lifecycle = match normalize_bench_dimension(
                            relpath,
                            meta.line_no,
                            "lifecycle",
                            &meta.lifecycle,
                            &["prepared", "ad_hoc"],
                        ) {
                            Ok(lifecycle) => Some(lifecycle),
                            Err(violation) => {
                                violations.push(violation);
                                None
                            }
                        };
                        let storage = match normalize_bench_dimension(
                            relpath,
                            meta.line_no,
                            "storage",
                            &meta.storage,
                            &["memory", "file"],
                        ) {
                            Ok(storage) => Some(storage),
                            Err(violation) => {
                                violations.push(violation);
                                None
                            }
                        };
                        let concurrency = match normalize_bench_dimension(
                            relpath,
                            meta.line_no,
                            "concurrency",
                            &meta.concurrency,
                            &["sequential", "concurrent"],
                        ) {
                            Ok(concurrency) => Some(concurrency),
                            Err(violation) => {
                                violations.push(violation);
                                None
                            }
                        };

                        if let (Some(engine), Some(lifecycle), Some(storage), Some(concurrency)) =
                            (engine, lifecycle, storage, concurrency)
                        {
                            records.push(BenchmarkMetaRecord {
                                file_relpath: relpath.to_owned(),
                                benchmark_group: benchmark_group.clone(),
                                engine,
                                dimensions: BenchmarkDimensions {
                                    lifecycle,
                                    storage,
                                    concurrency,
                                },
                                comparison: meta.comparison,
                            });
                        }
                    }
                }
            } else {
                pending_meta.clear();
            }
            continue;
        }

        if pending_meta.is_empty() || trimmed.is_empty() || trimmed.starts_with("#[") {
            index += 1;
            continue;
        }

        if !trimmed.starts_with("//") {
            violations.push(format!(
                "{relpath}:{}: dangling BENCH-META block not attached to a benchmark function",
                pending_meta[0].line_no
            ));
            pending_meta.clear();
        }
        index += 1;
    }

    if !pending_meta.is_empty() {
        violations.push(format!(
            "{relpath}:{}: dangling BENCH-META block not attached to a benchmark function",
            pending_meta[0].line_no
        ));
    }

    if violations.is_empty() {
        Ok(records)
    } else {
        Err(violations)
    }
}

fn parse_benchmark_meta_annotation(
    relpath: &str,
    line_no: usize,
    raw_meta: &str,
) -> Result<BenchmarkMetaAnnotation, String> {
    let mut fields = BTreeMap::new();

    for part in raw_meta.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            return Err(format!(
                "{relpath}:{line_no}: malformed BENCH-META entry `{trimmed}`"
            ));
        };
        fields.insert(key.trim().to_owned(), value.trim().to_owned());
    }

    let engine = fields
        .remove("engine")
        .ok_or_else(|| format!("{relpath}:{line_no}: BENCH-META missing `engine` field"))?;
    let lifecycle = fields
        .remove("lifecycle")
        .ok_or_else(|| format!("{relpath}:{line_no}: BENCH-META missing `lifecycle` field"))?;
    let storage = fields
        .remove("storage")
        .ok_or_else(|| format!("{relpath}:{line_no}: BENCH-META missing `storage` field"))?;
    let concurrency = fields
        .remove("concurrency")
        .ok_or_else(|| format!("{relpath}:{line_no}: BENCH-META missing `concurrency` field"))?;
    let comparison = match fields.remove("comparison").as_deref() {
        None | Some("parity") => BenchmarkComparisonMode::Parity,
        Some("control") => BenchmarkComparisonMode::Control,
        Some(other) => {
            return Err(format!(
                "{relpath}:{line_no}: unsupported BENCH-META comparison `{other}`"
            ));
        }
    };

    if !fields.is_empty() {
        let unexpected = fields.keys().cloned().collect::<Vec<_>>().join(", ");
        return Err(format!(
            "{relpath}:{line_no}: unsupported BENCH-META field(s): {unexpected}"
        ));
    }

    Ok(BenchmarkMetaAnnotation {
        engine,
        lifecycle,
        storage,
        concurrency,
        comparison,
        line_no,
    })
}

fn parse_benchmark_function_name(trimmed_line: &str) -> Option<String> {
    let name_part = trimmed_line.strip_prefix("fn bench_")?;
    let end = name_part.find('(')?;
    Some(format!("bench_{}", &name_part[..end]))
}

fn parse_benchmark_group_name(body: &str) -> Option<String> {
    let marker = "benchmark_group(";
    let start = body.find(marker)? + marker.len();
    let remainder = body[start..].trim_start();
    if let Some(remainder) = remainder.strip_prefix('"') {
        let end = remainder.find('"')?;
        return Some(remainder[..end].to_owned());
    }
    None
}

fn normalize_bench_engine(relpath: &str, line_no: usize, engine: &str) -> Result<String, String> {
    match engine {
        "csqlite" | "sqlite3" => Ok("csqlite".to_owned()),
        "frankensqlite" | "fsqlite" => Ok("frankensqlite".to_owned()),
        _ => Err(format!(
            "{relpath}:{line_no}: unsupported BENCH-META engine `{engine}`"
        )),
    }
}

fn normalize_bench_dimension(
    relpath: &str,
    line_no: usize,
    field: &str,
    value: &str,
    allowed: &[&str],
) -> Result<String, String> {
    if allowed.contains(&value) {
        Ok(value.to_owned())
    } else {
        Err(format!(
            "{relpath}:{line_no}: unsupported BENCH-META {field} `{value}`"
        ))
    }
}

fn brace_delta(line: &str) -> isize {
    let opens = line.chars().filter(|ch| *ch == '{').count() as isize;
    let closes = line.chars().filter(|ch| *ch == '}').count() as isize;
    opens - closes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("workspace root")
            .to_path_buf()
    }

    #[test]
    fn parse_benchmark_meta_records_extracts_group_and_normalizes_aliases() {
        let source = r#"
// BENCH-META: engine=sqlite3, lifecycle=prepared, storage=memory, concurrency=sequential
// BENCH-META: engine=fsqlite, lifecycle=prepared, storage=memory, concurrency=sequential
fn bench_demo(c: &mut Criterion) {
    let mut group = c.benchmark_group("demo_group");
    group.finish();
}
"#;

        let records =
            parse_benchmark_meta_records("benches/demo.rs", source).expect("metadata should parse");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].benchmark_group, "demo_group");
        assert_eq!(records[0].engine, "csqlite");
        assert_eq!(records[1].engine, "frankensqlite");
        assert_eq!(records[0].dimensions.storage, "memory");
        assert_eq!(records[1].dimensions.lifecycle, "prepared");
        assert_eq!(records[0].comparison, BenchmarkComparisonMode::Parity);
    }

    #[test]
    fn parse_benchmark_meta_records_supports_control_annotations() {
        let source = r#"
// BENCH-META: engine=csqlite, lifecycle=prepared, storage=file, concurrency=concurrent, comparison=control
// BENCH-META: engine=frankensqlite, lifecycle=prepared, storage=memory, concurrency=sequential, comparison=control
fn bench_demo(c: &mut Criterion) {
    let mut group = c.benchmark_group("demo_group");
    group.finish();
}
"#;

        let records =
            parse_benchmark_meta_records("benches/demo.rs", source).expect("metadata should parse");
        assert_eq!(records.len(), 2);
        assert!(
            records
                .iter()
                .all(|record| record.comparison == BenchmarkComparisonMode::Control)
        );
    }

    #[test]
    fn parse_benchmark_meta_records_rejects_unknown_fields() {
        let source = r#"
// BENCH-META: engine=csqlite, lifecycle=prepared, storage=memory, concurrency=sequential, unknown=value
// BENCH-META: engine=frankensqlite, lifecycle=prepared, storage=memory, concurrency=sequential
fn bench_demo(c: &mut Criterion) {
    let mut group = c.benchmark_group("demo_group");
    group.finish();
}
"#;

        let violations = parse_benchmark_meta_records("benches/demo.rs", source)
            .expect_err("unknown BENCH-META fields should fail");
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("unsupported BENCH-META field(s): unknown"));
    }

    #[test]
    fn parse_benchmark_meta_records_accumulates_dimension_violations() {
        let source = r#"
// BENCH-META: engine=nope, lifecycle=prepared, storage=memory, concurrency=sequential
// BENCH-META: engine=fsqlite, lifecycle=prepared, storage=disk, concurrency=sideways
fn bench_demo(c: &mut Criterion) {
    let mut group = c.benchmark_group("demo_group");
    group.finish();
}
"#;

        let violations = parse_benchmark_meta_records("benches/demo.rs", source)
            .expect_err("invalid metadata should fail");
        assert_eq!(violations.len(), 3);
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("unsupported BENCH-META engine `nope`"))
        );
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("unsupported BENCH-META storage `disk`"))
        );
        assert!(
            violations.iter().any(
                |violation| violation.contains("unsupported BENCH-META concurrency `sideways`")
            )
        );
    }

    #[test]
    fn validate_benchmark_metadata_records_allows_control_mode_storage_divergence() {
        let source = r#"
// BENCH-META: engine=csqlite, lifecycle=prepared, storage=file, concurrency=concurrent, comparison=control
// BENCH-META: engine=frankensqlite, lifecycle=prepared, storage=memory, concurrency=sequential, comparison=control
fn bench_demo(c: &mut Criterion) {
    let mut group = c.benchmark_group("demo_group");
    group.finish();
}
"#;

        let records =
            parse_benchmark_meta_records("benches/demo.rs", source).expect("metadata should parse");
        let violations = validate_benchmark_metadata_records(records);
        assert!(violations.is_empty(), "{violations:#?}");
    }

    #[test]
    fn validate_benchmark_metadata_records_rejects_parity_dimension_mismatch() {
        let source = r#"
// BENCH-META: engine=csqlite, lifecycle=prepared, storage=file, concurrency=concurrent
// BENCH-META: engine=frankensqlite, lifecycle=prepared, storage=memory, concurrency=sequential
fn bench_demo(c: &mut Criterion) {
    let mut group = c.benchmark_group("demo_group");
    group.finish();
}
"#;

        let records =
            parse_benchmark_meta_records("benches/demo.rs", source).expect("metadata should parse");
        let violations = validate_benchmark_metadata_records(records);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("lifecycle/storage/concurrency parity mismatch"));
    }

    #[test]
    fn parse_benchmark_source_paths_discovers_checked_in_bench_targets() {
        let bench_paths = discover_benchmark_source_files(&workspace_root())
            .expect("checked-in e2e Cargo.toml should parse");
        assert!(
            bench_paths
                .contains(&"crates/fsqlite-e2e/benches/concurrent_write_bench.rs".to_owned())
        );
        assert!(bench_paths.contains(&"crates/fsqlite-e2e/benches/e2e_bench.rs".to_owned()));
        assert!(bench_paths.len() >= 8);
    }

    #[test]
    fn checked_in_benchmark_metadata_has_lifecycle_parity() {
        let violations = validate_benchmark_lifecycle_parity(&workspace_root());
        assert!(
            violations.is_empty(),
            "checked-in benchmark metadata drifted: {violations:#?}"
        );
    }
}
