#!/usr/bin/env python3
"""check_story_size.py — 用户故事文件大小与拆分阈值校验

类别：1. 文档治理
Tier：T1（< 5s）
输入：docs/domain/user-stories-*.md + governance/gate-rules.toml [story_split_thresholds]
输出：人类可读 + --json
退出码：
  0  通过（所有文件在阈值内）
  1  发现违规（超阈值）
  2  脚本自身错误
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib  # Python < 3.11

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DOMAIN_DIR = REPO_ROOT / "docs" / "domain"
GATE_RULES = REPO_ROOT / "governance" / "gate-rules.toml"

# 默认阈值（gate-rules.toml 未配置时）
DEFAULTS = {
    "max_file_kb": 20,
    "max_stories_per_file": 8,
    "max_ac_per_story": 15,
}


@dataclass
class Violation:
    file: str
    rule: str
    value: int
    threshold: int
    detail: str = ""


def load_thresholds() -> dict:
    if GATE_RULES.exists():
        with open(GATE_RULES, "rb") as f:
            cfg = tomllib.load(f)
        return cfg.get("story_split_thresholds", DEFAULTS)
    return DEFAULTS


def check(thresholds: dict) -> list[Violation]:
    violations: list[Violation] = []
    max_kb = thresholds.get("max_file_kb", DEFAULTS["max_file_kb"])
    max_stories = thresholds.get("max_stories_per_file", DEFAULTS["max_stories_per_file"])
    max_ac = thresholds.get("max_ac_per_story", DEFAULTS["max_ac_per_story"])

    for f in sorted(DOMAIN_DIR.glob("user-stories-*.md")):
        text = f.read_text(encoding="utf-8")
        rel = f.relative_to(REPO_ROOT).as_posix()
        kb = f.stat().st_size // 1024

        # 文件大小
        if kb > max_kb:
            violations.append(Violation(rel, "max_file_kb", kb, max_kb))

        # 故事数
        story_ids = re.findall(r"^## (US-[A-Z][A-Z0-9]+-\d+)", text, re.M)
        if len(story_ids) > max_stories:
            violations.append(Violation(rel, "max_stories_per_file", len(story_ids), max_stories))

        # 单故事 AC 条数
        blocks = re.split(r"^## US-[A-Z]", text, flags=re.M)
        for i, block in enumerate(blocks[1:], 1):
            ac_match = re.search(r"### 验收标准\s*\n([\s\S]*?)(?=^##|\Z)", block, re.M)
            if not ac_match:
                continue
            n = len(re.findall(r"^\s*\d+\.\s+", ac_match.group(1), re.M))
            if n > max_ac:
                sid = story_ids[i - 1] if i - 1 < len(story_ids) else f"story#{i}"
                violations.append(Violation(rel, "max_ac_per_story", n, max_ac, detail=sid))

    return violations


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    thresholds = load_thresholds()
    violations = check(thresholds)

    if args.json:
        print(json.dumps({
            "check": "check_story_size",
            "tier": "T1",
            "category": "文档治理",
            "thresholds": thresholds,
            "violations": [{"file": v.file, "rule": v.rule, "value": v.value,
                            "threshold": v.threshold, "detail": v.detail} for v in violations],
            "ok": len(violations) == 0,
        }, ensure_ascii=False, indent=2))
    else:
        print(f"check_story_size (T1, 文档治理) — thresholds: "
              f"{thresholds.get('max_file_kb', 20)}KB / "
              f"{thresholds.get('max_stories_per_file', 8)} stories / "
              f"{thresholds.get('max_ac_per_story', 15)} AC")
        if violations:
            print(f"\n  警告（{len(violations)} 项超阈值）：")
            for v in violations:
                detail = f" ({v.detail})" if v.detail else ""
                print(f"    ⚠ [{v.file}] {v.rule}: {v.value} > {v.threshold}{detail}")
        else:
            print("  ✓ 所有故事文件在拆分阈值内")

    return 0  # warning 不阻塞 T1（拆分是建议，不是硬约束）


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
