"""Proposal selection and rendering for issue_runner."""
from datetime import datetime
from typing import Any

from issue_runner import *  # noqa: F403

def latest_human_time(comments: list[dict[str, Any]]) -> datetime | None:
    times = [parse_time(str(c["created_at"])) for c in comments if not is_agent_comment(c)]
    return max(times) if times else None

def latest_actionable_human_time(comments: list[dict[str, Any]]) -> datetime | None:
    times = [
        parse_time(str(c["created_at"]))
        for c in comments
        if not is_status_comment(c) and not is_obsolete_control_comment(c) and not is_confirm_comment(c)
    ]
    return max(times) if times else None

def latest_status_time(comments: list[dict[str, Any]]) -> datetime | None:
    times = [parse_time(str(c["created_at"])) for c in comments if is_status_comment(c)]
    return max(times) if times else None

def build_proposal_comment(issue: dict[str, Any], comments: list[dict[str, Any]], *, revision: bool = False) -> str:
    labels = ", ".join(label["name"] for label in issue.get("labels") or []) or "无"
    scope = "待确认"
    text = f"{issue.get('title', '')} {issue.get('body', '')} {comment_summary(comments, limit=10)}"
    code_context = code_context_summary(issue, comments)
    commonality = commonality_summary(issue, comments)
    if "菜单" in text or "导航" in text:
        scope = "可能是管理端左侧导航 / 菜单交互"
    conclusion = "需要补充"
    confidence = "中"
    reason = "issue 描述较短，缺少截图、页面入口或期望效果，直接开发容易修错位置。"
    action = "建议先补充截图或说明具体页面；如果确认是管理端菜单视觉问题，再按低风险 UI 修复执行。"
    if has_selection_state_signal(text):
        scope = "可能是管理端 DataGrid 勾选 / 全选状态交互"
        conclusion = "建议执行"
        confidence = "中高"
        reason = (
            "issue 已指向可复现的选择状态异常，应先核查 DataGrid 与页面 "
            "selectedRowKeys / onSelectedRowKeysChange / 自动首选第一条逻辑。"
        )
        action = (
            "回复裸一行 `确认方案` 后执行最小修复；执行前先定位选择状态所有者，"
            "确认是否存在把多选数组压成单选或取消后自动回填第一条的问题。"
        )
    elif any(word in text for word in ("报错", "失败", "打不开", "不能", "无法")):
        conclusion = "建议执行"
        confidence = "中"
        reason = "描述指向可复现故障，优先用日志、接口或页面复现定位。"
        action = "回复裸一行 `确认方案` 后执行最小修复；如复现信息不足，执行时会先停止补问。"
    elif any(word in text for word in ("截图", "图片", "红框", "圈", "页面")):
        conclusion = "建议执行"
        confidence = "中"
        reason = "已有截图或页面位置补充，可以按当前范围做最小修复。"
        action = "回复裸一行 `确认方案` 后执行最小修复；未确认前继续只评论判断。"
    elif any(word in text for word in ("新增", "增加", "字段", "状态", "流程", "规则")):
        conclusion = "需要人工决策"
        confidence = "中"
        reason = "可能引入字段、状态、流程或业务规则变化，不能直接由 agent 拍板。"
        action = "先补业务边界和验收标准，再决定是否确认执行。"
    marker = REVISION_PROPOSAL_MARKER if revision else PROPOSAL_MARKER
    title = "修正方案" if revision else "方案提案"
    feedback_note = "\n本轮是用户验收反馈后的修正方案；旧确认不得复用。\n" if revision else ""
    return f"""{marker}
## WMS Issue Agent {title}

- Issue：#{issue["number"]} {issue["title"]}
- 作者：{(issue.get("user") or {}).get("login", "unknown")}
- 标签：{labels}
- 当前状态：open
- 可能影响范围：{scope}
- 结论：{conclusion}
- 置信度：{confidence}
{feedback_note}

### 问题复述

{short(issue.get("body") or issue.get("title") or "", 800)}

### 评论摘要

{comment_summary(comments)}

### 截图 / 附件

{attachment_summary(comments, issue=issue)}

### 根因文件和代码核查

{code_context}

### 相似 / 共性问题判断

{commonality}

### 改动计划

- 前端：按根因文件定位；若是用户可见行为，补真实页面验证和截图。
- 后端：暂未发现必须改动；如果执行中发现 API / 数据模型缺口，停止并重新提案。
- 数据：暂未发现必须改动；新增字段、状态、角色、模块或业务默认值必须停止确认。
- 文档：如共性成立，更新对应 prompt / runbook / skill / 规范；能脚本化的补治理脚本。
- 测试：先补最小回归测试，再做实现。

### 判断依据

- {reason}
- 上方代码核查是本轮判断的仓库证据；如果没有命中代码，执行前必须先补充复现信息。
- 当前步骤只做判断和确认，不改代码、不提交、不推送。

### 预计影响范围

- 前端：{scope}
- 后端：暂未发现必须改动
- 文档 / 测试：按实际修复范围补最小验证

### 建议动作

{action}

### 验证要求

- 至少运行 `git diff --check` 和 `just gov-t1`。
- 涉及前端或用户可见行为时，必须截图、重启本地测试前后端，并校验运行版本是本次修复提交。

### 风险与停止条件

- 如果需要新增字段、状态、角色、模块或业务默认值，必须停止并向用户确认。
- 如果复现信息不足，必须先补信息，不能靠猜测开发。

### 请确认

- 同意按上述方案执行：回复裸一行 `确认方案`
- 需要补充：直接继续评论需求细节或截图
- 暂不执行：评论 `/reject`
"""

def build_fix_prompt(issue: dict[str, Any], comments: list[dict[str, Any]]) -> str:
    proposal = latest_marker_comment(comments, (PROPOSAL_MARKER, REVISION_PROPOSAL_MARKER))
    proposal_url = str(proposal.get("html_url") or "") if proposal else ""
    commonality = commonality_summary(issue, comments)
    return f"""请处理 Gitea issue #{issue["number"]}：{issue["title"]}

来源：{issue.get("html_url", "")}
确认对应方案：{proposal_url or "未提供"}

用户已在最新方案后回复裸一行“确认方案”。请按 WMS 仓库规则处理：
1. 使用 `wms-issue-codex-exec`、`wms-loop-engineering` 和 `wms-execution-retrospective` 执行；先按 proposal 的“相似 / 共性问题判断”核查同类页面、组件、字段、流程和治理脚本，若共性成立，必须同步补规则、脚本或矩阵，不能只修一个页面。
2. 当前命令已由 issue-agent 在 issue 专属 worktree 中用 `codex exec` 直接启动；先运行 `pwd`、`git status --short --branch` 和 `git worktree list` 核对，禁止修改主工作区。
3. 只修复 issue 指向的问题。新增字段、状态、角色、模块或业务默认值时停止并向用户确认。
4. 完成后运行相关测试，至少 `git diff --check` 和 `just gov-t1`。
5. 涉及前端或用户可见行为时，禁止在子 worktree 中启动或占用 9002；9002 只允许主工作区固定会话 `wms-web-admin-9002`。issue worktree 预览必须使用本 prompt 末尾分配的 `WMS_ISSUE_WEB_PORT`，执行 `just dev-web-worktree-restart <worktree> $WMS_ISSUE_WEB_PORT` 并用 `just dev-web-worktree-verify <worktree> $WMS_ISSUE_WEB_PORT` 校验进程 cwd 与 worktree 一致。纯前端修复默认共用主后端 `18080`；改后端 / API / 数据库时必须使用本 prompt 末尾分配的 `WMS_ISSUE_API_PORT`，执行 `just dev-api-worktree-restart <worktree> $WMS_ISSUE_API_PORT` 并用 `just dev-api-worktree-verify <worktree> $WMS_ISSUE_API_PORT` 校验 `/healthz`、LAN URL 和进程一致性。需要真实截图或重启证据时，合并到主工作区后由主代理运行 `just dev-web-restart` 和 `just dev-web-verify`，后端按仓库现有运行方式启动并检查 `/healthz`。重启后必须校验运行的是本次修复版本：记录当前提交哈希，并用 `just dev-web-verify` 输出、页面可见变更、接口响应版本字段或等价证据证明不是旧进程缓存；把端口、URL、提交哈希、进程一致性和版本校验结果写入 issue。
6. 如果 issue 评论包含截图 / 附件，必须先打开或下载附件辅助定位，不能只按文字猜测。
7. 涉及前端或用户可见行为时，必须采集真实前端截图；必须用 `POST /repos/{{owner}}/{{repo}}/issues/<编号>/assets` 把截图上传为 Gitea 附件，并用 Markdown 图片评论到 issue；不能只写本地路径。
8. 当前可用截图 / 附件如下：
{attachment_summary(comments, issue=issue, limit=10)}
9. 读取 Gitea issue、评论或附件元数据必须使用 `tea api`，例如 `tea api /repos/{{owner}}/{{repo}}/issues/{issue["number"]}`；禁止裸 `curl` 访问 Gitea API。
10. 当前暂停 Gitea PR：允许在 issue 专属 worktree 的本地分支上提交，禁止推送远端、禁止创建 Gitea PR、禁止自行合并到主工作区。
11. 最后必须评论 `{DELIVERY_MARKER}` delivery 信息：本地 worktree、分支、提交哈希或 diff 状态、验证结果、截图证据、本地测试环境重启结果、codex exec 日志位置、本地合并前置条件和剩余风险，并明确下一步是“等待主代理本地 review 后合并”或“阻塞”。
12. 如果用户验收反馈不对，不得继续复用本次确认；必须等待 issue-agent 重新生成修正方案并由用户再次回复“确认方案”。

端口和进程一致性规则：
- 主工作区固定：前端 9002，后端 18080。
- issue worktree 前端：9003-9099，每个 worktree 独立端口。
- issue worktree 后端：18081-18099；只有改后端 / API / 数据库时启动，纯前端修复共用 18080。
- 端口验证必须证明端口对应进程来自本 issue worktree；不能只证明端口可访问。
- delivery 必须写明本轮使用的前端端口、后端端口或共用后端、LAN URL、`/healthz` 结果和进程一致性校验结果。

当前相似 / 共性问题判断：
{commonality}

Issue 正文：
{issue.get("body") or ""}

近期人工评论：
{comment_summary(comments, limit=10)}
"""

def choose_action(
    issue: dict[str, Any],
    comments: list[dict[str, Any]],
) -> Action | None:
    human_time = latest_actionable_human_time(comments)
    proposal = latest_marker_comment(comments, (PROPOSAL_MARKER, REVISION_PROPOSAL_MARKER))
    proposal_time = parse_time(str(proposal["created_at"])) if proposal else None
    exec_time = latest_marker_time(comments, EXEC_MARKER)
    status_time = latest_status_time(comments)
    confirm = latest_confirm_after(comments, proposal_time) if proposal_time else None
    key = confirm_key(confirm) if confirm else None
    confirmed = confirm is not None and not confirmation_consumed(int(issue["number"]), key or "")
    if status_time and not confirmed and (human_time is None or human_time <= status_time):
        return None
    if proposal_time is None:
        return Action(issue=issue, kind="proposal", body=build_proposal_comment(issue, comments))
    if has_reject_after(comments, proposal_time):
        return None
    if human_time and human_time > proposal_time:
        if exec_time and exec_time > proposal_time:
            return Action(issue=issue, kind="revision-proposal", body=build_proposal_comment(issue, comments, revision=True))
        return Action(issue=issue, kind="proposal", body=build_proposal_comment(issue, comments))
    if confirmed:
        if exec_time is None or exec_time < proposal_time:
            return Action(issue=issue, kind="exec", body=build_fix_prompt(issue, comments), confirm_key=key)
    if exec_time and exec_time > proposal_time:
        return None
    return None
