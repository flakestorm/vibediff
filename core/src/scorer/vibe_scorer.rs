use std::{fs, path::PathBuf, sync::Arc};

use anyhow::Result;
use chrono::Utc;
use tokio::sync::Semaphore;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::{
    ast::{
        css_impact::CssImpactAnalyzer, dependency_graph::DependencyGraph, engine::AstEngine,
        entity_mapper::EntityChange,
    },
    cache::ast_cache::{AstCache, CacheKey},
    config::vibediff_config::VibeDiffConfig,
    git::{
        diff_parser::{DiffHunk, Language},
        intent::IntentRecord,
    },
    scorer::{
        assertion_record::{
            AssertionRecord, ConcernType, DimensionReasoning, DimensionScores, FlaggedEntity,
            VibeLabel,
        },
        llm_bridge::{LlmProvider, MockProvider, OllamaProvider},
        prompt_builder::{VIBE_SCORER_SYSTEM_PROMPT, build_user_prompt},
    },
};

pub struct VibeScorer {
    config: VibeDiffConfig,
    cache: Arc<AstCache>,
}

impl VibeScorer {
    pub fn new(config: VibeDiffConfig) -> Self {
        let cache = AstCache::new(PathBuf::from(".vibediff-cache")).unwrap_or_else(|_| {
            AstCache::new(std::env::temp_dir().join("vibediff-cache")).expect("cache init failed")
        });
        Self { config, cache: Arc::new(cache) }
    }

    pub async fn score(&self, hunks: Vec<DiffHunk>, intent: IntentRecord) -> Result<AssertionRecord> {
        let entities = self.extract_entities(&hunks).await?;
        let mut prompt = build_user_prompt(&intent, &entities);
        let downstream = DependencyGraph::build_from_repo(PathBuf::from(".").as_path())
            .find_downstream_impact(&entities);
        let css_index = CssImpactAnalyzer::build_from_repo(PathBuf::from(".").as_path());
        let mut css_impacts = Vec::new();
        for h in &hunks {
            let ext = h.file_path.extension().and_then(|x| x.to_str()).unwrap_or_default();
            if matches!(ext, "css" | "scss") {
                css_impacts.extend(css_index.analyze_css_hunk(h));
            }
        }
        if !downstream.is_empty() || !css_impacts.is_empty() {
            prompt.push_str("\n## DETECTED IMPACTS\n");
            prompt.push_str(&format!(
                "Downstream impacts: {} | CSS impacts: {}\n",
                downstream.len(),
                css_impacts.len()
            ));
            for d in downstream.iter().take(20) {
                prompt.push_str(&format!(
                    "- downstream: '{}' may impact '{}'\n",
                    d.changed_entity,
                    d.impacted_file.display()
                ));
            }
            for c in css_impacts.iter().take(20) {
                prompt.push_str(&format!(
                    "- css: '{}' referenced by '{}'\n",
                    c.css_entity,
                    c.impacted_file.display()
                ));
            }
        }

        // Try strict LLM scoring first; fallback to deterministic heuristic.
        let mut record = if let Ok(r) = self.score_with_llm(&intent, &prompt).await {
            r
        } else {
            self.score_heuristic(&intent, &entities)
        };
        record.commit_hash = intent.commit_hash;
        Ok(record)
    }

    fn score_heuristic(&self, _intent: &IntentRecord, entities: &[EntityChange]) -> AssertionRecord {
        let scores = self.compute_scores(entities);
        let composite_score = ((scores.composite() * 100.0).round()) / 100.0;
        let label = VibeLabel::from_score(composite_score);
        let flagged_entities = if entities.len() > 30 {
            vec![FlaggedEntity {
                entity_name: "bulk_change".to_string(),
                concern: ConcernType::DisproportionateChange,
                detail: "High entity count for single intent boundary.".to_string(),
            }]
        } else {
            vec![]
        };
        AssertionRecord {
            assertion_id: Uuid::new_v4(),
            model: self.config.local_model.clone(),
            commit_hash: String::new(),
            timestamp: Utc::now(),
            scores,
            composite_score,
            label,
            reasoning: DimensionReasoning {
                logic_match: "Heuristic score from entity/intent alignment.".to_string(),
                scope_adherence: "Scope based on changed file spread.".to_string(),
                side_effect_detection: "Side-effect penalty from detected patterns.".to_string(),
                structural_proportionality: "Entity count proportionality heuristic.".to_string(),
            },
            flagged_entities,
            suggested_commit_message: None,
        }
    }

    pub async fn extract_entities(&self, hunks: &[DiffHunk]) -> Result<Vec<EntityChange>> {
        let semaphore = Arc::new(Semaphore::new(8));
        let mut handles = Vec::new();
        let selected: Vec<&DiffHunk> = if hunks.len() > 100 {
            hunks.iter().step_by(3).collect()
        } else {
            hunks.iter().collect()
        };
        for h in selected {
            if is_excluded_path(&h.file_path) {
                continue;
            }
            if h.file_path.exists() && !matches!(h.language, Language::Unknown) {
                let key = CacheKey(format!("{}:{}", h.file_path.display(), h.raw_hunk));
                if let Some(cached) = self.cache.get(&key)? {
                    handles.push(tokio::spawn(async move { Ok::<Vec<EntityChange>, anyhow::Error>(cached) }));
                    continue;
                }
                let h_owned = h.clone();
                let cache = self.cache.clone();
                let permit = semaphore.clone().acquire_owned().await?;
                handles.push(tokio::spawn(async move {
                    let _permit = permit;
                    let src = fs::read_to_string(&h_owned.file_path)?;
                    let analyzed = tokio::task::spawn_blocking(move || {
                        AstEngine::analyze_file(
                            &h_owned.file_path,
                            &src,
                            std::slice::from_ref(&h_owned),
                            h_owned.language.clone(),
                        )
                    })
                    .await??;
                    cache.set(key, analyzed.clone())?;
                    Ok::<Vec<EntityChange>, anyhow::Error>(analyzed)
                }));
            }
        }
        let mut entities = Vec::new();
        for h in handles {
            entities.extend(h.await??);
        }
        Ok(entities)
    }

    fn compute_scores(&self, entities: &[EntityChange]) -> DimensionScores {
        let mut side_effect_hits = 0f64;
        for e in entities {
            side_effect_hits += e.side_effects.len() as f64;
        }
        let se = (1.0 - (side_effect_hits / 10.0)).clamp(0.0, 1.0);
        let scope = if entities.len() <= 20 { 0.9 } else { 0.6 };
        let logic = if entities.is_empty() { 0.5 } else { 0.85 };
        let sp = if entities.len() <= 30 { 0.9 } else { 0.6 };
        DimensionScores {
            logic_match: logic,
            scope_adherence: scope,
            side_effect_detection: se,
            structural_proportionality: sp,
        }
    }

    async fn score_with_llm(&self, intent: &IntentRecord, prompt: &str) -> Result<AssertionRecord> {
        let provider: Box<dyn LlmProvider> = if self.config.use_mock_llm {
            Box::new(MockProvider)
        } else {
            Box::new(OllamaProvider::new(
                self.config.ollama_url.clone(),
                self.config.local_model.clone(),
                self.config.timeout_secs,
            ))
        };
        let mut attempts = 0;
        while attempts < 3 {
            let raw = provider.complete(VIBE_SCORER_SYSTEM_PROMPT, prompt).await?;
            if let Ok(mut record) = parse_llm_assertion(raw.as_str(), &self.config.local_model) {
                record.commit_hash = intent.commit_hash.clone();
                return Ok(record);
            }
            attempts += 1;
        }
        anyhow::bail!("unable to parse llm assertion after retries")
    }
}

fn is_excluded_path(path: &std::path::Path) -> bool {
    let s = path.to_string_lossy();
    s.ends_with(".md")
        || s.contains("node_modules/")
        || s.contains("vendor/")
        || s.contains("__tests__/")
        || s.ends_with(".generated.ts")
        || s.ends_with(".pb.go")
}

fn parse_llm_assertion(raw: &str, model: &str) -> Result<AssertionRecord> {
    let clean = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    #[derive(serde::Deserialize)]
    struct RawScores {
        logic_match: f64,
        scope_adherence: f64,
        side_effect_detection: f64,
        structural_proportionality: f64,
    }
    #[derive(serde::Deserialize)]
    struct RawReasoning {
        logic_match: String,
        scope_adherence: String,
        side_effect_detection: String,
        structural_proportionality: String,
    }
    #[derive(serde::Deserialize)]
    struct RawFlaggedEntity {
        entity_name: String,
        concern: String,
        detail: String,
    }
    #[derive(serde::Deserialize)]
    struct RawAssertion {
        scores: RawScores,
        reasoning: RawReasoning,
        flagged_entities: Option<Vec<RawFlaggedEntity>>,
        suggested_commit_message: Option<String>,
    }
    let value: RawAssertion = serde_json::from_str(clean)?;
    let scores = DimensionScores {
        logic_match: value.scores.logic_match.clamp(0.0, 1.0),
        scope_adherence: value.scores.scope_adherence.clamp(0.0, 1.0),
        side_effect_detection: value.scores.side_effect_detection.clamp(0.0, 1.0),
        structural_proportionality: value.scores.structural_proportionality.clamp(0.0, 1.0),
    };
    let composite_score = ((scores.composite() * 100.0).round()) / 100.0;
    let label = VibeLabel::from_score(composite_score);
    Ok(AssertionRecord {
        assertion_id: Uuid::new_v4(),
        model: model.to_string(),
        commit_hash: String::new(),
        timestamp: Utc::now(),
        scores,
        composite_score,
        label,
        reasoning: DimensionReasoning {
            logic_match: value.reasoning.logic_match,
            scope_adherence: value.reasoning.scope_adherence,
            side_effect_detection: value.reasoning.side_effect_detection,
            structural_proportionality: value.reasoning.structural_proportionality,
        },
        flagged_entities: value.flagged_entities.unwrap_or_default().into_iter().map(|f| FlaggedEntity {
            entity_name: f.entity_name,
            concern: match f.concern.as_str() {
                "SCOPE_VIOLATION" => ConcernType::ScopeViolation,
                "UNDOCUMENTED_SIDE_EFFECT" => ConcernType::UndocumentedSideEffect,
                "DISPROPORTIONATE_CHANGE" => ConcernType::DisproportionateChange,
                _ => ConcernType::LogicMismatch,
            },
            detail: f.detail,
        }).collect(),
        suggested_commit_message: value.suggested_commit_message,
    })
}

impl VibeScorer {
    pub fn cache_clear(&self) -> Result<()> {
        self.cache.clear()
    }
    pub fn cache_prune(&self, max_entries: usize) -> Result<usize> {
        self.cache.prune(max_entries)
    }

    pub async fn warm_cache(&self, repo_root: &std::path::Path) -> Result<usize> {
        let mut count = 0usize;
        for entry in WalkDir::new(repo_root).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path().to_path_buf();
            if is_excluded_path(&path) {
                continue;
            }
            let lang = match path.extension().and_then(|x| x.to_str()).unwrap_or_default() {
                "ts" | "tsx" | "js" | "jsx" => Language::TypeScript,
                "rs" => Language::Rust,
                "py" => Language::Python,
                "go" => Language::Go,
                _ => Language::Unknown,
            };
            if matches!(lang, Language::Unknown) {
                continue;
            }
            let hunk = DiffHunk {
                file_path: path.clone(),
                language: lang,
                changed_lines: 1..=1,
                added_lines: vec![(1, "+warm_cache".to_string())],
                removed_lines: vec![],
                raw_hunk: "warm".to_string(),
            };
            let _ = self.extract_entities(&[hunk]).await?;
            count += 1;
        }
        Ok(count)
    }
}
