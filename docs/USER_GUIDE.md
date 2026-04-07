# VibeDiff User Guide

## 0. Installation and Environment Setup

### 0.1 One-command install availability

- `pip install vibediff`: not published yet
- `npm i -g vibediff`: not published yet
- `brew install vibediff`: planned
- `cargo install vibediff`: planned after crate publish

Current reliable path is source build with Rust.

### 0.2 Prerequisites

- `git`
- Rust toolchain (`rustc`, `cargo`)
- optional: Ollama for local scoring provider

### 0.3 Install prerequisites by OS

#### Windows (PowerShell)

```powershell
winget install Git.Git
winget install Rustlang.Rustup
git --version
rustc --version
cargo --version
```

#### macOS

```bash
brew install git rustup-init
rustup-init -y
source "$HOME/.cargo/env"
git --version
rustc --version
cargo --version
```

#### Linux (Debian/Ubuntu)

```bash
sudo apt update
sudo apt install -y git curl build-essential pkg-config libssl-dev
curl https://sh.rustup.rs -sSf | sh -s -- -y
source "$HOME/.cargo/env"
git --version
rustc --version
cargo --version
```

### 0.4 Build from source

```bash
git clone https://github.com/fhumarang/vibediff.git
cd vibediff/core
cargo build --release
```

Produced binary:

- Windows: `target\release\vibediff_core.exe`
- macOS/Linux: `target/release/vibediff_core`

### 0.5 Optional Ollama setup

```bash
ollama pull llama3.2:3b
```

Ensure endpoint availability:

```bash
curl http://localhost:11434
```

## 1. What VibeDiff Checks

VibeDiff evaluates semantic alignment of code changes against stated intent and reports:

- dimension scores
- composite score
- label: `ALIGNED | DRIFTING | SUSPECT | MISALIGNED`

## 2. CLI Commands

From `core/`:

### 2.0 Built-in command help (`--help`)

Use built-in CLI help to see command-specific options and explanations:

```bash
cargo run -- --help
cargo run -- check --help
cargo run -- cache --help
cargo run -- cache prune --help
```

If installed globally:

```bash
vibediff --help
vibediff check --help
vibediff cache --help
```

### 2.1 `check` (main audit command)

```bash
cargo run -- check HEAD --format cli
```

What it does:

- analyzes diff/intent
- computes alignment score
- prints terminal-friendly output

Example output (CLI mode):

```text
VibeDiff Audit Result
score: 0.78
label: Drifting
dimensions: logic=0.82 scope=0.72 side_effect=0.75 structural=0.80
```

JSON output:

```bash
cargo run -- check HEAD --format json
```

Example JSON fields:

- `composite_score`
- `label`
- `scores`
- `reasoning`
- `flagged_entities`

Raw semantic entities only:

```bash
cargo run -- check HEAD --format entity-json
```

Output:

- `EntityChange[]` records per detected semantic entity.

SARIF output:

```bash
cargo run -- check HEAD --format sarif
```

Output:

- SARIF JSON for security/reporting integrations (e.g., GitHub code scanning style tooling).

Staged-only audit:

```bash
cargo run -- check --staged --format cli
```

Use this before commit to audit exactly what is staged.

### 2.2 `install-hooks`

```bash
cargo run -- install-hooks
```

What it does:

- writes `.git/hooks/pre-commit`
- runs VibeDiff on staged changes during commit
- honors policy env vars like `VIBEDIFF_MIN_SCORE` and `VIBEDIFF_FAIL_OPEN`

### 2.3 Cache commands

Clear cache:

```bash
cargo run -- cache clear
```

Prune cache:

```bash
cargo run -- cache prune --max-entries 10000
```

Warm cache (pre-analyze repo files):

```bash
cargo run -- warm-cache
```

Example output:

```text
warmed cache entries for 143 files
```

## 3. Score Interpretation

- `0.85-1.00`: `ALIGNED`
- `0.70-0.84`: `DRIFTING`
- `0.50-0.69`: `SUSPECT`
- `<0.50`: `MISALIGNED`

Practical guidance:

- `ALIGNED`: safe default pass
- `DRIFTING`: review carefully; usually warning
- `SUSPECT`: likely scope/side-effect mismatch; block recommended
- `MISALIGNED`: hard block recommended

## 4. Hook Integration

`install-hooks` creates `.git/hooks/pre-commit` running staged checks.

Recommended policy:

- warn on `DRIFTING`
- block on `SUSPECT` or `MISALIGNED`

Example commit flow:

```bash
git add .
git commit -m "fix(auth): guard missing token"
```

If score falls below threshold with blocking policy, commit is rejected with VibeDiff output.

## 5. Local LLM Setup

1. Install Ollama
2. Pull model:

```bash
ollama pull llama3.2:3b
```

3. Ensure endpoint is available at `http://localhost:11434`.

## 6. CI Integration

Use the provided workflow baseline and run JSON mode for machine parsing:

```bash
cargo run -- check HEAD --format json
```

For raw semantic extraction only:

```bash
cargo run -- check HEAD --format entity-json
```

For full workflow customization details (secrets, triggers, inputs/outputs, policy gates), see:

- [docs/CONFIGURATION_GUIDE.md](CONFIGURATION_GUIDE.md)

### 6.1 Workflow Files Explained

For OSS usage, these are the relevant workflows:

- `.github/workflows/core-ci.yml`
  - Purpose: validate OSS Rust core (`cargo clippy`, `cargo test`, bench compile).
  - Trigger: push/PR changes touching `core/**`.
  - Output: CI status for OSS engine quality.

- `.github/workflows/vibediff-check.yml`
  - Purpose: semantic audit on PRs and sticky PR comment.
  - Trigger: PR events + manual `workflow_dispatch`.
  - Output: `vibediff_result.json` + markdown PR comment.

- `.github/workflows/python-release.yml`
  - Purpose: build/publish PyPI wheels using maturin on tags.
  - Trigger: version tags (`v*`) + manual dispatch.
  - Output: wheel artifacts and (if configured) PyPI release.

### 6.2 CI Inputs and Outputs (practical)

Inputs:

- git refs / PR branch
- model provider env vars (`VIBEDIFF_PROVIDER`, API keys, or Ollama URL/model)
- policy thresholds (`--min-score`, fail-open behavior)

Outputs:

- CLI text report
- JSON assertion record (`score`, `label`, `reasoning`, `flagged_entities`)
- SARIF (when `--format sarif` is used)
- PR markdown comment generated by `.github/scripts/vibediff_comment.py`

## 7. Troubleshooting

- **No diff detected**: ensure files are staged or valid commit ref exists.
- **LLM parse failure**: VibeDiff retries; if all retries fail it falls back to deterministic heuristic.
- **Ollama unavailable**: start Ollama or set mock mode in config for local dev.
- **`cargo` not found**: install Rust via rustup and restart shell session.
- **Binary not found globally**: run using full path (`target/release/...`) or add release folder to PATH.
