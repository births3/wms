#!/usr/bin/env python3
"""validate_adr_index.py — ADR 编号、索引、状态合法性

类别：1. 文档治理
Tier：T1（< 10s）
输入：docs/adr/*.md（不含 README.md）
输出：人类可读 + --json
退出码：
  0  通过
  1  发现编号冲突 / 状态非法 / 索引未登记 / 缺必填段
  2  脚本自身错误

校验项：
- 文件名格式：NNNN-<slug>.md（4 位顺序号 + 短横线 + slug）
- 编号唯一、不复用
- 状态字段必须 ∈ {Proposed, Accepted, Deprecated, Superseded by ADR-XXXX}
- 必填段：背景 / 决策 / 后果（或英文等价）
- docs/adr/README.md 索引（如存在）必须列出所有 ADR

注：第 0 周 README.md 索引可能尚未建立 → 仅警告不失败
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

FILENAME_RE = re.compile(r"^(\d{4})-([a-z0-9][a-z0-9-]*)\.md$")
STATUS_RE = re.compile(r"^- 状态[:：]\s*(.+?)\s*$", re.MULTILINE)
STATUS_OK = {"Proposed", "Accepted", "Deprecated"}
STATUS_SUPERSEDED_RE = re.compile(r"^Superseded by ADR-\d{4}$")

REQUIRED_SECTIONS = ["背景", "决策", "后果"]


@dataclass
class AdrInfo:
    file: str
    number: str
    slug: str
    status: str = ""
    issues: list[str] = field(default_factory=list)


def parse_adr(p: Path) -> AdrInfo:
    name = p.name
    m = FILENAME_RE.match(name)
    if not m:
        return AdrInfo(
            file=name,
            number="",
            slug="",
            issues=[f"filename does not match NNNN-<slug>.md"],
        )
    number, slug = m.group(1), m.group(2)
    text = p.read_text(encoding="utf-8")

    info = AdrInfo(file=name, number=number, slug=slug)

    sm = STATUS_RE.search(text)
    if not sm:
        info.issues.append("missing '状态' field in header")
    else:
        info.status = sm.group(1)
        if (
            info.status not in STATUS_OK
            and not STATUS_SUPERSEDED_RE.match(info.status)
            # 兼容版本号修饰：Accepted（v0.2，取代 v0.1 三阶段版）
            and not any(info.status.startswith(s) for s in STATUS_OK)
        ):
            info.issues.append(f"invalid status: {info.status!r}")

    for section in REQUIRED_SECTIONS:
        if not re.search(rf"^##\s+{re.escape(section)}", text, re.MULTILINE) and not re.search(
            rf"^##\s+\d+\.\s*{re.escape(section)}", text, re.MULTILINE
        ):
            info.issues.append(f"missing required section: {section}")

    return info


def check_index(adr_files: list[AdrInfo]) -> list[str]:
    """检查 docs/adr/README.md 索引是否完整。"""
    issues: list[str] = []
    readme = ADR_DIR / "README.md"
    if not readme.exists():
        return ["[warn] docs/adr/README.md not found (will be created in Wave 0 task 9)"]
    text = readme.read_text(encoding="utf-8")
    for a in adr_files:
        if not a.number:
            continue
        if a.file not in text and f"ADR-{a.number}" not in text:
            issues.append(f"ADR-{a.number} ({a.file}) not listed in README.md")
    return issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    if not ADR_DIR.exists():
        print("docs/adr/ not found", file=sys.stderr)
        return 2

    adr_files = sorted(
        [p for p in ADR_DIR.glob("*.md") if p.name != "README.md"]
    )
    infos = [parse_adr(p) for p in adr_files]

    # 编号唯一
    nums: dict[str, list[str]] = {}
    for i in infos:
        if i.number:
            nums.setdefault(i.number, []).append(i.file)
    for n, files in nums.items():
        if len(files) > 1:
            for f in files:
                next(x for x in infos if x.file == f).issues.append(
                    f"duplicate number {n} also used by: {[x for x in files if x != f]}"
                )

    index_issues = check_index(infos)

    has_failure = any(i.issues for i in infos) or any(
        not s.startswith("[warn]") for s in index_issues
    )

    if args.json:
        payload = {
            "check": "validate_adr_index",
            "tier": "T1",
            "category": "文档治理",
            "adrs": [asdict(i) for i in infos],
            "index_issues": index_issues,
            "ok": not has_failure,
        }
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print(f"validate_adr_index (T1, 文档治理) — found {len(infos)} ADRs")
        for i in infos:
            mark = "✓" if not i.issues else "✘"
            print(f"  {mark} {i.file}  status={i.status or '?'}")
            for issue in i.issues:
                print(f"      - {issue}")
        if index_issues:
            print("\nindex issues:")
            for s in index_issues:
                print(f"  {s}")

    return 0 if not has_failure else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
