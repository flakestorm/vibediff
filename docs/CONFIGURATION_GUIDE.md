# VibeDiff Configuration Guide

This guide explains what you can configure in VibeDiff for local usage and CI/CD workflows.

## 1. What Is Configurable

You are expected to customize:

- workflow triggers and branch filters
- scoring thresholds and fail-open/fail-closed behavior
- model provider and model ID
- API keys/secrets
- output formats (`cli`, `json`, `sarif`)
- PR comment behavior

## 2. Runtime Configuration

Common environment variables:

- `VIBEDIFF_PROVIDER` (`ollama`, `openai`, `anthropic`, `gemini`, `openai_compatible`)
- `VIBEDIFF_MODEL` (model name/id)
- `VIBEDIFF_OLLAMA_URL` (default `http://localhost:11434`)
- `VIBEDIFF_API_BASE` (for openai-compatible endpoints)
- `VIBEDIFF_API_KEY`
- `OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `GEMINI_API_KEY`

Policy and behavior:

- `VIBEDIFF_MIN_SCORE` (used by hook/pipeline policy)
- `VIBEDIFF_FAIL_OPEN` (`true` or `false`)

## 3. GitHub Workflows Explained

### Workflow customization policy

What should usually be customized:

- triggers (`on: push`, `pull_request`, branches, paths)
- runtime versions (Rust/Node/Python)
- commands/flags (`vibediff check ...`, thresholds, format)
- environment vars and secrets (`PYPI_API_TOKEN`, model/provider keys)
- gating policy (when to fail vs warn)
- artifact/report paths

What should stay stable unless intentional:

- required permissions for posting PR comments
- core step order (checkout -> build -> run audit -> generate/post outputs)
- output contract assumptions (JSON fields consumed by scripts)

Workflows are templates, not immutable. Modify them for your team policy and pipeline standards.

### `.github/workflows/core-ci.yml`

- Purpose: OSS core quality checks
- Typical steps: checkout, Rust toolchain, clippy, tests, bench compile
- Safe to modify: Rust version, steps, trigger paths

Concrete customizations:

```yaml
# Run core CI only on main and release branches
on:
  push:
    branches: [main, "release/*"]
    paths:
      - "core/**"
```

```yaml
# Pin Rust toolchain version instead of stable
- uses: dtolnay/rust-toolchain@stable
  with:
    toolchain: "1.82.0"
```

```yaml
# Add extra quality gates
- run: cargo fmt --all -- --check
- run: cargo clippy -- -D warnings
- run: cargo test --all-features
```

### `.github/workflows/vibediff-check.yml`

- Purpose: run semantic audit on PRs and post sticky PR comment
- Typical steps: checkout, build, run `vibediff check --format json`, generate markdown comment, post comment
- Safe to modify: trigger branches/types, audit flags, policy gate logic, comment formatting

Concrete customizations:

```yaml
# Trigger only on PRs into main
on:
  pull_request:
    branches: [main]
    types: [opened, synchronize, reopened, ready_for_review]
```

```yaml
# Add strict policy thresholds directly in command
- name: Run VibeDiff audit
  run: cargo run -- check HEAD --format json --min-score 0.70 --fail-open false > /tmp/vibediff_result.json
```

```yaml
# Gate by label in CI step
- name: Enforce policy gate
  run: |
    python3 - <<'PY'
    import json
    data = json.load(open('/tmp/vibediff_result.json', encoding='utf-8'))
    if data.get('label') in {'MISALIGNED', 'SUSPECT'}:
        raise SystemExit('VibeDiff policy block')
    print('VibeDiff pass')
    PY
```

```yaml
# Configure provider through environment variables (example: OpenAI)
- name: Run VibeDiff audit
  env:
    VIBEDIFF_PROVIDER: openai
    OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
    VIBEDIFF_MODEL: gpt-4o-mini
  run: cargo run -- check HEAD --format json > /tmp/vibediff_result.json
```

Required permissions for PR comments:

```yaml
permissions:
  contents: read
  pull-requests: write
```

### `.github/workflows/python-release.yml`

- Purpose: build/publish Python wheels (maturin)
- Trigger: tags (`v*`) and manual dispatch
- Requires secret: `PYPI_API_TOKEN`
- Safe to modify: supported Python versions, OS matrix, publish conditions

Concrete customizations:

```yaml
# Restrict publish to semantic version tags only
on:
  push:
    tags:
      - "v[0-9]+.[0-9]+.[0-9]+"
```

```yaml
# Limit Python versions for wheel build
with:
  command: build
  args: --release --manifest-path core/Cargo.toml --interpreter 3.10 3.11 3.12 --out dist
```

```yaml
# Publish condition guard (only from main repo)
if: startsWith(github.ref, 'refs/tags/v') && github.repository == 'fhumarang/vibediff'
```

Secrets needed:

- `PYPI_API_TOKEN` (required for upload)
- optional signing/notary secrets if you later add artifact signing

### End-to-end configuration examples

#### Example A: Fast feedback mode (non-blocking)

- `--fail-open true`
- do not block CI on `DRIFTING` or `SUSPECT`
- still post PR comment

```yaml
- run: cargo run -- check HEAD --format json --min-score 0.50 --fail-open true > /tmp/vibediff_result.json
```

#### Example B: Strict compliance mode (blocking)

- `--fail-open false`
- block on `SUSPECT` and `MISALIGNED`
- enforce provider and model centrally in CI env

```yaml
- env:
    VIBEDIFF_PROVIDER: anthropic
    ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
    VIBEDIFF_MODEL: claude-3-5-sonnet-latest
  run: cargo run -- check HEAD --format json --min-score 0.80 --fail-open false > /tmp/vibediff_result.json
```

## 4. Workflow Inputs and Outputs

Inputs:

- commit/PR diff context
- provider credentials and model selection
- threshold policy settings

Outputs:

- CLI audit summary
- JSON assertion record
- SARIF report (if enabled)
- PR comment markdown

## 5. Real Pipeline Example

```yaml
name: vibediff-gate
on:
  pull_request:
    types: [opened, synchronize, reopened]
jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --release
        working-directory: core
      - run: cargo run -- check HEAD --format json > /tmp/vibediff_result.json
        working-directory: core
      - run: |
          python - <<'PY'
          import json
          d=json.load(open('/tmp/vibediff_result.json', encoding='utf-8'))
          if d.get('label') in {'MISALIGNED','SUSPECT'}:
              raise SystemExit('VibeDiff policy block')
          print('VibeDiff pass')
          PY
```

## 6. What Not To Break

- required GitHub permissions for PR comment posting
- JSON schema fields consumed by comment/gating scripts
- release-tag convention used by publish workflows
