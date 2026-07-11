#!/usr/bin/env python3
"""validate_environment.py — 开发环境就绪检查

类别：5. 运行治理
Tier：T1（< 10s）
输入：无
输出：人类可读 + --json 机器可读
退出码：
  0  全部就绪
  1  有必需工具缺失或版本过低
  2  脚本自身错误

检查项：
- python3 ≥ 3.10（治理脚本依赖）
- git ≥ 2.30（worktree、partial clone）
- rustc / cargo（可选；未到 Wave 1 时仅警告）
- pnpm（可选；未到 Wave 2 时仅警告）
- node（可选）
- just（可选；未安装时主入口不可用）
- lefthook（可选；未安装时 git hooks 不生效）

约束：本脚本不可有外部依赖（仅 Python 标准库）。
"""
from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
from dataclasses import asdict, dataclass


@dataclass
class Tool:
    name: str
    required: bool
    min_version: tuple[int, ...] | None = None
    version_cmd: tuple[str, ...] = ("--version",)
    version_regex: str = r"(\d+)\.(\d+)(?:\.(\d+))?"


TOOLS: list[Tool] = [
    Tool("python3", required=True, min_version=(3, 10)),
    Tool("git", required=True, min_version=(2, 30)),
    Tool("rustc", required=False, min_version=(1, 70)),
    Tool("cargo", required=False, min_version=(1, 70)),
    Tool("node", required=False, min_version=(20, 0)),
    Tool("pnpm", required=False, min_version=(8, 0)),
    Tool("just", required=False),
    Tool("lefthook", required=False),
    Tool("gitleaks", required=True),
    Tool("ruff", required=True),
]


# Python 包依赖（治理脚本运行时必须可 import）
# 名称为 import 名（不是 pip 包名，注意区别如 PyYAML / yaml）
PYTHON_PACKAGES_REQUIRED: list[str] = [
    "pathspec",  # _diff.py 需要（gate-rules 的 glob 匹配）
    "markdown",  # check_doc_links.py 需要（slugify 锚点）
]

# Python < 3.11 才需要的兼容包
PYTHON_PACKAGES_REQUIRED_PY310: list[str] = [
    "tomli",  # _diff.py / _check_data.py 在 PY<3.11 时需要
]


@dataclass
class PyPackageCheck:
    name: str
    found: bool
    note: str = ""

    @property
    def ok(self) -> bool:
        return self.found


@dataclass
class CheckResult:
    name: str
    found: bool
    version: str | None
    ok: bool
    note: str = ""


def _run_version(name: str, args: tuple[str, ...]) -> str | None:
    path = shutil.which(name)
    if not path:
        return None
    try:
        out = subprocess.run(
            [path, *args],
            capture_output=True,
            text=True,
            timeout=2,  # T1 预算紧，单工具版本探测最多 2 秒
            check=False,
        )
        return (out.stdout or out.stderr).strip().splitlines()[0] if (out.stdout or out.stderr) else ""
    except subprocess.TimeoutExpired:
        return "<timeout>"
    except OSError:
        return ""


def _parse_version(text: str, regex: str) -> tuple[int, ...] | None:
    if not text:
        return None
    m = re.search(regex, text)
    if not m:
        return None
    parts = [int(p) for p in m.groups() if p is not None]
    return tuple(parts)


def check_tool(t: Tool) -> CheckResult:
    raw = _run_version(t.name, t.version_cmd)
    if raw is None:
        return CheckResult(
            name=t.name,
            found=False,
            version=None,
            ok=not t.required,
            note="not found" + (" (REQUIRED)" if t.required else " (optional)"),
        )
    if raw == "<timeout>":
        return CheckResult(
            name=t.name,
            found=True,
            version=None,
            ok=not t.required,
            note="version probe timed out (>2s); skipped",
        )
    parsed = _parse_version(raw, t.version_regex) if t.min_version else None
    if t.min_version and parsed:
        ok = parsed >= t.min_version
        note = "" if ok else f"version {'.'.join(map(str, parsed))} < required {'.'.join(map(str, t.min_version))}"
    else:
        ok = True
        note = ""
    return CheckResult(name=t.name, found=True, version=raw, ok=ok, note=note)


def check_python_packages() -> list[PyPackageCheck]:
    """检查治理脚本运行所需的 Python 包是否可 import。"""
    import importlib

    pkgs = list(PYTHON_PACKAGES_REQUIRED)
    if sys.version_info < (3, 11):
        pkgs.extend(PYTHON_PACKAGES_REQUIRED_PY310)

    results: list[PyPackageCheck] = []
    for pkg in pkgs:
        try:
            importlib.import_module(pkg)
            results.append(PyPackageCheck(name=pkg, found=True))
        except ImportError:
            results.append(PyPackageCheck(
                name=pkg,
                found=False,
                note=f"missing — install: pip install {pkg}",
            ))
    return results


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true", help="JSON 输出")
    args = parser.parse_args(argv)

    # 并行检查（每个 tool 独立 subprocess，避免串行 9 次 = 5+ 秒）
    from concurrent.futures import ThreadPoolExecutor

    with ThreadPoolExecutor(max_workers=min(9, len(TOOLS))) as ex:
        results = list(ex.map(check_tool, TOOLS))

    pkg_results = check_python_packages()

    if args.json:
        payload = {
            "check": "validate_environment",
            "tier": "T1",
            "category": "运行治理",
            "results": [asdict(r) for r in results],
            "python_packages": [asdict(p) for p in pkg_results],
            "ok": all(r.ok for r in results) and all(p.ok for p in pkg_results),
        }
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print("validate_environment (T1, 运行治理)")
        for r in results:
            mark = "✓" if r.ok else "✘"
            ver = r.version or "—"
            note = f"  ({r.note})" if r.note else ""
            print(f"  {mark} {r.name:<10} {ver}{note}")

        print("  ── python packages ──")
        for p in pkg_results:
            mark = "✓" if p.ok else "✘"
            note = f"  ({p.note})" if p.note else ""
            print(f"  {mark} {p.name:<10} import{note}")

        failed_tools = [r for r in results if not r.ok]
        failed_pkgs = [p for p in pkg_results if not p.ok]
        total_failed = len(failed_tools) + len(failed_pkgs)
        if total_failed:
            print(f"\n{total_failed} required check(s) failed")
        else:
            print("\n✓ environment OK")

    return 0 if all(r.ok for r in results) and all(p.ok for p in pkg_results) else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
