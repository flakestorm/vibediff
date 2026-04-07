#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--result", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    data = json.loads(Path(args.result).read_text(encoding="utf-8"))
    score = data.get("composite_score", 0.0)
    label = data.get("label", "UNKNOWN")
    reasoning = data.get("reasoning", {})
    flagged = data.get("flagged_entities", [])

    lines = [
        "## VibeDiff Semantic Audit",
        "",
        f"- Score: `{score}`",
        f"- Label: `{label}`",
        "",
        "### Reasoning",
        f"- Logic match: {reasoning.get('logic_match', 'n/a')}",
        f"- Scope adherence: {reasoning.get('scope_adherence', 'n/a')}",
        f"- Side effects: {reasoning.get('side_effect_detection', 'n/a')}",
        f"- Structural proportionality: {reasoning.get('structural_proportionality', 'n/a')}",
        "",
    ]
    if flagged:
        lines.append("### Flagged entities")
        for f in flagged:
            lines.append(f"- `{f.get('entity_name', 'unknown')}`: {f.get('concern', 'N/A')} - {f.get('detail', '')}")
    else:
        lines.append("No flagged entities.")

    Path(args.output).write_text("\n".join(lines) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
