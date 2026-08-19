#!/usr/bin/env python3
"""check_approval_source_chain.py — 库存状态变更的审批源链路检查

类别：1. 文档治理
Tier：T1（< 10s）
输入：docs/domain/user-stories-*.md
输出：人类可读 + --json
退出码：
  0  通过
  1  发现链路缺失
  2  脚本自身错误

背景：
  M3-003 库存状态变更要求带"审批源"字段（5 类枚举）。
  调用方（M-QL/M-SA/M2-验收/M-RC/M3-养护）触发状态变更时必须声明审批源类型，否则审计追踪丢失出处。

检查规则：
  对每个故事正文，若同时满足：
    (a) 含触发关键词（"加锁"/"隔离"/"扣减"/"状态变更"/"状态变为"/"变为不合格"...）
    (b) 不含 "审批源"（approval_source）
  则报告为违规。

例外：M3-003 自身（定义审批源）和 M3-001/M3-002（查询/批次管理）等只读故事不算。
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
DOMAIN_DIR = REPO_ROOT / "docs" / "domain"

# 触发库存状态变更的关键词（强信号：明确表达"调用 M3 改库存"的语境）
TRIGGER_KEYWORDS = [
    "加减锁",  # M-QL/M3 直接的加减锁动作
    "扣减库存", "库存扣减",  # M-SA/M4 扣减
    "状态变更",  # M-QL 联动业务流程
    "执行状态变更",  # M-QL
    "做库存调整",  # M-RC
]

# 必须出现的字段名（任一即可）
REQUIRED_KEYWORDS = ["审批源", "approval_source", "approval source"]

# 例外故事：从 governance/check-data.toml 加载（v0.4 起从硬编码迁出）
sys.path.insert(0, str(_THIS.parent))
from _check_data import load_approval_source_exemptions  # noqa: E402

EXEMPT_STORY_IDS = load_approval_source_exemptions()

STORY_ID_RE = re.compile(r"^##\s+(US-[A-Z0-9]+-\d{3}[a-z]?)")


@dataclass
class Violation:
    file: str
    story_id: str
    matched_triggers: list[str] = field(default_factory=list)


def _split_stories(text: str) -> list[tuple[str, str]]:
    parts: list[tuple[str, str]] = []
    current_id = ""
    current_lines: list[str] = []
    for line in text.splitlines():
        m = STORY_ID_RE.match(line)
        if m:
            if current_id:
                parts.append((current_id, "\n".join(current_lines)))
            current_id = m.group(1)
            current_lines = [line]
        else:
            current_lines.append(line)
    if current_id:
        parts.append((current_id, "\n".join(current_lines)))
    return parts


def _strip_review(text: str) -> str:
    idx = (text.find("\n<details markdown=\"1\">\n<summary>📋 Review 记录") if "📋 Review 记录" in text else text.find("\n## Review 记录"))
    return text[:idx] if idx != -1 else text


def check_file(path: Path) -> list[Violation]:
    rel = path.relative_to(REPO_ROOT).as_posix()
    text = path.read_text(encoding="utf-8")
    text = _strip_review(text)
    violations: list[Violation] = []
    for sid, story_text in _split_stories(text):
        if sid in EXEMPT_STORY_IDS:
            continue
        triggered = [k for k in TRIGGER_KEYWORDS if k in story_text]
        if not triggered:
            continue
        if any(k in story_text for k in REQUIRED_KEYWORDS):
            continue
        violations.append(Violation(file=rel, story_id=sid, matched_triggers=triggered))
    return violations


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    files = sorted(DOMAIN_DIR.glob("user-stories-*.md"))
    all_violations: list[Violation] = []
    for f in files:
        all_violations.extend(check_file(f))

    if args.json:
        payload = {
            "check": "check_approval_source_chain",
            "tier": "T1",
            "category": "文档治理",
            "scanned": len(files),
            "violations": [asdict(v) for v in all_violations],
            "ok": not all_violations,
        }
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print(f"check_approval_source_chain (T1, 文档治理) — scanned {len(files)} files")
        if not all_violations:
            print("  ✓ 所有触发库存状态变更的故事都声明了审批源")
        else:
            print(f"  ✘ {len(all_violations)} 处违规（触发库存状态变更但未声明审批源）:")
            for v in all_violations:
                triggers = "/".join(v.matched_triggers[:3])
                print(f"    {v.file}  [{v.story_id}]  匹配触发词: {triggers}")

    return 0 if not all_violations else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
