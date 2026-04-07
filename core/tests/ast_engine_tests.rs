use vibediff_core::{
    ast::engine::AstEngine,
    git::diff_parser::{DiffHunk, Language},
};

#[test]
fn typescript_ast_analysis_returns_entities() {
    let src = "function hello(){ return fetch('/x') }\n";
    let h = DiffHunk {
        file_path: "example.ts".into(),
        language: Language::TypeScript,
        changed_lines: 1..=1,
        added_lines: vec![(1, "+function hello(){ return fetch('/x') }".to_string())],
        removed_lines: vec![],
        raw_hunk: "@@".to_string(),
    };
    let entities = AstEngine::analyze_typescript_file("example.ts".as_ref(), src, &[h]).unwrap();
    assert!(!entities.is_empty());
}
