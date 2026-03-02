#[test]
fn test_count_zero_args() {
    let sql = "SELECT count();";
    let stmt = crate::parser::Parser::from_sql(sql).parse_all().unwrap();
    let mut schema = Schema::new();
    let mut analyzer = SemanticAnalyzer::new(&schema);
    let mut scope = Scope::new();
    analyzer.analyze_statement(&stmt.into_iter().next().unwrap(), &mut scope);
    assert!(analyzer.errors.is_empty(), "expected no errors, got {:?}", analyzer.errors);
}
