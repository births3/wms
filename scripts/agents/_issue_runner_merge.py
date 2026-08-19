"""Local merge helpers for issue_runner."""
import re
from typing import Any

from issue_runner import *  # noqa: F403

def pr_mentions_issue(pr: dict[str, Any], issue_number: int) -> bool:
    text = text_blob(pr.get("title"), pr.get("body"))
    return bool(re.search(rf"(?:issue|关联|关闭|close[sd]?)\s*#?{issue_number}\b", text, re.I))

def has_merge_blocker(text: str) -> bool:
    return any(word in text for word in ("/reject", "不要合并", "暂不合并", "合并阻塞", "blocked", "BLOCKED"))

def has_merge_evidence(text: str) -> bool:
    return "验证" in text and ("截图" in text or "附件" in text) and ("重启" in text or "healthz" in text)

def is_auto_merge_branch(head_ref: str, issue_number: int) -> bool:
    return head_ref.startswith("agent/") or head_ref.startswith(f"fix/issue-{issue_number}-")

def current_branch() -> str:
    return run(["git", "branch", "--show-current"]).strip()

def pull_is_merged(pr: dict[str, Any]) -> bool:
    return bool(pr.get("merged") or pr.get("merged_at") or pr.get("merge_commit_sha"))

def merge_verification_error(pr: dict[str, Any]) -> str | None:
    if pull_is_merged(pr):
        return None
    return (
        f"state={pr.get('state')}, merged={pr.get('merged')}, "
        f"mergeable={pr.get('mergeable')}, merged_at={pr.get('merged_at')}, "
        f"merge_commit_sha={pr.get('merge_commit_sha')}"
    )

def merge_blockers(issue: dict[str, Any], comments: list[dict[str, Any]], pr: dict[str, Any]) -> list[str]:
    human_comments = [body_of(c) for c in comments if "wms-issue-agent:" not in body_of(c)]
    text = text_blob(issue.get("body"), pr.get("body"), *human_comments)
    head_ref = str((pr.get("head") or {}).get("ref") or "")
    base_ref = str((pr.get("base") or {}).get("ref") or "")
    blockers: list[str] = []
    if issue.get("state") != "closed":
        blockers.append("issue 未关闭")
    if base_ref and base_ref != current_branch():
        blockers.append(f"PR base 不是当前工作分支：{base_ref}")
    if has_active_merge_marker(comments):
        if pull_is_merged(pr):
            blockers.append("issue 已有合并 marker")
        else:
            blockers.append("issue 有未纠正的合并 marker 但 PR 未合并")
    failure_time = latest_marker_time(comments, MERGE_FAILED_MARKER)
    if failure_time and not has_command_after(comments, failure_time, MERGE_RETRY_COMMAND):
        blockers.append(f"已有自动合并失败 marker，需人工评论 {MERGE_RETRY_COMMAND} 后重试")
    if pr.get("state") != "open":
        blockers.append("PR 不是 open")
    if pull_is_merged(pr):
        blockers.append("PR 已合并")
    if not pr.get("mergeable"):
        blockers.append("PR 不可自动合并")
    if not is_auto_merge_branch(head_ref, int(issue["number"])):
        blockers.append("PR 分支不是 agent/* 或 fix/issue-<编号>-*")
    if has_merge_blocker(text):
        blockers.append("存在阻塞或拒绝合并评论")
    if not has_merge_evidence(text):
        blockers.append("缺少验证、截图或重启证据")
    return blockers

def extract_local_merge_branches(text: str, issue_number: int) -> list[str]:
    branches: list[str] = []
    for match in LOCAL_BRANCH_PATTERN.finditer(text):
        branch = match.group(0).strip("`'\"，。,.；;")
        if branch in branches:
            continue
        if is_auto_merge_branch(branch, issue_number):
            branches.append(branch)
    return branches

def latest_local_delivery_branches(issue_number: int, comments: list[dict[str, Any]]) -> list[str]:
    deliveries = [comment for comment in comments if DELIVERY_MARKER in body_of(comment)]
    for comment in sorted(deliveries, key=lambda c: parse_time(str(c["created_at"])), reverse=True):
        branches = extract_local_merge_branches(body_of(comment), issue_number)
        if branches:
            return branches
    return []

def local_branch_exists(branch: str) -> bool:
    result = subprocess.run(
        ["git", "show-ref", "--verify", "--quiet", f"refs/heads/{branch}"],
        cwd=REPO_ROOT,
        check=False,
    )
    return result.returncode == 0

def local_branch_merged(branch: str) -> bool:
    result = subprocess.run(
        ["git", "merge-base", "--is-ancestor", branch, "HEAD"],
        cwd=REPO_ROOT,
        check=False,
    )
    return result.returncode == 0

def workspace_clean() -> bool:
    return run(["git", "status", "--porcelain"]).strip() == ""

def local_merge_blockers(
    issue: dict[str, Any],
    comments: list[dict[str, Any]],
    branch: str,
    *,
    branch_exists: bool,
    branch_merged: bool,
    workspace_clean: bool,
    current_ref: str | None = None,
) -> list[str]:
    issue_number = int(issue["number"])
    latest_delivery = latest_marker_comment(comments, (DELIVERY_MARKER,))
    human_comments = [body_of(c) for c in comments if not is_agent_comment(c)]
    text = text_blob(issue.get("body"), body_of(latest_delivery or {}), *human_comments)
    blockers: list[str] = []
    if issue.get("state") != "closed":
        blockers.append("issue 未关闭")
    if not latest_delivery:
        blockers.append("缺少本地交付 delivery")
    if has_active_merge_marker(comments):
        blockers.append("issue 已有合并 marker")
    failure_time = latest_marker_time(comments, MERGE_FAILED_MARKER)
    if failure_time and not has_command_after(comments, failure_time, MERGE_RETRY_COMMAND):
        blockers.append(f"已有自动合并失败 marker，需人工评论 {MERGE_RETRY_COMMAND} 后重试")
    if not is_auto_merge_branch(branch, issue_number):
        blockers.append("本地分支不是 agent/* 或 fix/issue-<编号>-*")
    if current_ref and current_ref == branch:
        blockers.append("当前工作分支就是待合并分支")
    if not branch_exists:
        blockers.append(f"本地分支不存在：{branch}")
    if has_merge_blocker(text):
        blockers.append("存在阻塞或拒绝合并评论")
    if not has_merge_evidence(text):
        blockers.append("缺少验证、截图或重启证据")
    if branch_exists and not branch_merged and not workspace_clean:
        blockers.append("主工作区存在未提交改动")
    return blockers

def abort_local_merge() -> None:
    subprocess.run(["git", "merge", "--abort"], cwd=REPO_ROOT, check=False, capture_output=True, text=True)

def merge_local_branch_checked(issue_number: int, branch: str) -> str:
    try:
        run(["git", "merge", "--no-ff", "--no-commit", branch])
    except Exception:
        abort_local_merge()
        raise
    try:
        for command in LOCAL_MERGE_VALIDATIONS:
            run(list(command))
        run(["git", "commit", "-m", f"杂项(issue)：合并 closed issue #{issue_number} 本地分支"])
    except Exception:
        abort_local_merge()
        raise
    return run(["git", "rev-parse", "--short", "HEAD"]).strip()

def build_local_merge_comment(issue_number: int, branch: str, merge_commit: str, *, already_merged: bool = False) -> str:
    state = "已在当前工作分支中" if already_merged else "已由本地合并队列合并到当前工作分支"
    return (
        f"{MERGE_MARKER}\n"
        f"issue 已关闭，本地分支 `{branch}` {state}。\n"
        f"- issue：#{issue_number}\n"
        f"- 分支：`{branch}`\n"
        f"- 合并提交：`{merge_commit}`\n"
        f"- 触发：closed issue #{issue_number}\n"
        "- 远端：未推送；Gitea PR 合并仍暂停。\n"
    )

def build_local_merge_failed_comment(issue_number: int, branch: str, error: str) -> str:
    return (
        f"{MERGE_FAILED_MARKER}\n"
        "本地合并队列已停止：合并、验证或提交失败，因此没有写入合并 marker。\n"
        f"- issue：#{issue_number}\n"
        f"- 分支：`{branch}`\n"
        f"- 失败原因：{short(error, 500)}\n"
        f"- 处理方式：修复本地分支或主工作区后，人工评论 `{MERGE_RETRY_COMMAND}` 再允许 watcher 重试。\n"
    )

def run_local_merge_closed(args: argparse.Namespace) -> int:
    issues = [get_issue(args.issue)] if args.issue else list_closed_issues(args.limit)
    clean = workspace_clean()
    current_ref = current_branch()
    for issue in issues:
        issue_number = int(issue["number"])
        comments = get_comments(issue_number)
        branches = latest_local_delivery_branches(issue_number, comments)
        if not branches:
            print(f"skip: issue #{issue_number} 未找到本地 delivery 分支", flush=True)
            continue
        for branch in branches:
            exists = local_branch_exists(branch)
            merged = exists and local_branch_merged(branch)
            blockers = local_merge_blockers(
                issue,
                comments,
                branch,
                branch_exists=exists,
                branch_merged=merged,
                workspace_clean=clean,
                current_ref=current_ref,
            )
            if blockers:
                print(f"skip: issue #{issue_number} 分支 {branch}：{'; '.join(blockers)}", flush=True)
                continue
            if not args.apply:
                print(f"dry-run: 将本地合并 issue #{issue_number} 分支 {branch}", flush=True)
                return 0
            if merged:
                merge_commit = run(["git", "rev-parse", "--short", "HEAD"]).strip()
                post_comment(issue_number, build_local_merge_comment(issue_number, branch, merge_commit, already_merged=True))
                print(f"already-merged: issue #{issue_number} 分支 {branch}", flush=True)
                return 0
            try:
                merge_commit = merge_local_branch_checked(issue_number, branch)
            except Exception as exc:  # noqa: BLE001
                post_comment(issue_number, build_local_merge_failed_comment(issue_number, branch, str(exc)))
                print(f"local-merge-failed: issue #{issue_number} 分支 {branch}：{short(str(exc), 240)}", flush=True)
                return 2
            post_comment(issue_number, build_local_merge_comment(issue_number, branch, merge_commit))
            print(f"local-merged: issue #{issue_number} 分支 {branch}", flush=True)
            return 0
    print("no-action: 没有可本地合并的 closed issue 分支", flush=True)
    return 0
