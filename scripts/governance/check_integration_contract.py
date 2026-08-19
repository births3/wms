#!/usr/bin/env python3
"""check_integration_contract.py — H-INT 统一外部集成能力契约登记一致性校验

类别：1. 文档治理
Tier：T1（< 10s）
输入：docs/adr/0030-integration-capability.md + docs/architecture-dependencies.md + ROADMAP.md
输出：人类可读 + --json
退出码：
  0  通过（H-INT 在三处真相源登记一致）
  1  发现登记不一致
  2  脚本自身错误

背景：
  ADR-0030 将 H-INT 确立为横向能力（契约先行，引擎延后）。
  该决策登记在三处真相源，必须保持同步：
    1. ADR-0030 本身（状态须为 Accepted）
    2. architecture-dependencies.md §1.1 横向能力表（H-INT 行）
    3. ROADMAP.md（H-INT 波次归属）
  本脚本守护这一同步约束；当连接器代码落地后，可扩展为校验各连接器声明 conform。

校验项（硬，违反 → exit 1）：
  1. ADR-0030 存在
  2. ADR-0030 状态 = Accepted
  3. architecture-dependencies.md 登记 H-INT
  4. ROADMAP.md 提及 H-INT

提示性输出（仅打印，不参与判定、不影响退出码）：
  - 列出已知外部对接模块（H8/H5/M5/M10/M-TC/H4），提醒其实现时应在用户故事声明遵守 ADR-0030。
    （连接器代码尚未落地，暂无法自动校验 conform；待落地后再扩展为真正的校验项。）
"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
ADR = REPO_ROOT / "docs" / "adr" / "0030-integration-capability.md"
ARCH = REPO_ROOT / "docs" / "architecture-dependencies.md"
ROADMAP = REPO_ROOT / "ROADMAP.md"

# 已知外部对接模块（依赖图 + ADR-0030 背景表）；仅用于提示性输出，不参与判定
KNOWN_CONNECTORS = ["H8", "H5", "M5", "M10", "M-TC", "H4"]


@dataclass
class Issue:
    kind: str  # "missing" | "status"
    detail: str


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    issues: list[Issue] = []

    # 1 + 2：ADR 存在且 Accepted
    if not ADR.exists():
        issues.append(Issue("missing", "ADR-0030 文件不存在"))
    else:
        adr_text = ADR.read_text(encoding="utf-8")
        # 状态行形如 "- 状态：Accepted"
        status_ok = any(
            line.strip().startswith("- 状态：") and "Accepted" in line
            for line in adr_text.splitlines()
        )
        if not status_ok:
            issues.append(Issue("status", "ADR-0030 状态非 Accepted（H-INT 尚未生效，不应登记进真相源）"))

    # 3：依赖图登记 H-INT
    if not (ARCH.exists() and "H-INT" in ARCH.read_text(encoding="utf-8")):
        issues.append(Issue("missing", "architecture-dependencies.md 未登记 H-INT"))

    # 4：ROADMAP 提及 H-INT
    if not (ROADMAP.exists() and "H-INT" in ROADMAP.read_text(encoding="utf-8")):
        issues.append(Issue("missing", "ROADMAP.md 未提及 H-INT"))

    if args.json:
        print(json.dumps({
            "check": "check_integration_contract",
            "tier": "T1",
            "category": "文档治理",
            "known_connectors": KNOWN_CONNECTORS,
            "issues": [asdict(i) for i in issues],
            "ok": not issues,
        }, ensure_ascii=False, indent=2))
    else:
        print("check_integration_contract (T1, 文档治理)")
        if not issues:
            print("  ✓ H-INT 在 ADR-0030 / 依赖图 / ROADMAP 三处登记一致")
        else:
            print(f"  ✘ {len(issues)} 处登记不一致:")
            for i in issues:
                print(f"    [{i.kind}] {i.detail}")
        print(f"  · 已知对接模块（实现时应声明 conform ADR-0030）：{', '.join(KNOWN_CONNECTORS)}")

    return 0 if not issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
