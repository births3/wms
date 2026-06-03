#!/usr/bin/env python3
"""check_component_props_classname.py — Props 接口必须支持 className

类别：6. 原型治理
Tier：T1（< 10s）
输入：packages/ui/src/business/**/*.tsx
输出：人类可读 + --json
退出码：0 通过 / 1 违规 / 2 脚本错误

校验项（对照 docs/frontend-coding-standards.md §3）：
- 业务复合组件 Props 接口（<Name>Props）必须含 className 支持：
  - 直接声明 className?: string
  - 或继承 HTMLAttributes / SVGAttributes / ButtonHTMLAttributes 等（自动获得）
- forwardRef 必须存在
- displayName 必须设置

不覆盖：
- className 是否真的转发到根元素（人工 review）
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
BUSINESS_DIR = REPO_ROOT / "packages" / "ui" / "src" / "business"

PROPS_INTERFACE_RE = re.compile(r"export\s+interface\s+(\w+Props)\b([^{]*)\{([^}]*)\}", re.DOTALL)
HTML_ATTRS_RE = re.compile(r"(HTMLAttributes|SVGAttributes|ButtonHTMLAttributes|InputHTMLAttributes|SelectHTMLAttributes|TextareaHTMLAttributes|AnchorHTMLAttributes|HTMLProps)")
CLASSNAME_FIELD_RE = re.compile(r"\bclassName\s*\??\s*:")
FORWARD_REF_RE = re.compile(r"React\.forwardRef|forwardRef<")
DISPLAY_NAME_RE = re.compile(r"\.displayName\s*=")
# 泛型函数组件（React forwardRef 不支持泛型，社区惯用 export function Name<T>）
GENERIC_FUNC_RE = re.compile(r"export\s+function\s+\w+\s*<\s*\w")
SKIP_TAG = "@governance: skip-classname"


def _check_file(path: Path) -> list[str]:
    issues: list[str] = []
    text = path.read_text(encoding="utf-8")
    if SKIP_TAG in text[:500]:
        return []

    matches = list(PROPS_INTERFACE_RE.finditer(text))
    if not matches:
        return []

    for m in matches:
        name = m.group(1)
        extends_part = m.group(2)
        body = m.group(3)
        has_html_attrs = bool(HTML_ATTRS_RE.search(extends_part))
        has_classname = bool(CLASSNAME_FIELD_RE.search(body))
        if not (has_html_attrs or has_classname):
            issues.append(f"接口 {name} 既未继承 HTMLAttributes 也未声明 className")

    # 泛型函数组件：React forwardRef 不支持泛型 → 豁免 forwardRef + displayName
    is_generic_fn = bool(GENERIC_FUNC_RE.search(text))
    if not is_generic_fn:
        if not FORWARD_REF_RE.search(text):
            issues.append("未使用 React.forwardRef")
        if not DISPLAY_NAME_RE.search(text):
            issues.append("未设置 displayName")

    return issues


def run() -> list[str]:
    errors: list[str] = []
    if not BUSINESS_DIR.exists():
        return errors
    for f in sorted(BUSINESS_DIR.rglob("*.tsx")):
        if ".stories." in f.name or ".spec." in f.name or ".test." in f.name:
            continue
        for issue in _check_file(f):
            rel = f.relative_to(REPO_ROOT).as_posix()
            errors.append(f"{rel}: {issue}")
    return errors


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    try:
        errors = run()
    except Exception as e:
        if args.json:
            print(json.dumps({"status": "error", "message": str(e)}))
        else:
            print(f"[ERROR] {e}", file=sys.stderr)
        sys.exit(2)

    if args.json:
        print(json.dumps({"status": "fail" if errors else "pass", "errors": errors, "ok": not errors}))
    else:
        if errors:
            print(f"✗ check_component_props_classname: {len(errors)} 项违规")
            for e in errors:
                print(f"  - {e}")
        else:
            print("✓ check_component_props_classname: 通过")

    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
