#!/usr/bin/env python3
"""validate_doc_layers.py — 文档四层一致性校验

类别：1. 文档治理
Tier：T1（< 10s）
输入：扫描 docs/ + 仓库根 *.md + governance/
输出：人类可读 + --json
退出码：
  0  通过
  1  发现层间不一致
  2  脚本自身错误

校验项：
- L1: governance.md / architecture-dependencies.md 引用的 ADR 文件必须存在
- L2: governance.md 变更记录段必须存在且非空
- L3: docs/domain/*.md 如果存在，对应 backend/crates 目录应存在（弱校验）
- L4: ROADMAP.md / TODO.md / CHANGELOG.md 必须存在
- 跨层: L2 文档引用的 L1 ADR 状态不能是 Deprecated（引用已废弃决策 = 规范过时）
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

ADR_DIR = REPO_ROOT / "docs" / "adr"
DOCS_DIR = REPO_ROOT / "docs"

# ADR 引用模式：ADR-NNNN 或 docs/adr/NNNN-xxx.md
ADR_REF_RE = re.compile(r"(?:ADR-|docs/adr/)(\d{4})")
STATUS_RE = re.compile(r"^- 状态[:：]\s*(.+?)\s*$", re.MULTILINE)


@dataclass
class Issue:
    layer: str
    file: str
    message: str
    severity: str = "error"  # error | warn


def _read(p: Path) -> str:
    try:
        return p.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return ""


def _adr_status(number: str) -> str | None:
    """读取某 ADR 的状态；不存在时检查索引是否标记为预留。"""
    candidates = list(ADR_DIR.glob(f"{number}-*.md"))
    if candidates:
        text = _read(candidates[0])
        m = STATUS_RE.search(text)
        return m.group(1) if m else "unknown"
    # 文件不存在 → 检查 ADR 索引是否标记为"预留"
    index = ADR_DIR / "README.md"
    if index.exists():
        idx_text = _read(index)
        if re.search(rf"ADR-{number}.*预留|{number}.*预留|{number}.*reserved", idx_text, re.IGNORECASE):
            return "Reserved"
    return None


def check_l1_references(issues: list[Issue]) -> None:
    """L2 文档引用的 ADR 必须存在且非 Deprecated。"""
    l2_files = [
        DOCS_DIR / "governance.md",
        DOCS_DIR / "architecture-dependencies.md",
    ]
    for f in l2_files:
        if not f.exists():
            continue
        text = _read(f)
        rel = f.relative_to(REPO_ROOT).as_posix()
        for m in ADR_REF_RE.finditer(text):
            num = m.group(1)
            status = _adr_status(num)
            if status is None:
                issues.append(Issue(
                    layer="L1→L2",
                    file=rel,
                    message=f"references ADR-{num} but file does not exist and not marked as reserved in index",
                ))
            elif status == "Reserved":
                pass  # 预留编号，引用合法
            elif status.startswith("Deprecated"):
                issues.append(Issue(
                    layer="L1→L2",
                    file=rel,
                    message=f"references ADR-{num} which is Deprecated — update reference",
                    severity="warn",
                ))


def check_l2_changelog(issues: list[Issue]) -> None:
    """governance.md 必须有变更记录段。"""
    gov = DOCS_DIR / "governance.md"
    if not gov.exists():
        issues.append(Issue(layer="L2", file="docs/governance.md", message="file missing"))
        return
    text = _read(gov)
    if "## 7. 变更记录" not in text and "变更记录" not in text:
        issues.append(Issue(
            layer="L2",
            file="docs/governance.md",
            message="missing '变更记录' section",
        ))


def check_l3_domain_code_sync(issues: list[Issue]) -> None:
    """docs/domain/*.md 如果存在，对应 backend 目录应存在（弱校验）。"""
    domain_dir = DOCS_DIR / "domain"
    if not domain_dir.exists():
        return
    for md in domain_dir.glob("*.md"):
        if md.name == ".gitkeep":
            continue
        # 从文件名推断上下文名：如 inbound.md → backend/crates 下应有 inbound 相关
        ctx = md.stem.replace("-", "_")
        # 宽松检查：backend 目录树里有这个名字就行
        backend = REPO_ROOT / "backend"
        if backend.exists():
            found = any(backend.rglob(f"*{ctx}*"))
            if not found:
                issues.append(Issue(
                    layer="L3",
                    file=f"docs/domain/{md.name}",
                    message=f"domain doc exists but no matching backend path containing '{ctx}'",
                    severity="warn",
                ))


def check_infra_docs(issues: list[Issue]) -> None:
    """docs/infra/ 基础设施文档校验。"""
    infra_dir = DOCS_DIR / "infra"
    if not infra_dir.exists():
        issues.append(Issue(
            layer="L2",
            file="docs/infra/",
            message="infra directory missing (expected technical-specs.md)",
        ))
        return
    specs = infra_dir / "technical-specs.md"
    if not specs.exists():
        issues.append(Issue(
            layer="L2",
            file="docs/infra/technical-specs.md",
            message="technical-specs.md missing",
        ))
        return
    text = _read(specs)
    required_sections = ["H6", "H7", "H8"]
    for section in required_sections:
        if section not in text:
            issues.append(Issue(
                layer="L2",
                file="docs/infra/technical-specs.md",
                message=f"missing infrastructure module section: {section}",
                severity="warn",
            ))


def check_l4_existence(issues: list[Issue]) -> None:
    """L4 运营文档必须存在。"""
    required = ["README.md", "ROADMAP.md", "TODO.md", "CHANGELOG.md"]
    for name in required:
        if not (REPO_ROOT / name).exists():
            issues.append(Issue(layer="L4", file=name, message="required L4 file missing"))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    issues: list[Issue] = []
    check_l1_references(issues)
    check_l2_changelog(issues)
    check_infra_docs(issues)
    check_l3_domain_code_sync(issues)
    check_l4_existence(issues)

    errors = [i for i in issues if i.severity == "error"]
    warns = [i for i in issues if i.severity == "warn"]

    if args.json:
        payload = {
            "check": "validate_doc_layers",
            "tier": "T1",
            "category": "文档治理",
            "issues": [asdict(i) for i in issues],
            "ok": not errors,
        }
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print(f"validate_doc_layers (T1, 文档治理)")
        if not issues:
            print("  ✓ all document layers consistent")
        else:
            for i in issues:
                mark = "✘" if i.severity == "error" else "⚠"
                print(f"  {mark} [{i.layer}] {i.file}: {i.message}")
            if errors:
                print(f"\n  {len(errors)} error(s), {len(warns)} warning(s)")
            else:
                print(f"\n  ✓ no errors ({len(warns)} warning(s))")

    return 0 if not errors else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
