#!/usr/bin/env python3
"""check_doc_links.py — 文档内部链接有效性检查

类别：1. 文档治理
Tier：T1（< 10s）
输入：扫描 docs/ + 仓库根 *.md
输出：人类可读 + --json
退出码：
  0  无失效链接
  1  发现失效相对链接
  2  脚本自身错误

只检查 markdown 中的相对链接（以 / 或非 http(s)/mailto 开头）：
- 链接目标文件必须存在
- 锚点（#section）暂不验证（成本高，价值有限）
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path


_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent

LINK_RE = re.compile(r"\[(?P<text>[^\]]*)\]\((?P<url>[^)\s]+)(?:\s+\"[^\"]*\")?\)")


@dataclass
class BrokenLink:
    file: str
    line: int
    target: str
    text: str


def _find_md_files() -> list[Path]:
    files: list[Path] = []
    for p in REPO_ROOT.rglob("*.md"):
        rel = p.relative_to(REPO_ROOT).as_posix()
        if rel.startswith(("node_modules/", "target/", ".git/")):
            continue
        files.append(p)
    return sorted(files)


def _is_external(url: str) -> bool:
    return url.startswith(("http://", "https://", "mailto:", "tel:"))


def _strip_anchor(url: str) -> str:
    return url.split("#", 1)[0]


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
            target = _strip_anchor(url)
            if not target:
                continue  # pure anchor #foo
            # 解析路径：以 / 开头视为相对仓库根，否则相对当前文件
            if target.startswith("/"):
                resolved = REPO_ROOT / target.lstrip("/")
            else:
                resolved = (base / target).resolve()
            if not resolved.exists():
                broken.append(
                    BrokenLink(
                        file=rel, line=lineno, target=url, text=text_
                    )
                )
    return broken


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    md_files = _find_md_files()
    all_broken: list[BrokenLink] = []
    for p in md_files:
        all_broken.extend(check_file(p))

    if args.json:
        payload = {
            "check": "check_doc_links",
            "tier": "T1",
            "category": "文档治理",
            "scanned": len(md_files),
            "broken": [asdict(b) for b in all_broken],
            "ok": not all_broken,
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

    return 0 if not all_broken else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
