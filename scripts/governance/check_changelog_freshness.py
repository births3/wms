#!/usr/bin/env python3
"""check_changelog_freshness.py — CHANGELOG 与最新 git tag 同步检查。

类别：4. 流程治理
Tier：T1（< 10s）
输入：CHANGELOG.md + git tag
输出：人类可读 + --json
退出码：
  0  CHANGELOG 存在；若仓库已有 tag，最新 tag 已记录
  1  CHANGELOG 缺失，或最新 tag 未记录
  2  脚本自身错误
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
CHANGELOG = REPO_ROOT / "CHANGELOG.md"


@dataclass
class Issue:
    kind: str
    target: str
    detail: str


def latest_git_tag() -> str | None:
    result = subprocess.run(
        ["git", "tag", "--list", "--sort=-creatordate"],
        cwd=REPO_ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return None
    for line in result.stdout.splitlines():
        tag = line.strip()
        if tag:
            return tag
    return None


def _tag_candidates(tag: str) -> set[str]:
    candidates = {tag}
    if tag.startswith("v") and len(tag) > 1:
        candidates.add(tag[1:])
    else:
        candidates.add(f"v{tag}")
    return candidates


def check_changelog_text(text: str, *, latest_tag: str | None) -> list[Issue]:
    if not latest_tag:
        return []

    if any(candidate in text for candidate in _tag_candidates(latest_tag)):
        return []

    return [
        Issue(
            "missing_latest_tag",
            "CHANGELOG.md",
            f"CHANGELOG 未记录最新 git tag: {latest_tag}",
        )
    ]


def run(path: Path = CHANGELOG) -> tuple[list[Issue], str | None]:
    latest_tag = latest_git_tag()
    if not path.exists():
        return [Issue("missing_file", path.relative_to(REPO_ROOT).as_posix(), "CHANGELOG.md 不存在")], latest_tag
    return check_changelog_text(path.read_text(encoding="utf-8"), latest_tag=latest_tag), latest_tag


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    issues, latest_tag = run()

    if args.json:
        print(json.dumps({
            "check": "check_changelog_freshness",
            "tier": "T1",
            "category": "流程治理",
            "file": CHANGELOG.relative_to(REPO_ROOT).as_posix(),
            "latest_tag": latest_tag,
            "issues": [asdict(issue) for issue in issues],
            "ok": not issues,
        }, ensure_ascii=False, indent=2))
    else:
        print("check_changelog_freshness (T1, 流程治理)")
        print(f"  · file: {CHANGELOG.relative_to(REPO_ROOT).as_posix()}")
        print(f"  · latest tag: {latest_tag or '(none)'}")
        if issues:
            print(f"  ✘ {len(issues)} 项 CHANGELOG 同步问题:")
            for issue in issues:
                print(f"    [{issue.kind}] {issue.target}: {issue.detail}")
        else:
            print("  ✓ CHANGELOG 与最新 tag 同步")

    return 0 if not issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as error:  # noqa: BLE001
        print(f"script error: {error}", file=sys.stderr)
        sys.exit(2)
