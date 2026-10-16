#!/usr/bin/env python3
"""One-off: adds the low/medium/high complexity label to every Wave 2
backlog issue already created on GitHub, matched by title against the
backlog JSON. The creation script didn't apply this label originally —
see scripts/create_wave_issues.py's label_for(), now fixed to include it
going forward.

Usage: python3 scripts/backfill_complexity_labels.py docs/WAVE2_BACKLOG.json [--apply]
"""
import argparse
import json
import subprocess
import sys
import time
from pathlib import Path

LABEL_FOR_COMPLEXITY = {"high": "high", "medium": "medium", "trivial": "low"}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("backlog_json")
    parser.add_argument("--apply", action="store_true")
    parser.add_argument("--repo", default="sorocharge/sorocharge-core")
    args = parser.parse_args()

    items = json.loads(Path(args.backlog_json).read_text())
    title_to_complexity = {it["title"]: it["complexity"] for it in items}

    result = subprocess.run(
        ["gh", "issue", "list", "--repo", args.repo, "--state", "all", "--limit", "1000",
         "--json", "number,title,labels"],
        capture_output=True, text=True, check=True,
    )
    live = json.loads(result.stdout)

    to_fix = []
    for issue in live:
        complexity = title_to_complexity.get(issue["title"])
        if complexity is None:
            continue  # not a backlog issue (e.g. prior-wave issue)
        want = LABEL_FOR_COMPLEXITY[complexity]
        have = {l["name"] for l in issue["labels"]}
        if want not in have:
            to_fix.append((issue["number"], issue["title"], want))

    print(f"{len(to_fix)} backlog issues missing their complexity label out of "
          f"{sum(1 for i in live if i['title'] in title_to_complexity)} matched.")

    if not args.apply:
        for num, title, label in to_fix[:10]:
            print(f"  would add '{label}' to #{num}: {title}")
        if len(to_fix) > 10:
            print(f"  ... and {len(to_fix) - 10} more")
        print("\nDry run — re-run with --apply to actually add labels.")
        return 0

    fixed = 0
    for num, title, label in to_fix:
        r = subprocess.run(
            ["gh", "issue", "edit", str(num), "--repo", args.repo, "--add-label", label],
            capture_output=True, text=True,
        )
        if r.returncode != 0:
            print(f"FAILED on #{num} ({title}): {r.stderr}", file=sys.stderr)
            print(f"{fixed} fixed before this failure.", file=sys.stderr)
            return 1
        fixed += 1
        print(f"[{fixed}/{len(to_fix)}] #{num} -> added '{label}'")
        time.sleep(1.0)

    print(f"\nDone. {fixed} issue(s) labeled.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
