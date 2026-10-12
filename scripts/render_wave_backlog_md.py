#!/usr/bin/env python3
"""Generates docs/WAVE2_BACKLOG.md from docs/WAVE2_BACKLOG.json.

This file is generated, not hand-maintained. Re-run after any edit to the
JSON:

    python3 scripts/render_wave_backlog_md.py
"""
import json
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
JSON_PATH = ROOT / "docs" / "WAVE2_BACKLOG.json"
MD_PATH = ROOT / "docs" / "WAVE2_BACKLOG.md"


def render(items: list[dict]) -> str:
    tally = Counter(it["complexity"] for it in items)
    points = sum(it["points"] for it in items)
    by_area = Counter(it["area"] for it in items)

    lines = []
    lines.append("<!-- GENERATED FILE — do not hand-edit.")
    lines.append("     Source of truth is docs/WAVE2_BACKLOG.json.")
    lines.append("     Regenerate with: python3 scripts/render_wave_backlog_md.py -->")
    lines.append("")
    lines.append("# Wave 2 backlog — sorocharge-core")
    lines.append("")
    lines.append(
        f"**{len(items)} issues, {points} points** — "
        f"{tally['high']} high (200 pts each), "
        f"{tally['medium']} medium (150 pts each), "
        f"{tally['trivial']} trivial (100 pts each)."
    )
    lines.append("")
    lines.append("By area: " + ", ".join(f"`{a}` ({n})" for a, n in sorted(by_area.items())))
    lines.append("")
    lines.append(
        "Every item below cites a specific file and gap found by reading "
        "this repository during this wave's survey, not a generic roadmap "
        "item. Complexity labels were assigned to hit an exact point "
        "quota per explicit maintainer instruction — they do not all "
        "correspond to real relative effort size."
    )
    lines.append("")

    for complexity, label in [("high", "High — 200 points"), ("medium", "Medium — 150 points"), ("trivial", "Trivial — 100 points")]:
        group = [it for it in items if it["complexity"] == complexity]
        lines.append(f"## {label} ({len(group)})")
        lines.append("")
        for it in group:
            gfi = " `good first issue`" if it["good_first_issue"] else ""
            lines.append(f"### {it['title']}")
            lines.append("")
            lines.append(f"`area:{it['area']}`{gfi}")
            lines.append("")
            lines.append(f"**Description.** {it['description']}")
            lines.append("")
            lines.append(f"**Why.** {it['why']}")
            lines.append("")
            lines.append("**Acceptance criteria.**")
            for c in it["criteria"]:
                lines.append(f"- [ ] {c}")
            lines.append("")
            lines.append("**Guidance.**")
            for g in it["guidance"]:
                lines.append(f"- {g}")
            if it["entry_point"]:
                lines.append("")
                lines.append(f"**Entry point.** `{it['entry_point']}`")
            lines.append("")
    return "\n".join(lines)


def main() -> int:
    items = json.loads(JSON_PATH.read_text())
    MD_PATH.write_text(render(items))
    print(f"wrote {MD_PATH} from {len(items)} issues in {JSON_PATH}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
