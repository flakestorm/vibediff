use std::{fs, path::PathBuf};

use criterion::{Criterion, criterion_group, criterion_main};
use vibediff_core::{
    config::vibediff_config::VibeDiffConfig,
    git::diff_parser::{DiffHunk, Language},
    scorer::vibe_scorer::VibeScorer,
};

fn bench_extract_entities(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let scorer = VibeScorer::new(VibeDiffConfig::default());
    let hunks = vec![DiffHunk {
        file_path: "src/main.ts".into(),
        language: Language::TypeScript,
        changed_lines: 1..=1,
        added_lines: vec![(1, "+function test() { return fetch('/x') }".to_string())],
        removed_lines: vec![],
        raw_hunk: "@@ -1 +1 @@".to_string(),
    }];
    c.bench_function("extract_entities_small", |b| {
        b.iter(|| {
            let _ = rt.block_on(async { scorer.extract_entities(&hunks).await });
        });
    });

    let bench_dir: PathBuf = std::env::temp_dir().join("vibediff-bench-files");
    let _ = fs::create_dir_all(&bench_dir);
    let many: Vec<DiffHunk> = (0..50)
        .map(|i| {
            let file_path = bench_dir.join(format!("file_{i}.ts"));
            let _ = fs::write(&file_path, format!("function test_{i}() {{ return {i}; }}"));
            DiffHunk {
            file_path,
            language: Language::TypeScript,
            changed_lines: 1..=1,
            added_lines: vec![(1, format!("+function test_{i}() {{ return fetch('/x') }}"))],
            removed_lines: vec![],
            raw_hunk: format!("@@ -1 +1 file_{i} @@"),
        }})
        .collect();
    // Prime cache once
    let _ = rt.block_on(async { scorer.extract_entities(&many).await });
    c.bench_function("extract_entities_50_files_cache_hit", |b| {
        b.iter(|| {
            let _ = rt.block_on(async { scorer.extract_entities(&many).await });
        });
    });
}

criterion_group!(benches, bench_extract_entities);
criterion_main!(benches);
