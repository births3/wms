#!/usr/bin/env python3
"""check_baseline_health.py — baseline 数量守护 + 过期检测

类别：4. 流程治理
Tier：T1（< 10s）
输入：governance/baselines/*.json + governance/baselines/baseline-health.json（历史快照）
输出：人类可读 + --json
退出码：
  0  通过（baseline 数量未上涨 + 无过期未处理）
  1  发现违规（数量上涨 / 过期未处理 / 文件格式错误）
  2  脚本自身错误

背景：
  ADR-0003 §机制 3 + governance.md §1.3 要求"baseline 单调下降"。
  本脚本守护此约束：不让 baseline 沦为永久豁免的垃圾箱。

检查项：
  1. 每个 baseline 文件 JSON 格式合法
  2. 每个 baseline 文件的 ignored 数量 ≤ 历史最高（即不允许新增）
     - 例外：首次运行（无历史快照）时建立初始快照
     - 例外：脚本自身刚加 baseline（git diff 显示 ignored 数量增加但有 PR 评审通过）
  3. 无 expires_at 早于今天且 id 仍在 baseline 的条目（即"应该过期的还赖着不走"）
  4. snapshot 文件随脚本一起入库，PR 审查可见

行为约定：
  - 默认仅检测，不修改任何文件（pre-commit 安全）
  - --update-snapshot 时显式写入快照文件（PR 评审通过后人工运行）
  - 可收缩项以 ⓘ 报告，提示运行 --update-snapshot 应用

用法：
  python3 scripts/governance/check_baseline_health.py
  python3 scripts/governance/check_baseline_health.py --update-snapshot   更新快照（人工评审通过后）
  python3 scripts/governance/check_baseline_health.py --json
"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
BASELINE_DIR = REPO_ROOT / "governance" / "baselines"
SNAPSHOT_FILE = BASELINE_DIR / "baseline-health.json"


@dataclass
class Issue:
    check: str
    kind: str  # "growth" | "expired" | "malformed"
    message: str


def _today() -> str:
    return datetime.now(timezone.utc).date().isoformat()


def load_snapshot() -> dict[str, int]:
    """读取历史快照（每个 baseline 的最大允许数量）。"""
    if not SNAPSHOT_FILE.exists():
        return {}
    try:
        data = json.loads(SNAPSHOT_FILE.read_text(encoding="utf-8"))
        return data.get("max_counts", {})
    except (json.JSONDecodeError, OSError):
        return {}


def save_snapshot(counts: dict[str, int]) -> None:
    """保存当前 baseline 数量为新基准线（auto-shrink 已发生 → 收紧上限）。"""
    BASELINE_DIR.mkdir(parents=True, exist_ok=True)
    payload = {
        "version": 1,
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "max_counts": counts,
        "note": "由 check_baseline_health.py 维护；每个 check 的 baseline 数量上限。",
    }
    SNAPSHOT_FILE.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )


def scan_baselines() -> tuple[dict[str, int], list[Issue]]:
    """扫描所有 baseline 文件，返回 {check_name: count} 与 issue 列表。"""
    counts: dict[str, int] = {}
    issues: list[Issue] = []

    if not BASELINE_DIR.exists():
        return counts, issues

    today = _today()

    for f in sorted(BASELINE_DIR.glob("*.json")):
        if f.name == "baseline-health.json":
            continue
        check_name = f.stem
        try:
            data = json.loads(f.read_text(encoding="utf-8"))
        except json.JSONDecodeError as e:
            issues.append(Issue(
                check=check_name,
                kind="malformed",
                message=f"JSON parse error: {e}",
            ))
            continue
        ignored = data.get("ignored", [])
        counts[check_name] = len(ignored)

        # 过期检测
        for entry in ignored:
            exp = entry.get("expires_at")
            if exp and exp < today:
                issues.append(Issue(
                    check=check_name,
                    kind="expired",
                    message=f"id={entry.get('id')!r} expired_at={exp} (today={today})",
                ))

    return counts, issues


def check_growth(current: dict[str, int], snapshot: dict[str, int]) -> list[Issue]:
    """检查每个 baseline 数量是否超过历史上限。"""
    issues: list[Issue] = []
    for check, count in current.items():
        max_allowed = snapshot.get(check)
        if max_allowed is None:
            continue  # 首次出现，不算违规
        if count > max_allowed:
            issues.append(Issue(
                check=check,
                kind="growth",
                message=f"baseline 数量 {count} > 历史上限 {max_allowed}（运行 --update-snapshot 提升上限需 PR 评审）",
            ))
    return issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    parser.add_argument(
        "--update-snapshot",
        action="store_true",
        help="把当前 baseline 数量保存为新基准线（PR 评审通过后人工运行）",
    )
    args = parser.parse_args(argv)

    current_counts, scan_issues = scan_baselines()
    snapshot = load_snapshot()
    growth_issues = check_growth(current_counts, snapshot)

    all_issues = scan_issues + growth_issues

    if args.update_snapshot:
        # 即使有 issue 也允许更新（人工已评审）
        save_snapshot(current_counts)
        print(f"✓ snapshot updated: {len(current_counts)} checks tracked")
        return 0

    # 检测可收缩项（仅报告，不自动写文件 — 避免 pre-commit 时改 working tree）
    shrunk: dict[str, int] = {}
    for check, count in current_counts.items():
        max_allowed = snapshot.get(check)
        if max_allowed is not None and count < max_allowed:
            shrunk[check] = count

    if args.json:
        payload = {
            "check": "check_baseline_health",
            "tier": "T1",
            "category": "流程治理",
            "current_counts": current_counts,
            "max_counts_snapshot": snapshot,
            "issues": [asdict(i) for i in all_issues],
            "shrunk": shrunk,
            "ok": not all_issues,
        }
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        total = sum(current_counts.values())
        print(f"check_baseline_health (T1, 流程治理) — {len(current_counts)} baselines, {total} suppressed entries")
        if shrunk:
            print(f"  ⓘ {len(shrunk)} baseline(s) can be shrunk (run --update-snapshot to apply):")
            for check, count in shrunk.items():
                print(f"      {check}: {snapshot[check]} → {count}")
        if not all_issues:
            print("  ✓ all baselines within limits")
        else:
            for i in all_issues:
                print(f"  ✘ [{i.kind}] {i.check}: {i.message}")

    return 0 if not all_issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
