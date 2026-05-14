"""治理脚本公共库：git diff 解析

详细规则：见 docs/adr/0003-governance-model.md §机制 4

接口：
- get_changed_files(base_ref, include_untracked) → 文件路径相对仓库根
- match_rules(changed, rules)                    → 匹配 gate-rules.toml 的规则
- repo_root()                                    → 仓库根目录绝对路径

约定：
- 默认 base_ref 是 'main'
- 在 worktree 内运行也能正常工作（git common-dir 共享）
"""
from __future__ import annotations

import fnmatch
import subprocess
from dataclasses import dataclass
from pathlib import Path


_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent


def repo_root() -> Path:
    return REPO_ROOT


def _run(cmd: list[str], cwd: Path | None = None) -> tuple[int, str, str]:
    p = subprocess.run(
        cmd,
        cwd=cwd or REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    return p.returncode, p.stdout, p.stderr


def _has_git() -> bool:
    code, _, _ = _run(["git", "rev-parse", "--is-inside-work-tree"])
    return code == 0


def _has_ref(ref: str) -> bool:
    code, _, _ = _run(["git", "rev-parse", "--verify", "--quiet", ref])
    return code == 0


def get_changed_files(
    base_ref: str = "main",
    include_untracked: bool = True,
    only_staged: bool = False,
) -> list[str]:
    """返回相对仓库根的变更文件路径列表。

    优先级：
    - only_staged=True       → 仅 staged 文件（pre-commit 场景）
    - 否则                    → base_ref...HEAD 的差异 + 未跟踪（如开启）

    base_ref 不存在时（如新仓库 / 无 main）→ 返回所有跟踪文件 + 未跟踪
    """
    if not _has_git():
        return []

    if only_staged:
        code, out, _ = _run(["git", "diff", "--name-only", "--cached"])
        return [l.strip() for l in out.splitlines() if l.strip()]

    files: set[str] = set()

    if _has_ref(base_ref):
        code, out, _ = _run(["git", "diff", "--name-only", f"{base_ref}...HEAD"])
        if code == 0:
            files.update(l.strip() for l in out.splitlines() if l.strip())
        # 工作区相对 HEAD 的未提交修改也算
        code2, out2, _ = _run(["git", "diff", "--name-only", "HEAD"])
        if code2 == 0:
            files.update(l.strip() for l in out2.splitlines() if l.strip())
    else:
        # 无 base ref → 视为初次提交场景，所有跟踪文件都是变更
        code, out, _ = _run(["git", "ls-files"])
        if code == 0:
            files.update(l.strip() for l in out.splitlines() if l.strip())

    if include_untracked:
        code, out, _ = _run(
            ["git", "ls-files", "--others", "--exclude-standard"]
        )
        if code == 0:
            files.update(l.strip() for l in out.splitlines() if l.strip())

    return sorted(files)


@dataclass
class GateRule:
    match: str  # glob 模式（fnmatch 语法 + ** 支持）
    checks: list[str]
    tier: str = "T2"

    def matches(self, path: str) -> bool:
        return _glob_match(self.match, path)


def _glob_match(pattern: str, path: str) -> bool:
    """简易 glob 匹配，支持 ** 递归。"""
    # 把 ** 翻成 fnmatch 不直接支持的形式：先用 fnmatch 处理一段
    # 这里实现一个保守版本：把 ** 视为"任意路径段"。
    if "**" in pattern:
        # 拆分 ** 两侧
        # e.g. "backend/crates/**" → 前缀 "backend/crates/"
        # e.g. "**/test_*.py"      → 后缀 "/test_*.py" 或 "test_*.py"
        if pattern.endswith("/**"):
            prefix = pattern[:-3]
            return path == prefix.rstrip("/") or path.startswith(prefix)
        if pattern.startswith("**/"):
            tail = pattern[3:]
            # 要么直接匹配根下的 tail，要么任意目录下的 tail
            return fnmatch.fnmatch(path, tail) or any(
                fnmatch.fnmatch(path[i:], tail)
                for i in range(len(path))
                if i == 0 or path[i - 1] == "/"
            )
        # 中间含 ** ：用 fnmatch 把 ** 转 *
        normalized = pattern.replace("**", "*")
        return fnmatch.fnmatch(path, normalized)
    return fnmatch.fnmatch(path, pattern)


def match_rules(changed: list[str], rules: list[GateRule]) -> dict[str, list[str]]:
    """返回 {check_name: [matched files...]}.

    一个文件可能匹配多条规则，对应的 checks 都会被触发。
    """
    triggered: dict[str, list[str]] = {}
    for f in changed:
        for r in rules:
            if r.matches(f):
                for c in r.checks:
                    triggered.setdefault(c, []).append(f)
    return triggered


def load_gate_rules(toml_path: Path | None = None) -> list[GateRule]:
    """读取 governance/gate-rules.toml。

    Python 3.11+ 自带 tomllib；3.10 需 tomli。本仓库 Wave 0 阶段
    用最简化的 TOML 子集，自带 'tomllib' 不可用时回退到正则解析。
    """
    if toml_path is None:
        toml_path = REPO_ROOT / "governance" / "gate-rules.toml"
    if not toml_path.exists():
        return []

    text = toml_path.read_text(encoding="utf-8")
    rules: list[GateRule] = []

    try:
        import tomllib  # Python 3.11+

        data = tomllib.loads(text)
        for r in data.get("rules", []):
            rules.append(
                GateRule(
                    match=r["match"],
                    checks=list(r.get("checks", [])),
                    tier=r.get("tier", "T2"),
                )
            )
        return rules
    except ModuleNotFoundError:
        pass  # 走简化解析

    # 简化解析（Python 3.10 兜底）：仅支持本仓库的 gate-rules.toml 子集
    import re

    block_re = re.compile(
        r"\[\[rules\]\](.*?)(?=\n\[\[rules\]\]|\Z)", re.DOTALL
    )
    for m in block_re.finditer(text):
        block = m.group(1)
        match = _toml_string(block, "match")
        tier = _toml_string(block, "tier") or "T2"
        checks = _toml_array(block, "checks")
        if match:
            rules.append(GateRule(match=match, checks=checks, tier=tier))
    return rules


def _toml_string(block: str, key: str) -> str | None:
    import re

    m = re.search(rf'{key}\s*=\s*"([^"]*)"', block)
    return m.group(1) if m else None


def _toml_array(block: str, key: str) -> list[str]:
    import re

    m = re.search(rf"{key}\s*=\s*\[(.*?)\]", block, re.DOTALL)
    if not m:
        return []
    inner = m.group(1)
    return [s.strip().strip('"') for s in inner.split(",") if s.strip()]
