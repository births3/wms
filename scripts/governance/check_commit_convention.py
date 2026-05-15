#!/usr/bin/env python3
"""check_commit_convention.py — Conventional Commits 提交信息校验

类别：4. 流程治理
Tier：T1（< 10s）
模式：
  --staged       校验当前暂存区将要 commit 的信息（pre-commit 钩子用）
  --file <PATH>  校验指定文件中的 commit message（commit-msg 钩子用，由 lefthook 传入）
  --last         校验 HEAD 上次 commit
  --range REF    校验某个 ref...HEAD 范围内的所有 commit

格式：<type>(<scope>): <subject>

允许的 type：feat fix docs style refactor perf test build ci chore revert
允许的 scope（详见 docs/governance.md §3.2）：
  master-data inbound inventory outbound quality cold-chain
  billing compliance audit pda web-admin api infra governance docs

退出码：
  0 通过
  1 格式不合规
  2 脚本自身错误

注：在 pre-commit 钩子上下文中"暂存区将要 commit 的信息"还不存在；
   --staged 模式下若没有 commit message（如刚 git add），脚本会跳过返回 0。
   commit-msg 钩子由 lefthook 传 --file 参数，会校验当次 message。
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from dataclasses import asdict, dataclass
from pathlib import Path


_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent

TYPES = {
    "功能", "修复", "文档", "格式", "重构",
    "性能", "测试", "构建", "集成", "杂项", "回滚",
}
SCOPES = {
    "基础档案", "入库", "库存", "出库", "质量",
    "冷链", "计费", "合规", "审计",
    "pda", "管理端", "接口", "基础设施", "治理", "文档",
    "追溯码", "对账", "药检", "校验", "质量联系单", "企微", "快递",
}

# 允许 BREAKING CHANGE 在 footer
HEADER_RE = re.compile(
    r"^(?P<type>[a-z\u4e00-\u9fff]+)(?:\((?P<scope>[a-z0-9\u4e00-\u9fff\-,/ ]+)\))?(?P<bang>!)?[:：]\s*(?P<subject>.+)$"
)


@dataclass
class CommitIssue:
    sha: str
    header: str
    issues: list[str]


def _read_msg_from_file(path: Path) -> str:
    text = path.read_text(encoding="utf-8")
    # 去掉行首注释（git commit message 的 #）
    lines = [l for l in text.splitlines() if not l.startswith("#")]
    # 去掉末尾空行
    while lines and not lines[-1].strip():
        lines.pop()
    return "\n".join(lines).strip()


def _git(args: list[str]) -> tuple[int, str]:
    p = subprocess.run(
        ["git", *args],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    return p.returncode, p.stdout


def validate_message(sha: str, msg: str) -> CommitIssue:
    issues: list[str] = []
    if not msg.strip():
        return CommitIssue(sha=sha, header="", issues=["empty commit message"])

    lines = msg.splitlines()
    header = lines[0]

    if len(header) > 100:
        issues.append(f"header too long ({len(header)} > 100)")

    m = HEADER_RE.match(header)
    if not m:
        issues.append(
            "header format invalid; expected: <type>(<scope>): <subject>"
        )
        return CommitIssue(sha=sha, header=header, issues=issues)

    t = m.group("type")
    scope = m.group("scope")
    subject = m.group("subject")

    if t not in TYPES:
        issues.append(f"unknown type: {t!r}; allowed: {sorted(TYPES)}")
    if scope:
        # 可能多 scope 用逗号或斜杠
        for s in re.split(r"[,/]", scope):
            s = s.strip()
            if s and s not in SCOPES:
                issues.append(f"unknown scope: {s!r}; allowed: {sorted(SCOPES)}")
    if not subject.strip():
        issues.append("empty subject")
    if subject.endswith("."):
        issues.append("subject should not end with '.'")
    if subject[:1].isupper() and t != "revert":
        # 中文不算大写；这里只对 ASCII 大写做提示，非强制
        pass

    return CommitIssue(sha=sha, header=header, issues=issues)


def collect_messages(args) -> list[tuple[str, str]]:
    """返回 [(sha_or_label, msg), ...]"""
    if args.file:
        path = Path(args.file)
        if not path.is_absolute():
            path = REPO_ROOT / path
        return [("STAGED", _read_msg_from_file(path))]

    if args.staged:
        # pre-commit 钩子调用本脚本时，commit message 还不存在
        # （由 commit-msg 钩子专门校验，不必重复）
        # 此模式直接跳过：返回空列表 → 0 退出码（noop）
        return []

    if args.last:
        code, out = _git(["log", "-1", "--pretty=%H%n%B"])
        if code != 0:
            return []
        sha, _, body = out.strip().partition("\n")
        return [(sha, body.strip())]

    if args.range:
        code, out = _git(["log", "--reverse", "--pretty=%H", f"{args.range}..HEAD"])
        if code != 0:
            return []
        results = []
        for sha in [s for s in out.splitlines() if s.strip()]:
            code2, body = _git(["log", "-1", "--pretty=%B", sha])
            if code2 == 0:
                results.append((sha, body.strip()))
        return results

    # 默认：校验最近一次提交（如有）
    return collect_messages(argparse.Namespace(
        file=None, staged=False, last=True, range=None, json=args.json,
    ))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    g = parser.add_mutually_exclusive_group()
    g.add_argument("--staged", action="store_true", help="校验暂存区/COMMIT_EDITMSG")
    g.add_argument("--file", help="校验指定 commit message 文件（lefthook 传入）")
    g.add_argument("--last", action="store_true", help="校验最近一次 commit")
    g.add_argument("--range", help="校验 <ref>..HEAD 范围内的 commit")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    msgs = collect_messages(args)

    issues: list[CommitIssue] = []
    for sha, msg in msgs:
        issues.append(validate_message(sha, msg))

    failed = [i for i in issues if i.issues]

    if args.json:
        payload = {
            "check": "check_commit_convention",
            "tier": "T1",
            "category": "流程治理",
            "checked": len(issues),
            "failures": [asdict(i) for i in failed],
            "ok": not failed,
        }
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print(f"check_commit_convention (T1, 流程治理) — checked {len(issues)} message(s)")
        if not msgs:
            print("  (no message to check; ok)")
        for i in issues:
            mark = "✓" if not i.issues else "✘"
            short = (i.header[:80] + "…") if len(i.header) > 80 else i.header
            print(f"  {mark} {i.sha[:7]}  {short}")
            for s in i.issues:
                print(f"      - {s}")

    return 0 if not failed else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
