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
    match: str  # gitignore-style 模式
    checks: list[str]
    tier: str = "T2"

    def matches(self, path: str) -> bool:
        import pathspec

        spec = pathspec.PathSpec.from_lines("gitwildmatch", [self.match])
        return spec.match_file(path)


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
    """读取 governance/gate-rules.toml。"""
    if toml_path is None:
        toml_path = REPO_ROOT / "governance" / "gate-rules.toml"
    if not toml_path.exists():
        return []

    text = toml_path.read_text(encoding="utf-8")
    rules: list[GateRule] = []

    # Python 3.11+ 有 tomllib；3.10 用 tomli
    try:
        import tomllib
        data = tomllib.loads(text)
    except ModuleNotFoundError:
        import tomli
        data = tomli.loads(text)

    for r in data.get("rules", []):
        rules.append(
            GateRule(
                match=r["match"],
                checks=list(r.get("checks", [])),
                tier=r.get("tier", "T2"),
            )
        )
    return rules
