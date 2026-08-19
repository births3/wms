#!/usr/bin/env python3
"""check_feature_flags.py — Feature Flag 文件版治理校验

类别：4. 流程治理
Tier：T1（< 10s）
输入：deploy/feature_flags.toml
输出：人类可读 + --json
退出码：
  0  通过
  1  Feature Flag 元数据缺失、日期非法或清理期过期
  2  脚本自身错误

Wave 1 文件版 Feature Flag 仅做最小存储；本脚本守护 ADR-0016 §Feature Flag 治理：
- 每个 flag 必须有 owner
- 每个 flag 必须有 created_at
- 每个 flag 必须有 cleanup_by
- cleanup_by 不得晚于 created_at + 90 天
- cleanup_by 早于当前日期视为过期，必须清理
"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass
from datetime import date, datetime, timedelta
from pathlib import Path
from typing import Any

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
FEATURE_FLAGS = REPO_ROOT / "deploy" / "feature_flags.toml"
MAX_LIFETIME_DAYS = 90


@dataclass
class Issue:
    kind: str
    target: str
    detail: str


def _load_toml(path: Path) -> dict[str, Any]:
    text = path.read_text(encoding="utf-8")
    try:
        import tomllib
        return tomllib.loads(text)
    except ModuleNotFoundError:
        import tomli
        return tomli.loads(text)


def _parse_date(value: Any, field: str, key: str, issues: list[Issue]) -> date | None:
    if isinstance(value, datetime):
        return value.date()
    if isinstance(value, date):
        return value
    if isinstance(value, str):
        try:
            return date.fromisoformat(value)
        except ValueError:
            pass
    issues.append(Issue("invalid_date", key, f"{field} 必须是 YYYY-MM-DD 日期"))
    return None


def _flag_key(flag: dict[str, Any], index: int) -> str:
    raw = flag.get("key")
    return str(raw).strip() if raw else f"flags[{index}]"


def check_flags(data: dict[str, Any], *, today: date | None = None) -> list[Issue]:
    today = today or date.today()
    issues: list[Issue] = []
    flags = data.get("flags", [])
    if flags is None:
        flags = []
    if not isinstance(flags, list):
        return [Issue("invalid_schema", "flags", "flags 必须是数组")]

    for index, flag in enumerate(flags):
        if not isinstance(flag, dict):
            issues.append(Issue("invalid_schema", f"flags[{index}]", "flag 条目必须是 table/object"))
            continue

        key = _flag_key(flag, index)
        if not str(flag.get("key", "")).strip():
            issues.append(Issue("missing_field", key, "缺少 key"))
        if not str(flag.get("owner", "")).strip():
            issues.append(Issue("missing_field", key, "缺少 owner"))
        if not isinstance(flag.get("enabled"), bool):
            issues.append(Issue("missing_field", key, "enabled 必须是 boolean"))

        created_at = _parse_date(flag.get("created_at"), "created_at", key, issues)
        cleanup_by = _parse_date(flag.get("cleanup_by"), "cleanup_by", key, issues)
        if created_at is None or cleanup_by is None:
            continue

        if created_at > today:
            issues.append(Issue("invalid_date", key, f"created_at {created_at.isoformat()} 晚于当前日期"))
        max_cleanup = created_at + timedelta(days=MAX_LIFETIME_DAYS)
        if cleanup_by > max_cleanup:
            issues.append(Issue(
                "lifetime_too_long",
                key,
                f"cleanup_by {cleanup_by.isoformat()} 超过 created_at + {MAX_LIFETIME_DAYS} 天（最晚 {max_cleanup.isoformat()}）",
            ))
        if cleanup_by < today:
            issues.append(Issue("expired", key, f"cleanup_by {cleanup_by.isoformat()} 已过期，必须清理或续期审批"))

    return issues


def run(path: Path = FEATURE_FLAGS) -> tuple[list[Issue], int]:
    if not path.exists():
        return [Issue("missing_file", path.relative_to(REPO_ROOT).as_posix(), "Feature Flag 文件不存在")], 0
    data = _load_toml(path)
    flags = data.get("flags", [])
    count = len(flags) if isinstance(flags, list) else 0
    return check_flags(data), count


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    issues, count = run()

    if args.json:
        print(json.dumps({
            "check": "check_feature_flags",
            "tier": "T1",
            "category": "流程治理",
            "file": FEATURE_FLAGS.relative_to(REPO_ROOT).as_posix(),
            "flag_count": count,
            "max_lifetime_days": MAX_LIFETIME_DAYS,
            "issues": [asdict(i) for i in issues],
            "ok": not issues,
        }, ensure_ascii=False, indent=2))
    else:
        print("check_feature_flags (T1, 流程治理)")
        print(f"  · file: {FEATURE_FLAGS.relative_to(REPO_ROOT).as_posix()}")
        print(f"  · flags: {count}")
        if issues:
            print(f"  ✘ {len(issues)} 项 Feature Flag 治理违规:")
            for issue in issues:
                print(f"    [{issue.kind}] {issue.target}: {issue.detail}")
        else:
            print("  ✓ Feature Flag 元数据与 90 天清理期通过")

    return 0 if not issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
