#!/usr/bin/env python3
"""Creates GitHub issues for a wave backlog from its JSON file.

Usage:
    python3 scripts/create_wave_issues.py docs/WAVE2_BACKLOG.json [--apply]

Dry-run by default — prints what it would do and exits 0 without touching
GitHub. Pass --apply to actually create issues.

Safe to re-run: before creating anything, it reads every existing issue
title in the target repo (open and closed) and skips any backlog entry
whose title already exists, since `gh issue create` is not idempotent and
a re-run would otherwise duplicate the whole backlog.

On a failure partway through, it reports the title it died on and how many
issues were created before stopping, then exits non-zero — re-running the
same command resumes from there, since already-created titles are skipped.
"""
import argparse
import json
import subprocess
import sys
import time
from pathlib import Path

REPO = "sorocharge/sorocharge-core"
PACE_SECONDS = 1.5

REQUIRED_FIELDS = ["title", "complexity", "points", "area", "description", "why", "criteria", "guidance"]
VALID_COMPLEXITY = {"high": 200, "medium": 150, "trivial": 100}


def validate(items: list[dict]) -> list[str]:
    errors = []
    seen_titles = set()
    for i, it in enumerate(items):
        for f in REQUIRED_FIELDS:
            if f not in it or it[f] in (None, "", []):
                errors.append(f"item {i}: missing or empty field '{f}'")
        title = it.get("title")
        if title:
            if title in seen_titles:
                errors.append(f"duplicate title in backlog: {title!r}")
            seen_titles.add(title)
        complexity = it.get("complexity")
        if complexity not in VALID_COMPLEXITY:
            errors.append(f"item {i} ({title!r}): invalid complexity {complexity!r}")
        elif it.get("points") != VALID_COMPLEXITY[complexity]:
            errors.append(
                f"item {i} ({title!r}): points {it.get('points')!r} doesn't match "
                f"complexity {complexity!r} (expected {VALID_COMPLEXITY[complexity]})"
            )
        criteria = it.get("criteria") or []
        if not (3 <= len(criteria) <= 5):
            errors.append(f"item {i} ({title!r}): criteria count {len(criteria)} not in [3,5]")
        guidance = it.get("guidance") or []
        if not (2 <= len(guidance) <= 4):
            errors.append(f"item {i} ({title!r}): guidance count {len(guidance)} not in [2,4]")
        if complexity == "trivial":
            if not it.get("good_first_issue"):
                errors.append(f"item {i} ({title!r}): trivial issue missing good_first_issue:true")
            if not it.get("entry_point"):
                errors.append(f"item {i} ({title!r}): trivial issue missing entry_point")
        else:
            if it.get("good_first_issue"):
                errors.append(f"item {i} ({title!r}): good_first_issue:true on a non-trivial issue")
    return errors


def existing_titles(repo: str) -> set[str]:
    result = subprocess.run(
        ["gh", "issue", "list", "--repo", repo, "--state", "all", "--limit", "1000", "--json", "title"],
        capture_output=True, text=True, check=True,
    )
    return {row["title"] for row in json.loads(result.stdout)}


def label_for(it: dict) -> list[str]:
    labels = []
    type_label = {
        "fix": "bug", "test": "enhancement", "feat": "enhancement",
        "docs": "documentation", "chore": "enhancement", "refactor": "enhancement",
        "perf": "enhancement", "ci": "enhancement", "research": "question",
    }.get(it["title"].split(":")[0].split("(")[0], "enhancement")
    labels.append(type_label)
    # Complexity label: GitHub's own "good first issue" already exists;
    # "high"/"medium"/"low" do not by default and must be created once
    # per repo (see scripts/backfill_complexity_labels.py, which also
    # backfills this onto any issue created before this label existed).
    labels.append({"high": "high", "medium": "medium", "trivial": "low"}[it["complexity"]])
    if it.get("good_first_issue"):
        labels.append("good first issue")
    return labels


def body_for(it: dict) -> str:
    lines = [
        f"<!-- wave-backlog: {it['complexity']} / {it['points']} points / area:{it['area']} -->",
        "",
        "## Summary",
        it["description"],
        "",
        "## Why it matters",
        it["why"],
        "",
        "## Acceptance Criteria",
    ]
    lines += [f"- [ ] {c}" for c in it["criteria"]]
    lines += ["", "## Guidance"]
    lines += [f"- {g}" for g in it["guidance"]]
    if it.get("entry_point"):
        lines += ["", f"**Entry point:** `{it['entry_point']}`"]
    return "\n".join(lines)


def ensure_complexity_labels_exist(repo: str) -> None:
    """high/medium/low aren't in GitHub's default label set — create them
    once if missing, so --label doesn't fail on a repo that's never run
    this script before."""
    colors = {"high": "d93f0b", "medium": "fbca04", "low": "0e8a16"}
    result = subprocess.run(
        ["gh", "label", "list", "--repo", repo, "--limit", "200", "--json", "name"],
        capture_output=True, text=True, check=True,
    )
    existing = {row["name"] for row in json.loads(result.stdout)}
    for name, color in colors.items():
        if name not in existing:
            subprocess.run(
                ["gh", "label", "create", name, "--repo", repo, "--color", color,
                 "--description", f"{name.capitalize()} complexity"],
                capture_output=True, text=True, check=True,
            )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("backlog_json")
    parser.add_argument("--apply", action="store_true", help="actually create issues (default: dry-run)")
    parser.add_argument("--repo", default=REPO)
    args = parser.parse_args()

    items = json.loads(Path(args.backlog_json).read_text())
    errors = validate(items)
    if errors:
        print(f"REFUSING to touch GitHub — {len(errors)} validation error(s):", file=sys.stderr)
        for e in errors[:50]:
            print(f"  - {e}", file=sys.stderr)
        return 1

    print(f"Validated {len(items)} issues ({sum(it['points'] for it in items)} points) against {args.repo}.")

    if args.apply:
        ensure_complexity_labels_exist(args.repo)

    try:
        already = existing_titles(args.repo)
    except subprocess.CalledProcessError as e:
        print(f"Failed to list existing issues: {e.stderr}", file=sys.stderr)
        return 1

    to_create = [it for it in items if it["title"] not in already]
    skipped = len(items) - len(to_create)
    print(f"{skipped} already exist in {args.repo} and will be skipped; {len(to_create)} to create.")

    if not args.apply:
        print("\nDRY RUN — no issues created. Re-run with --apply to create them.")
        for it in to_create[:10]:
            print(f"  would create [{it['complexity']}/{it['points']}pt]: {it['title']}")
        if len(to_create) > 10:
            print(f"  ... and {len(to_create) - 10} more")
        return 0

    created = 0
    for it in to_create:
        labels = label_for(it)
        cmd = [
            "gh", "issue", "create", "--repo", args.repo,
            "--title", it["title"], "--body", body_for(it),
            "--label", ",".join(labels),
        ]
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            print(f"\nFAILED on: {it['title']}", file=sys.stderr)
            print(result.stderr, file=sys.stderr)
            print(f"\n{created} issue(s) created before this failure. Re-run the same command to resume "
                  f"— already-created titles will be skipped automatically.", file=sys.stderr)
            return 1
        created += 1
        print(f"[{created}/{len(to_create)}] created: {it['title']}")
        time.sleep(PACE_SECONDS)

    print(f"\nDone. {created} issue(s) created, {skipped} skipped (already existed).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
