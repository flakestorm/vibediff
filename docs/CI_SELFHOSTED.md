# Self-hosted CI with VibeDiff (BYO LLM key)

Use this guide when you run **VibeDiff in your own GitHub Actions** (or other CI) with **your** infrastructure and **your** model provider keys. Nothing here requires the VibeDiff GitHub App or paid cloud.

For org-wide PR enforcement without maintaining workflow YAML, see the [README](../README.md) **CI Integration** section ([VibeDiff GitHub App](https://vibediff.dev/app)).

---

## Prerequisites

- Install the `vibediff` CLI in the job (for example `pip install vibediff` after wheels are published, or build from `core/` in this repo).
- Configure LLM access via the same environment variables as local development ([User Guide — Model providers](USER_GUIDE.md)).
- Keep API keys in GitHub **Secrets**, not in the workflow file.

---

## Example: GitHub Actions on pull requests

Add a workflow under `.github/workflows/` in **your** repository. Adjust triggers, branches, and `vibediff check` arguments to match how you diff PRs (for example `HEAD` vs merge base).

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
        env:
          VIBEDIFF_PROVIDER: openai
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
          VIBEDIFF_MODEL: gpt-4o-mini
        run: |
          vibediff check HEAD --format json > vibediff_result.json

      - name: Generate PR comment
        run: |
          python3 .github/scripts/vibediff_comment.py \
            --result vibediff_result.json \
            --output vibediff_comment.md

      - name: Post sticky PR comment
        uses: marocchino/sticky-pull-request-comment@v2
        with:
          path: vibediff_comment.md
          header: vibediff-audit

      - name: Enforce policy (optional)
        if: ${{ vars.VIBEDIFF_ENFORCE == 'true' }}
        run: |
          LABEL=$(jq -r '.label' vibediff_result.json)
          if [[ "$LABEL" == "MISALIGNED" ]]; then
            echo "::error::VibeDiff: PR is MISALIGNED. Review flagged entities."
            exit 1
          fi
```

Copy [`.github/scripts/vibediff_comment.py`](../.github/scripts/vibediff_comment.py) into your repo (same path or adjust the `python3` step). The script turns JSON output from `vibediff check` into Markdown for the sticky comment action.

---

## CI gate without PR comments

Block a job from succeeding based on label/score:

```bash
vibediff check HEAD --format json > vibediff_result.json

python3 - <<'PY'
import json, sys
result = json.load(open("vibediff_result.json", encoding="utf-8"))
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

## This monorepo’s reference workflow

The **vibediff** project itself builds the CLI from `core/` with Cargo and runs the same comment script. See:

- [`.github/workflows/vibediff-check.yml`](../.github/workflows/vibediff-check.yml)

Workflow customization (triggers, Rust vs pip install, secrets) is covered in [Configuration Guide](CONFIGURATION_GUIDE.md).
