# VibeDiff

**Git-native semantic intent auditor. Catches the gap between what you said you changed and what you actually changed.**

[![Core CI](https://img.shields.io/github/actions/workflow/status/fhumarang/vibediff/core-ci.yml?branch=main&label=core%20ci&style=flat-square)](https://github.com/fhumarang/vibediff/actions/workflows/core-ci.yml)
[![Crates.io](https://img.shields.io/crates/v/vibediff_core?style=flat-square)](https://crates.io/crates/vibediff_core)
[![PyPI](https://img.shields.io/badge/pypi-vibediff-blue?style=flat-square)](https://pypi.org/project/vibediff/)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-green?style=flat-square)](docs/LICENSE.md)

---

## The Problem

You write `fix: prevent null pointer in user auth`.

Your AI assistant also quietly refactors the payment service, updates a shared config, and renames a CSS class.

Your reviewer sees green CI and approves.

That's **Agentic Drift** — and it compounds silently across every commit your AI coding tool makes. By the time you notice, the codebase does things nobody asked for, nobody reviewed, and nobody can trace.

**VibeDiff asks one question at every commit:**

> *"Do the structural changes in this diff actually match what the developer said they were doing?"*

---

## What It Does

VibeDiff parses your actual git diff using [tree-sitter](https://tree-sitter.github.io) AST analysis, extracts the semantic entities that changed (functions, classes, methods, modules), reads your commit message and PR body, sends only the **metadata** (never your source code) to a local or cloud LLM, and returns a structured **Vibe Score** with per-dimension reasoning.

```
git diff
   │
   ▼
┌─────────────────────┐
│  AST Parser         │  ← tree-sitter (TypeScript, Rust, Python, Go)
│  Entity Extractor   │
└──────────┬──────────┘
           │  EntityChange[]  (function name, change kind, side effects)
           ▼
┌─────────────────────┐
│  Intent Extractor   │  ← commit message, type, scope, PR body, ticket ref
└──────────┬──────────┘
           │  IntentRecord
           ▼
┌─────────────────────┐
│  Vibe Scorer        │  ← Ollama (local) / OpenAI / Anthropic / Gemini
│  LLM Bridge         │
└──────────┬──────────┘
           │  AssertionRecord (structured JSON)
           ▼
┌─────────────────────┐
│  Report Engine      │  ← CLI output / GitHub PR comment / SARIF
└─────────────────────┘
```

---

## See It In Action

### Pre-commit hook output

```
🔍 VibeDiff: analyzing staged changes...

╔══════════════════════════════════════╗
║        VibeDiff Audit Result         ║
╠══════════════════════════════════════╣
║  Score:  0.41    Label: MISALIGNED   ║
║  Flagged entities: 3                 ║
╚══════════════════════════════════════╝

⚠️  Flagged entities:
  • PaymentService.processCharge: SCOPE_VIOLATION — Entity appears outside stated commit scope (fix: null pointer in auth).
  • config/database.ts: UNDOCUMENTED_SIDE_EFFECT — Global config modified without mention in commit message.
  • AuthService.validateToken: LOGIC_MISMATCH — Null guard added, but method signature changed (undocumented refactor).

🔴 VibeDiff: commit blocked (score 0.41 < minimum 0.70)

💡 Suggested commit message:
   "fix(auth): add null guard to validateToken; refactor(payment): update processCharge signature"

   To bypass: VIBEDIFF_SKIP=1 git commit ...
```

### CLI check output (terminal)

```
$ vibediff check HEAD --format cli

VibeDiff Semantic Audit
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Commit:    a3f9c12  fix: prevent null pointer in user auth
Author:    Jane Dev
Timestamp: 2026-04-07T10:22:01Z

Scores
  Logic Match            0.82  ████████░░
  Scope Adherence        0.72  ███████░░░
  Side-Effect Detection  0.75  ███████░░░
  Structural Proportion  0.80  ████████░░
  ─────────────────────────────────────
  Composite              0.78  ⚠️  DRIFTING

Flagged Entities
  AuthService.validateToken
    → SCOPE_VIOLATION: Entity outside stated commit scope.

Suggested commit message:
  "fix(auth): add null guard to validateToken; minor scope drift detected"

✅ Audit complete — score above threshold, proceeding.
```

### GitHub PR comment (auto-posted)

```markdown
## 🔍 VibeDiff Semantic Audit

| Dimension              | Score |
|------------------------|-------|
| Logic Match            | 0.88  |
| Scope Adherence        | 0.91  |
| Side-Effect Detection  | 0.85  |
| Structural Proportion  | 0.90  |
| **Composite**          | **0.89** ✅ ALIGNED |

No flagged entities. This PR matches its stated intent.
```

### JSON output (for CI or tooling)

```json
{
  "assertion_id": "b3e1f2a0-...",
  "commit_hash": "a3f9c12",
  "model": "llama3.2:3b",
  "composite_score": 0.78,
  "label": "DRIFTING",
  "scores": {
    "logic_match": 0.82,
    "scope_adherence": 0.72,
    "side_effect_detection": 0.75,
    "structural_proportionality": 0.80
  },
  "reasoning": {
    "logic_match": "Null guard was added. Correct.",
    "scope_adherence": "PaymentService was touched outside the stated auth scope.",
    "side_effect_detection": "Minor config field modified without mention.",
    "structural_proportionality": "Change volume proportional."
  },
  "flagged_entities": [
    {
      "entity_name": "PaymentService.processCharge",
      "concern": "SCOPE_VIOLATION",
      "detail": "Entity appears outside stated commit scope."
    }
  ],
  "suggested_commit_message": "fix(auth): null guard + chore: minor payment service update"
}
```

---

## How Scoring Works

VibeDiff evaluates four independent dimensions, weighted into a composite **Vibe Score**:

| Dimension | Weight | What It Checks |
|---|---|---|
| **Logic Match** | 35% | Does the code logic match the stated intent? A `fix: null pointer` should show null guards added. |
| **Scope Adherence** | 30% | Are changed entities inside the expected scope? `feat(auth)` should not touch payment or config files. |
| **Side-Effect Detection** | 20% | Are there undocumented side effects? Global configs, CSS, shared modules touched without mention. |
| **Structural Proportionality** | 15% | Is the change volume proportional? A one-line fix that rewrites 3 files triggers scrutiny. |

```
VibeScore = (LM × 0.35) + (SA × 0.30) + (SE × 0.20) + (SP × 0.15)
```

| Score | Label | Meaning | CI Behavior |
|---|---|---|---|
| 0.85 – 1.00 | ✅ **ALIGNED** | Code matches intent | Pass |
| 0.70 – 0.84 | ⚠️ **DRIFTING** | Minor divergence detected | Warning (configurable to fail) |
| 0.50 – 0.69 | 🟠 **SUSPECT** | Significant unintended changes | Warning + full report |
| 0.00 – 0.49 | 🔴 **MISALIGNED** | Code contradicts stated intent | Fail (configurable) |

---

## Who This Is For

- **Individual developers** using AI coding assistants (Cursor, Copilot, Claude, Devin) who want a sanity check before pushing. Your AI is fast — but is it doing what you asked?
- **DevSecOps teams** who need semantic gates in CI/CD alongside linting and tests.
- **Engineering managers** who need an audit trail showing that every merged change matched its ticket.
- **Compliance-sensitive teams** (fintech, healthcare, defense) requiring semantic change attestation beyond structural diffs.

---

## Quick Install

```bash
pip install vibediff
```

Backed by [maturin](https://github.com/PyO3/maturin) wheel packaging — no Rust toolchain required.

Verify:

```bash
vibediff --version
```

> **Detailed OS-specific setup** (Windows, macOS, Linux), building from source, PATH configuration, and Ollama setup → [docs/USER_GUIDE.md](docs/USER_GUIDE.md)

---

## Model Provider Setup

VibeDiff works with your existing LLM setup. Pick one:

### Option 1 — Local (Ollama, default, fully private)

```bash
ollama pull llama3.2:3b
export VIBEDIFF_PROVIDER=ollama
export VIBEDIFF_OLLAMA_URL=http://localhost:11434
export VIBEDIFF_MODEL=llama3.2:3b
```

No data leaves your machine. Recommended for most developers.

### Option 2 — OpenAI

```bash
export VIBEDIFF_PROVIDER=openai
export OPENAI_API_KEY=sk-...
export VIBEDIFF_MODEL=gpt-4o-mini
```

### Option 3 — Anthropic

```bash
export VIBEDIFF_PROVIDER=anthropic
export ANTHROPIC_API_KEY=sk-ant-...
export VIBEDIFF_MODEL=claude-3-5-sonnet-latest
```

### Option 4 — Gemini

```bash
export VIBEDIFF_PROVIDER=gemini
export GEMINI_API_KEY=...
export VIBEDIFF_MODEL=gemini-1.5-flash
```

### Option 5 — Any OpenAI-compatible endpoint (OpenRouter, LM Studio, etc.)

```bash
export VIBEDIFF_PROVIDER=openai_compatible
export VIBEDIFF_API_BASE=https://openrouter.ai/api/v1
export VIBEDIFF_API_KEY=...
export VIBEDIFF_MODEL=meta-llama/llama-3.1-70b-instruct
```

---

## Usage

### Check staged changes before committing

```bash
vibediff check --staged --format cli
```

### Check the latest commit

```bash
vibediff check HEAD --format cli
```

### Check a specific commit by hash

```bash
vibediff check a3f9c12 --format cli
```

### Output as JSON (for CI scripting)

```bash
vibediff check HEAD --format json
```

### Output as SARIF (for GitHub Advanced Security / security tooling)

```bash
vibediff check HEAD --format sarif
```

### Output raw semantic entities (for debugging or tooling)

```bash
vibediff check HEAD --format entity-json
```

### Set a minimum score threshold

```bash
vibediff check HEAD --min-score 0.85 --format cli
```

### Install pre-commit hook (one-time setup per repo)

```bash
vibediff install-hooks
```

This writes `.git/hooks/pre-commit` — automatically runs `vibediff check --staged` before every commit. Configurable via environment variables:

```bash
export VIBEDIFF_MIN_SCORE=0.70    # score below this blocks the commit
export VIBEDIFF_FAIL_OPEN=true    # true = warn on LLM timeout; false = block
export VIBEDIFF_SKIP=1            # bypass the hook entirely for this commit
```

### Manage the local cache

```bash
vibediff cache clear                     # wipe all cached AST entries
vibediff cache prune --max-entries 5000  # keep only the most recent 5000 entries
vibediff warm-cache                      # pre-analyze repo files to speed up future checks
```

### Built-in help

```bash
vibediff --help
vibediff check --help
vibediff cache prune --help
```

---

## Pre-Commit Hook (Full Detail)

`vibediff install-hooks` writes this script to `.git/hooks/pre-commit`:

```bash
#!/usr/bin/env bash
# VibeDiff Pre-Commit Hook — installed by: vibediff install-hooks

VIBEDIFF_BIN="${VIBEDIFF_BIN:-vibediff}"
VIBEDIFF_TIMEOUT="${VIBEDIFF_TIMEOUT:-30}"
VIBEDIFF_FAIL_OPEN="${VIBEDIFF_FAIL_OPEN:-true}"
VIBEDIFF_MIN_SCORE="${VIBEDIFF_MIN_SCORE:-0.70}"

# Skip if VIBEDIFF_SKIP is set
if [[ -n "${VIBEDIFF_SKIP:-}" ]]; then
    echo "⚡ VibeDiff: skipped"
    exit 0
fi

echo "🔍 VibeDiff: analyzing staged changes..."

vibediff check \
    --staged \
    --format json \
    --output-file /tmp/vibediff_result.json

SCORE=$(jq -r '.composite_score' /tmp/vibediff_result.json)
LABEL=$(jq -r '.label' /tmp/vibediff_result.json)

# ... display score, block if below threshold
```

The hook is **fail-open by default** — a VibeDiff timeout or crash never blocks your commit.

---

## GitHub Actions Integration

Add `.github/workflows/vibediff-check.yml` to your repo:

```yaml
name: VibeDiff Semantic Audit

on:
  pull_request:
    types: [opened, synchronize, reopened]

permissions:
  contents: read
  pull-requests: write

jobs:
  vibediff-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install VibeDiff
        run: pip install vibediff

      - name: Run semantic audit
        run: |
          vibediff check \
            --format json \
            --output-file /tmp/vibediff_result.json

      - name: Generate and post PR comment
        run: |
          python3 .github/scripts/vibediff_comment.py \
            --result /tmp/vibediff_result.json \
            --output /tmp/comment.md

      - name: Post sticky PR comment
        uses: marocchino/sticky-pull-request-comment@v2
        with:
          path: /tmp/comment.md
          header: vibediff-audit

      - name: Enforce policy (optional)
        if: ${{ vars.VIBEDIFF_ENFORCE == 'true' }}
        run: |
          LABEL=$(jq -r '.label' /tmp/vibediff_result.json)
          if [[ "$LABEL" == "MISALIGNED" ]]; then
            echo "::error::VibeDiff: PR is MISALIGNED. Review flagged entities."
            exit 1
          fi
```

> Included as `.github/workflows/vibediff-check.yml`. Customize triggers, thresholds, and policy gates → [docs/CONFIGURATION_GUIDE.md](docs/CONFIGURATION_GUIDE.md)

---

## CI Gate (Script Example)

Block a PR merge programmatically based on Vibe Score:

```bash
vibediff check HEAD --format json > vibediff_result.json

python3 - <<'PY'
import json, sys
result = json.load(open("vibediff_result.json"))
label = result.get("label", "")
score = result.get("composite_score", 0)
print(f"Vibe Score: {score} ({label})")
if label in {"MISALIGNED", "SUSPECT"}:
    print("Flagged entities:")
    for e in result.get("flagged_entities", []):
        print(f"  • {e['entity_name']}: {e['detail']}")
    sys.exit(1)
print("Policy: PASS")
PY
```

---

## Architecture

VibeDiff is written in **Rust** with a tokio async runtime. Key design decisions:

| Principle | Implementation |
|---|---|
| **Local-first** | Full functionality offline. Ollama runs inference on-device. |
| **Privacy-preserving** | Source code never leaves the machine in OSS mode. Only entity metadata (names, types, change kinds) is sent to any LLM. |
| **Git-native** | Operates on raw diff hunks via `git2-rs`. Respects `.gitignore`. |
| **Zero-config** | `pip install vibediff && vibediff check HEAD` works immediately. |
| **Deterministic** | Same diff + same model + same prompt = same score (seeded temperature). |
| **Fail-open** | LLM timeout or error produces a warning, never an unintended hard block. |

**Languages supported:** TypeScript, Rust, Python, Go (via tree-sitter grammars)

**Cache:** Two-level — in-process `DashMap` (session) + persistent `sled` embedded DB (`~/.vibediff/cache/ast.db`)

**Latency targets:**

| Scenario | P50 | P99 |
|---|---|---|
| Cache hit | 80ms | 350ms |
| Cache miss, local Ollama | 2s | 6s |
| Cache miss, cloud LLM (GPT-4o-mini) | 500ms | 2s |

---

## OSS vs Cloud

| Feature | OSS (now) | Cloud (roadmap) |
|---|---|---|
| Local CLI semantic audit | ✅ | ✅ |
| Pre-commit hook | ✅ | ✅ |
| GitHub Actions integration | ✅ | ✅ |
| JSON / SARIF outputs | ✅ | ✅ |
| Ollama + BYO API key | ✅ | ✅ |
| Team dashboard & drift trends | — | ✅ |
| Policy Server (org-wide YAML rules) | — | ✅ |
| Centralized audit history | — | ✅ |
| GitHub App (no workflow setup needed) | — | ✅ |
| SAML SSO / on-prem deployment | — | ✅ Enterprise |
| Compliance export (SOC 2, HIPAA) | — | ✅ Enterprise |

The OSS core is the foundation. Cloud is additive — your local setup never breaks when cloud features ship.

**Waitlist / announcements:** [https://vibediff.dev](https://vibediff.dev)

---

## Documentation

| Document | Description |
|---|---|
| [User Guide](docs/USER_GUIDE.md) | Full installation (Windows/macOS/Linux), CLI reference, hook setup, troubleshooting |
| [Configuration Guide](docs/CONFIGURATION_GUIDE.md) | Workflow YAML customization, env vars, CI policy tuning, secrets |
| [Contributing](docs/CONTRIBUTING.md) | Dev setup, PR process, coding standards |
| [Issue Guide](docs/ISSUE_GUIDE.md) | How to report bugs, request features, labels and triage |
| [License](docs/LICENSE.md) | Apache 2.0 |

---

## Contributing

Read [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) before opening a PR.

For bugs: [.github/ISSUE_TEMPLATE/bug_report.md](.github/ISSUE_TEMPLATE/bug_report.md)  
For features: [.github/ISSUE_TEMPLATE/feature_request.md](.github/ISSUE_TEMPLATE/feature_request.md)

---

## License

Apache 2.0 — see [docs/LICENSE.md](docs/LICENSE.md).
