#!/usr/bin/env python3
"""check_doc_links.py — 文档内部链接有效性检查 + 附录引用检查

类别：1. 文档治理
Tier：T1（< 10s）
输入：扫描 docs/ + 仓库根 *.md
输出：人类可读 + --json
退出码：
  0  无失效链接 + 附录被预期模块引用
  1  发现失效相对链接 或 附录未被预期模块引用
  2  脚本自身错误

检查项 1：相对链接目标文件与锚点存在
  - 锚点按 Python-Markdown toc slug 规则校验，与 MkDocs 生成结果一致

检查项 2：附录跨模块引用
  - M1 附录 A 储存条件温度区间表必须被 M5 / M-VR / M2 上架等温区相关模块引用
  - 缺一报告
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from urllib.parse import unquote

from markdown.extensions.toc import slugify


_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
DOMAIN_DIR = REPO_ROOT / "docs" / "domain"

LINK_RE = re.compile(r"\[(?P<text>[^\]]*)\]\((?P<url>[^)\s]+)(?:\s+\"[^\"]*\")?\)")
HEADING_RE = re.compile(r"^(#{1,6})\s+(.+?)\s*(?:#+\s*)?$")
EXPLICIT_ID_RE = re.compile(r"\{\s*#([A-Za-z0-9_-]+)\s*\}\s*$")

# 附录引用规则：从 governance/check-data.toml 加载（v0.4 起从硬编码迁出）
sys.path.insert(0, str(_THIS.parent))
from _check_data import load_appendix_references  # noqa: E402


@dataclass
class BrokenLink:
    file: str
    line: int
    target: str
    text: str


@dataclass
class MissingReference:
    appendix: str
    expected_in: str
    defined_in: str


def _find_md_files() -> list[Path]:
    files: list[Path] = []
    for p in REPO_ROOT.rglob("*.md"):
        rel = p.relative_to(REPO_ROOT).as_posix()
        # exclude 任意层级下的依赖、构建产物和本地 Agent worktree
        parts = rel.split("/")
        if any(part in ("node_modules", "target", ".git", ".claude", "dist", "build", "site") for part in parts):
            continue
        files.append(p)
    return sorted(files)


def _is_external(url: str) -> bool:
    return url.startswith(("http://", "https://", "mailto:", "tel:"))


def _strip_anchor(url: str) -> str:
    return url.split("#", 1)[0]


def _anchor(url: str) -> str:
    if "#" not in url:
        return ""
    return unquote(url.split("#", 1)[1]).strip()


def _heading_anchors(text: str) -> set[str]:
    anchors: set[str] = set()
    counts: dict[str, int] = {}

    for line in text.splitlines():
        m = HEADING_RE.match(line)
        if not m:
            continue

        title = m.group(2).strip()
        explicit = EXPLICIT_ID_RE.search(title)
        if explicit:
            anchor = explicit.group(1)
        else:
            title = EXPLICIT_ID_RE.sub("", title).strip()
            base = slugify(title, "-") or "_"
            index = counts.get(base, 0)
            anchor = base if index == 0 else f"{base}_{index}"
            counts[base] = index + 1
        anchors.add(anchor)

    return anchors


def check_file(p: Path) -> list[BrokenLink]:
    broken: list[BrokenLink] = []
    rel = p.relative_to(REPO_ROOT).as_posix()
    try:
        text = p.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return []

    base = p.parent
    for lineno, line in enumerate(text.splitlines(), start=1):
        for m in LINK_RE.finditer(line):
            url = m.group("url").strip()
            text_ = m.group("text")
            if _is_external(url):
                continue
            target = unquote(_strip_anchor(url))
            anchor = _anchor(url)
            if not target:
                resolved = p
            # 解析路径：以 / 开头视为相对仓库根，否则相对当前文件
            elif target.startswith("/"):
                resolved = REPO_ROOT / target.lstrip("/")
            else:
                resolved = (base / target).resolve()
            if not resolved.exists():
                broken.append(
                    BrokenLink(
                        file=rel, line=lineno, target=url, text=text_
                    )
                )
                continue
            if anchor:
                try:
                    target_text = resolved.read_text(encoding="utf-8")
                except UnicodeDecodeError:
                    continue
                if anchor not in _heading_anchors(target_text):
                    broken.append(
                        BrokenLink(
                            file=rel, line=lineno, target=url, text=text_
                        )
                    )
    return broken


def check_appendix_references() -> list[MissingReference]:
    missing: list[MissingReference] = []
    for ref in load_appendix_references():
        for ef in ref.expected_in:
            path = DOMAIN_DIR / ef
            if not path.exists():
                continue
            text = path.read_text(encoding="utf-8")
            if ref.appendix not in text:
                missing.append(
                    MissingReference(
                        appendix=ref.appendix, expected_in=ef, defined_in=ref.defined_in
                    )
                )
    return missing


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    md_files = _find_md_files()
    all_broken: list[BrokenLink] = []
    for p in md_files:
        all_broken.extend(check_file(p))

    missing_refs = check_appendix_references()

    has_issue = bool(all_broken or missing_refs)

    if args.json:
        payload = {
            "check": "check_doc_links",
            "tier": "T1",
            "category": "文档治理",
            "scanned": len(md_files),
            "broken": [asdict(b) for b in all_broken],
            "missing_appendix_refs": [asdict(m) for m in missing_refs],
            "ok": not has_issue,
        }
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print(f"check_doc_links (T1, 文档治理) — scanned {len(md_files)} files")
        if not all_broken:
            print("  ✓ no broken links")
        else:
            print(f"  ✘ {len(all_broken)} broken links:")
            for b in all_broken:
                print(f"    {b.file}:{b.line}  → {b.target}  [{b.text}]")
        if not missing_refs:
            print("  ✓ all appendix cross-references present")
        else:
            print(f"  ✘ {len(missing_refs)} missing appendix references:")
            for m in missing_refs:
                print(
                    f"    {m.appendix} (defined in {m.defined_in}) "
                    f"→ expected reference in {m.expected_in}"
                )

    return 0 if not has_issue else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
