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
from dataclasses import dataclass, field
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
        return [line.strip() for line in out.splitlines() if line.strip()]

    files: set[str] = set()

    if _has_ref(base_ref):
        code, out, _ = _run(["git", "diff", "--name-only", f"{base_ref}...HEAD"])
        if code == 0:
            files.update(line.strip() for line in out.splitlines() if line.strip())
        # 工作区相对 HEAD 的未提交修改也算
        code2, out2, _ = _run(["git", "diff", "--name-only", "HEAD"])
        if code2 == 0:
            files.update(line.strip() for line in out2.splitlines() if line.strip())
    else:
        # 无 base ref → 视为初次提交场景，所有跟踪文件都是变更
        code, out, _ = _run(["git", "ls-files"])
        if code == 0:
            files.update(line.strip() for line in out.splitlines() if line.strip())

    if include_untracked:
        code, out, _ = _run(
            ["git", "ls-files", "--others", "--exclude-standard"]
        )
        if code == 0:
            files.update(line.strip() for line in out.splitlines() if line.strip())

    return sorted(files)


@dataclass
class GateRule:
    match: str  # gitignore-style 模式
    checks: list[str]
    tier: str = "T2"
    rule_ids: list[str] = field(default_factory=list)
    source: str = ""
    contexts: list[str] = field(default_factory=list)

    def matches(self, path: str) -> bool:
        import pathspec

        spec = pathspec.PathSpec.from_lines("gitwildmatch", [self.match])
        return spec.match_file(path)


@dataclass
class GovernanceModel:
    version: int
    layers: list[str]
    decision_precedence: list[str]
    rule_verdicts: list[str]
    execution_statuses: list[str]
    allowed_contexts: list[str]
    default_source: str
    tier_contexts: dict[str, list[str]]


@dataclass
class GateConfig:
    model: GovernanceModel
    rules: list[GateRule]


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


def rules_for_execution(
    rules: list[GateRule],
    *,
    tier: str,
    context: str | None = None,
) -> list[GateRule]:
    """按 Tier 和可选执行场景筛选规则；未传场景时保持旧行为。"""
    return [
        rule
        for rule in rules
        if (rule.tier == tier or rule.tier == "any")
        and (context is None or not rule.contexts or context in rule.contexts)
    ]


def metadata_for_check(
    check_name: str,
    matched_files: list[str],
    rules: list[GateRule],
) -> tuple[list[str], list[str], list[str]]:
    """返回一次检查命中的 rule/source/context，供机器证据输出。"""
    matched = [
        rule
        for rule in rules
        if check_name in rule.checks
        and any(rule.matches(path) for path in matched_files)
    ]
    return (
        sorted({rule_id for rule in matched for rule_id in rule.rule_ids}),
        sorted({rule.source for rule in matched if rule.source}),
        sorted({context for rule in matched for context in rule.contexts}),
    )


def _load_toml(toml_path: Path) -> dict:
    text = toml_path.read_text(encoding="utf-8")
    try:
        import tomllib

        return tomllib.loads(text)
    except ModuleNotFoundError:
        import tomli

        return tomli.loads(text)


def _default_rule_id(check_name: str) -> str:
    return "GOV-" + check_name.upper().replace("_", "-")


def load_gate_config(toml_path: Path | None = None) -> GateConfig:
    """读取轻量治理模型和 diff gate；旧测试配置仍可省略模型段。"""
    if toml_path is None:
        toml_path = REPO_ROOT / "governance" / "gate-rules.toml"
    if not toml_path.exists():
        return GateConfig(
            model=GovernanceModel(0, [], [], [], [], [], "", {}),
            rules=[],
        )

    data = _load_toml(toml_path)
    raw_model = data.get("governance_model", {})
    tier_contexts = {
        tier: list(contexts)
        for tier, contexts in raw_model.get("tier_contexts", {}).items()
    }
    model = GovernanceModel(
        version=int(raw_model.get("version", 0)),
        layers=list(raw_model.get("layers", [])),
        decision_precedence=list(raw_model.get("decision_precedence", [])),
        rule_verdicts=list(raw_model.get("rule_verdicts", [])),
        execution_statuses=list(raw_model.get("execution_statuses", [])),
        allowed_contexts=list(raw_model.get("allowed_contexts", [])),
        default_source=str(raw_model.get("default_source", "")),
        tier_contexts=tier_contexts,
    )

    rules: list[GateRule] = []
    for raw_rule in data.get("rules", []):
        checks = list(raw_rule.get("checks", []))
        tier = raw_rule.get("tier", "T2")
        rules.append(
            GateRule(
                match=raw_rule["match"],
                checks=checks,
                tier=tier,
                rule_ids=list(raw_rule.get("rule_ids", []))
                or [_default_rule_id(check) for check in checks],
                source=raw_rule.get("source", model.default_source),
                contexts=list(
                    raw_rule.get("contexts", model.tier_contexts.get(tier, []))
                ),
            )
        )
    return GateConfig(model=model, rules=rules)


def load_gate_rules(toml_path: Path | None = None) -> list[GateRule]:
    """读取 governance/gate-rules.toml。"""
    return load_gate_config(toml_path).rules
