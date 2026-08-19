"""Self-test for issue_runner."""
import tempfile
from pathlib import Path

from issue_runner import *  # noqa: F403

def self_test() -> int:
    global CONSUMED_CONFIRMATIONS_FILE
    CONSUMED_CONFIRMATIONS_FILE = Path(tempfile.mkdtemp(prefix="wms-issue-agent-test-")) / "consumed.json"
    issue = {"number": 7, "title": "测试", "body": "正文", "labels": [], "user": {"login": "u"}}
    assert display_path(REPO_ROOT / "justfile") == "justfile"
    assert display_path(Path("/tmp/wms-issue-agent-preview.txt")) == "/tmp/wms-issue-agent-preview.txt"
    worktree_path, branch = issue_worktree(7, "20260702010101")
    assert worktree_path.name == "wms-agent-issue-7-20260702010101"
    assert branch == "fix/issue-7-20260702010101"
    tmp_env = Path(tempfile.mkdtemp(prefix="wms-issue-agent-env-")) / "env"
    tmp_env.write_text(
        "\n".join(
            [
                "# comment",
                "export https_proxy=http://127.0.0.1:7894",
                "http_proxy='http://127.0.0.1:7894'",
                "IGNORED=value",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    assert read_issue_agent_env(tmp_env) == {
        "https_proxy": "http://127.0.0.1:7894",
        "http_proxy": "http://127.0.0.1:7894",
    }
    command = build_codex_command(
        worktree_path,
        DEFAULT_OUT_DIR / "issue-7-exec.txt",
        DEFAULT_OUT_DIR / "issue-7-exec.log",
    )
    assert f"-C {quote(str(worktree_path))}" in command
    assert f"-C {quote(str(REPO_ROOT))} " not in command
    t0 = "2026-07-01T00:00:00Z"
    t1 = "2026-07-01T00:01:00Z"
    comments = [{"created_at": t0, "body": build_proposal_comment(issue, [])}]
    assert "### 根因文件和代码核查" in comments[0]["body"]
    assert "### 相似 / 共性问题判断" in comments[0]["body"]
    assert "暂未证明是共性问题" in comments[0]["body"]
    assert "/confirm" not in comments[0]["body"]
    assert "开始处理" not in comments[0]["body"]
    popup_issue = {**issue, "title": "点击按钮弹出新窗口时点击外边区域不会关闭"}
    popup_proposal = build_proposal_comment(popup_issue, [])
    assert "弹层 / 弹窗关闭交互" in popup_proposal
    assert "是否一起修改" in popup_proposal
    assert "prompt / runbook / skill / 规范" in popup_proposal
    assert "弹层 / 弹窗关闭交互" in build_fix_prompt(popup_issue, [])
    selection_issue = {
        **issue,
        "title": "全局勾选按钮再点中已勾选按钮，取消勾选，不要自动勾选第一个",
        "body": "全局勾选按钮再点中已勾选按钮，取消勾选，不要自动勾选第一个",
    }
    selection_proposal = build_proposal_comment(selection_issue, [])
    assert "DataGrid 选择状态一致性" in selection_proposal
    assert "selectedRowKeys" in selection_proposal
    assert "可能是管理端 DataGrid 勾选 / 全选状态交互" in selection_proposal
    assert "管理端动作入口一致性" not in selection_proposal
    assert "管理端菜单视觉问题" not in selection_proposal
    assert choose_action(issue, []).kind == "proposal"
    assert choose_action(issue, comments) is None
    confirmed = [*comments, {"created_at": t1, "body": "/confirm"}]
    assert choose_action(issue, confirmed) is None
    confirmed_zh = [*comments, {"created_at": t1, "body": "确认方案"}]
    action = choose_action(issue, confirmed_zh)
    assert action.kind == "exec"
    assert action.confirm_key
    mark_confirmation_consumed(7, action.confirm_key)
    assert choose_action(issue, confirmed_zh) is None
    later_confirmed_zh = [*comments, {"created_at": "2026-07-01T00:02:00Z", "body": "确认方案"}]
    assert choose_action(issue, later_confirmed_zh).kind == "exec"
    multiline_confirm = [*comments, {"created_at": t1, "body": "确认方案\n补充：需要截图"}]
    assert choose_action(issue, multiline_confirm).kind == "proposal"
    detail_then_confirm = [
        *comments,
        {"created_at": t1, "body": "补充：需要截图"},
        {"created_at": "2026-07-01T00:02:00Z", "body": "确认方案"},
    ]
    assert choose_action(issue, detail_then_confirm).kind == "proposal"
    start_zh = [*comments, {"created_at": t1, "body": "开始处理"}]
    assert choose_action(issue, start_zh) is None
    mentioned = [*comments, {"created_at": t1, "body": "不要 /confirm，先补截图"}]
    assert choose_action(issue, mentioned).kind == "proposal"
    mentioned_zh = [*comments, {"created_at": t1, "body": "不要确认方案，先补截图"}]
    assert choose_action(issue, mentioned_zh).kind == "proposal"
    refreshed = [*mentioned, {"created_at": "2026-07-01T00:02:00Z", "body": build_proposal_comment(issue, mentioned)}]
    assert choose_action(issue, refreshed) is None
    sent = [*confirmed_zh, {"created_at": "2026-07-01T00:02:00Z", "body": EXEC_MARKER}]
    assert choose_action(issue, sent) is None
    failed_exec = [
        *sent,
        {
            "created_at": "2026-07-01T00:03:00Z",
            "body": f"{STATUS_CORRECTION_MARKER}\n`codex exec` 执行超时，未形成完成闭环。",
        },
    ]
    assert choose_action(issue, failed_exec) is None
    failed_exec_with_feedback = [
        *failed_exec,
        {"created_at": "2026-07-01T00:04:00Z", "body": "重新执行，继续处理"},
    ]
    assert choose_action(issue, failed_exec_with_feedback).kind == "revision-proposal"
    failed_exec_refreshed = [
        *failed_exec_with_feedback,
        {
            "created_at": "2026-07-01T00:05:00Z",
            "body": build_proposal_comment(issue, failed_exec_with_feedback, revision=True),
        },
    ]
    assert choose_action(issue, failed_exec_refreshed) is None
    delivered = [
        *sent,
        {
            "created_at": "2026-07-01T00:03:00Z",
            "body": "已按确认处理 issue #7，最终交付 PR 为 #9。PR 合并前置条件：等待用户确认合并。",
        },
    ]
    assert choose_action(issue, delivered) is None
    after_delivery = [*delivered, {"created_at": "2026-07-01T00:04:00Z", "body": "补充：按钮还缺一个"}]
    assert choose_action(issue, after_delivery).kind == "revision-proposal"
    after_exec = [*sent, {"created_at": "2026-07-01T00:03:00Z", "body": "不对，还缺下拉异常"}]
    assert choose_action(issue, after_exec).kind == "revision-proposal"
    after_exec_refreshed = [
        *after_exec,
        {"created_at": "2026-07-01T00:04:00Z", "body": build_proposal_comment(issue, after_exec)},
    ]
    assert choose_action(issue, after_exec_refreshed) is None
    after_exec_confirmed = [*after_exec_refreshed, {"created_at": "2026-07-01T00:05:00Z", "body": "确认方案"}]
    assert choose_action(issue, after_exec_confirmed).kind == "exec"
    rejected = [*comments, {"created_at": t1, "body": "/reject 不执行 /confirm"}]
    assert choose_action(issue, rejected) is None
    with_asset = [
        *comments,
        {
            "created_at": t1,
            "body": "见截图",
            "user": {"login": "u"},
            "assets": [{"name": "image.png", "browser_download_url": "http://gitea/attachments/a"}],
        },
    ]
    assert choose_action(issue, [*with_asset, {"created_at": "2026-07-01T00:02:00Z", "body": "确认方案"}]).kind == "proposal"
    with_asset_refreshed = [
        *with_asset,
        {"created_at": "2026-07-01T00:02:00Z", "body": build_proposal_comment(issue, with_asset)},
        {"created_at": "2026-07-01T00:03:00Z", "body": "确认方案"},
    ]
    prompt = choose_action(issue, with_asset_refreshed).body
    assert "http://gitea/attachments/a" in prompt
    assert "上传为 Gitea 附件" in prompt
    assert "wms-execution-retrospective" in prompt
    assert "共性问题" in prompt
    assert DELIVERY_MARKER in prompt
    assert "tea api" in prompt
    assert "禁止裸 `curl`" in prompt
    assert "WMS_ISSUE_WEB_PORT" in prompt
    assert "WMS_ISSUE_API_PORT" in prompt
    assert "进程一致性" in prompt
    runtime_prompt = Path(tempfile.mkdtemp(prefix="wms-issue-agent-runtime-")) / "prompt.txt"
    runtime_prompt.write_text("base", encoding="utf-8")
    append_runtime_context(runtime_prompt, Path("/tmp/wms-agent-issue-7-demo"), 9003, 18081)
    runtime_text = runtime_prompt.read_text(encoding="utf-8")
    assert "WMS_ISSUE_WEB_PORT=9003" in runtime_text
    assert "WMS_ISSUE_API_PORT=18081" in runtime_text
    assert "dev-api-worktree-verify" in runtime_text
    issue_asset = {
        **issue,
        "assets": [{"name": "issue.png", "browser_download_url": "http://gitea/attachments/issue"}],
    }
    assert "http://gitea/attachments/issue" in build_proposal_comment(issue_asset, [])
    assert "http://gitea/attachments/issue" in build_fix_prompt(issue_asset, [])
    closed_issue = {"number": 8, "state": "closed", "body": "已验收"}
    pr = {
        "number": 9,
        "state": "open",
        "merged": False,
        "mergeable": True,
        "title": "修复：issue #8",
        "body": "关联 issue #8\n验证：通过\n截图证据：已上传附件\n后端重启：/healthz ok",
        "head": {"ref": "agent/issue-8-demo"},
    }
    assert pr_mentions_issue(pr, 8)
    assert not pr_mentions_issue({**pr, "title": "普通变更", "body": "普通 PR #8，不是 issue 关联"}, 8)
    assert merge_blockers(closed_issue, [], pr) == []
    assert merge_blockers(closed_issue, [], {**pr, "head": {"ref": "fix/issue-8-datagrid-views"}}) == []
    assert "PR 分支不是 agent/* 或 fix/issue-<编号>-*" in merge_blockers(
        closed_issue,
        [],
        {**pr, "head": {"ref": "fix/issue-9-other"}},
    )
    assert f"PR base 不是当前工作分支：other-branch" in merge_blockers(
        closed_issue,
        [],
        {**pr, "base": {"ref": "other-branch"}},
    )
    assert pull_is_merged({**pr, "merged": True})
    assert pull_is_merged({**pr, "merged_at": "2026-07-01T00:00:00Z"})
    assert pull_is_merged({**pr, "merge_commit_sha": "abc"})
    assert not pull_is_merged(pr)
    assert merge_verification_error(pr) == (
        "state=open, merged=False, mergeable=True, merged_at=None, merge_commit_sha=None"
    )
    false_merge_marker = [{"created_at": t1, "body": MERGE_MARKER}]
    assert "issue 有未纠正的合并 marker 但 PR 未合并" in merge_blockers(closed_issue, false_merge_marker, pr)
    corrected_marker = [
        *false_merge_marker,
        {"created_at": "2026-07-01T00:02:00Z", "body": MERGE_CORRECTION_MARKER},
    ]
    assert "issue 有未纠正的合并 marker 但 PR 未合并" not in merge_blockers(
        closed_issue,
        corrected_marker,
        pr,
    )
    failed_marker = [
        *corrected_marker,
        {"created_at": "2026-07-01T00:03:00Z", "body": MERGE_FAILED_MARKER},
    ]
    assert f"已有自动合并失败 marker，需人工评论 {MERGE_RETRY_COMMAND} 后重试" in merge_blockers(
        closed_issue,
        failed_marker,
        pr,
    )
    retried = [
        *failed_marker,
        {"created_at": "2026-07-01T00:04:00Z", "body": MERGE_RETRY_COMMAND},
    ]
    assert f"已有自动合并失败 marker，需人工评论 {MERGE_RETRY_COMMAND} 后重试" not in merge_blockers(
        closed_issue,
        retried,
        pr,
    )
    assert "缺少验证、截图或重启证据" in merge_blockers(closed_issue, [], {**pr, "body": "关联 issue #8"})
    assert merge_blockers(closed_issue, [{"created_at": t1, "body": f"{PROPOSAL_MARKER}\n评论 `/reject`"}], pr) == []
    assert "存在阻塞或拒绝合并评论" in merge_blockers(
        closed_issue,
        [{"created_at": t1, "body": "不要合并"}],
        pr,
    )
    delivery_comment = {
        "created_at": t1,
        "body": (
            f"{DELIVERY_MARKER}\n"
            "- 分支：`fix/issue-8-local-merge`\n"
            "- 验证：`just gov-t1` 通过\n"
            "- 截图证据：已上传附件\n"
            "- 本地测试环境重启结果：`/healthz` ok\n"
            "- 下一步：等待主代理本地 review 后合并\n"
        ),
    }
    assert extract_local_merge_branches(body_of(delivery_comment), 8) == ["fix/issue-8-local-merge"]
    assert latest_local_delivery_branches(8, [delivery_comment]) == ["fix/issue-8-local-merge"]
    assert local_merge_blockers(
        closed_issue,
        [delivery_comment],
        "fix/issue-8-local-merge",
        branch_exists=True,
        branch_merged=False,
        workspace_clean=True,
    ) == []
    assert "主工作区存在未提交改动" in local_merge_blockers(
        closed_issue,
        [delivery_comment],
        "fix/issue-8-local-merge",
        branch_exists=True,
        branch_merged=False,
        workspace_clean=False,
    )
    assert "缺少验证、截图或重启证据" in local_merge_blockers(
        closed_issue,
        [{**delivery_comment, "body": f"{DELIVERY_MARKER}\n- 分支：`fix/issue-8-local-merge`"}],
        "fix/issue-8-local-merge",
        branch_exists=True,
        branch_merged=False,
        workspace_clean=True,
    )
    assert MERGE_MARKER in build_local_merge_comment(8, "fix/issue-8-local-merge", "abc123")
    print("self-test: ok", flush=True)
    return 0
