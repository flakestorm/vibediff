use std::{fs, path::PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use vibediff_core::{
    config::vibediff_config::VibeDiffConfig,
    git::{diff_parser::DiffParser, intent::IntentExtractor},
    report::{cli_reporter::print_cli_report, sarif_reporter::to_sarif},
    scorer::assertion_record::VibeLabel,
    scorer::vibe_scorer::VibeScorer,
};

#[derive(Parser, Debug)]
#[command(name = "vibediff", version, about = "Semantic intent auditor")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Audit a commit or staged diff for semantic alignment.
    Check {
        /// Commit or rev to analyze (default: HEAD).
        rev: Option<String>,
        /// Analyze staged changes instead of commit history.
        #[arg(long)]
        staged: bool,
        /// Output format: cli, json, entity-json, sarif.
        #[arg(long, value_enum, default_value = "cli")]
        format: OutputFormat,
        /// Minimum allowed composite score before policy failure.
        #[arg(long, default_value_t = 0.5)]
        min_score: f64,
        /// Continue on policy/runtime failure when true.
        #[arg(long, default_value_t = true)]
        fail_open: bool,
    },
    /// Install pre-commit hook for staged semantic checks.
    InstallHooks,
    /// Manage local analysis cache.
    Cache {
        #[command(subcommand)]
        command: CacheCommands,
    },
    /// Pre-analyze repository files to warm cache.
    WarmCache,
}

#[derive(Subcommand, Debug)]
enum CacheCommands {
    /// Remove all local cache entries.
    Clear,
    /// Keep only up to max_entries in local cache.
    Prune {
        /// Maximum number of entries to keep after pruning.
        #[arg(long, default_value_t = 10000)]
        max_entries: usize,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Cli,
    Json,
    EntityJson,
    Sarif,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = VibeDiffConfig::default();
    match cli.command {
        Commands::Check { rev, staged, format, min_score, fail_open } => {
            let repo_path = PathBuf::from(".");
            let parser = DiffParser::new(repo_path.clone());
            let diff_hunks = if staged {
                parser.extract_staged_diff_hunks()?
            } else {
                parser.extract_rev_diff_hunks(rev.as_deref().unwrap_or("HEAD"))?
            };
            let intent = IntentExtractor::new(repo_path).extract(rev.as_deref())?;
            let scorer = VibeScorer::new(config);
            if matches!(format, OutputFormat::EntityJson) {
                let entities = scorer.extract_entities(&diff_hunks).await?;
                println!("{}", serde_json::to_string_pretty(&entities)?);
                return Ok(());
            }
            let assertion = scorer.score(diff_hunks, intent).await?;
            match format {
                OutputFormat::Cli => print_cli_report(&assertion),
                OutputFormat::Json => {
                    let json = serde_json::to_string_pretty(&assertion)?;
                    println!("{json}");
                }
                OutputFormat::EntityJson => unreachable!(),
                OutputFormat::Sarif => {
                    println!("{}", serde_json::to_string_pretty(&to_sarif(&assertion))?);
                }
            }
            if assertion.composite_score < min_score {
                if fail_open {
                    eprintln!("warning: score below min threshold but fail-open enabled");
                } else {
                    if matches!(assertion.label, VibeLabel::Misaligned) {
                        anyhow::bail!(
                            "blocking: label MISALIGNED score {:.2} below {:.2}",
                            assertion.composite_score,
                            min_score
                        );
                    }
                    anyhow::bail!(
                        "score {:.2} below min threshold {:.2}",
                        assertion.composite_score,
                        min_score
                    );
                }
            }
        }
        Commands::InstallHooks => {
            let hook = r#"#!/usr/bin/env bash
set -euo pipefail
VIBEDIFF_MIN_SCORE="${VIBEDIFF_MIN_SCORE:-0.70}"
VIBEDIFF_FAIL_OPEN="${VIBEDIFF_FAIL_OPEN:-true}"
TMP_JSON="$(mktemp)"
if vibediff check --staged --format json --min-score "$VIBEDIFF_MIN_SCORE" --fail-open "$VIBEDIFF_FAIL_OPEN" > "$TMP_JSON"; then
  cat "$TMP_JSON"
  LABEL="$(python3 - "$TMP_JSON" <<'PY'
import json,sys
data=json.load(open(sys.argv[1], encoding='utf-8'))
print(data.get('label','UNKNOWN'))
PY
)"
  if [[ "$LABEL" == "MISALIGNED" ]]; then
    echo "🔴 VibeDiff: commit blocked (MISALIGNED)"
    exit 1
  elif [[ "$LABEL" == "SUSPECT" ]]; then
    echo "🟠 VibeDiff: suspect commit detected (policy blocks suspect by default)"
    exit 1
  fi
  exit 0
else
  if [[ "$VIBEDIFF_FAIL_OPEN" == "true" ]]; then
    echo "⚠️ VibeDiff failed but fail-open enabled. Continuing."
    exit 0
  fi
  echo "🔴 VibeDiff failed and fail-open disabled. Blocking."
  exit 1
fi
"#;
            let git_hooks = PathBuf::from(".git").join("hooks");
            fs::create_dir_all(&git_hooks)?;
            let target = git_hooks.join("pre-commit");
            fs::write(&target, hook)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&target)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&target, perms)?;
            }
            println!("Installed pre-commit hook at {}", target.display());
        }
        Commands::Cache { command } => {
            let scorer = VibeScorer::new(config);
            match command {
                CacheCommands::Clear => {
                    scorer.cache_clear()?;
                    println!("cache cleared");
                }
                CacheCommands::Prune { max_entries } => {
                    let removed = scorer.cache_prune(max_entries)?;
                    println!("pruned {} cache entries", removed);
                }
            }
        }
        Commands::WarmCache => {
            let scorer = VibeScorer::new(config);
            let warmed = scorer.warm_cache(PathBuf::from(".").as_path()).await?;
            println!("warmed cache entries for {} files", warmed);
        }
    }
    Ok(())
}
