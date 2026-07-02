---
name: wms-issue-codex-exec
description: WMS Gitea issue 自动处理的 codex exec 两段式确认流程。用户要求不要用 tmux、直接 codex exec、先发方案提案、确认方案后执行、验收反馈不对要重新发修正方案，或处理 issue-agent 执行闭环时使用；不用于普通 worktree 子代理或无 issue 的本地开发。
---

# WMS Issue Codex Exec

替代旧 tmux 流程。核心规则：先提案，后执行；一次确认只对应一次方案；验收反馈不对时，重新提修正方案并等待新的“确认方案”。

## 先读

- `AGENTS.md`
- `docs/runbooks/gitea-issue-agent.md`
- 当前 issue 标题、正文、全部人工评论和附件
- 涉及目录的 `AGENTS.override.md`
- 涉及功能的用户故事、RTM、设计文档、接口或代码

## 流程

1. 读取 issue 与评论：使用 `tea api`，禁止裸 `curl`。
2. 核查代码：用 `rg` 找根因文件和调用链；不能只复述 issue。
3. 评论 `wms-issue-agent:proposal:v1` 方案提案，必须包含：
   - 问题复述和判断结论。
   - 根因文件和关键行号。
   - 相似 / 共性判断：影响哪些同类页面、组件、字段、流程或脚本；是否一起修；规则落到 prompt、skill、runbook、规范还是 T1 脚本。
   - 改动计划，按前端、后端、数据、文档、测试分层列出；不涉及的层写“不涉及”。
   - 验证计划，至少包含 `git diff --check`、`just gov-t1` 和任务相关测试；前端可见变更必须包含真实截图计划。
   - 风险、停止条件和需要用户补充的信息。
   - 明确等待用户回复裸一行 `确认方案`。
4. 只有在最新 proposal 之后出现人工评论且正文整行等于 `确认方案` 时，才执行。`/confirm`、`开始处理`、旧确认、否定句或 agent 自己的评论都不能触发。
5. 执行时新建独立 worktree 和分支，直接运行 `codex exec -C <worktree> - < <prompt>`；禁止 tmux、禁止交互式 Codex TUI、禁止在主工作区直接修 issue。
6. `codex exec` prompt 必须带上 issue、评论、附件 URL、根因文件、proposal URL、共性判断、写入范围、验证命令和禁止项。
7. 修复完成后回写 `wms-issue-agent:delivery:v1`：
   - PR 链接或本地提交状态。
   - 提交哈希。
   - 验证命令和退出码。
   - 前端或用户可见行为的截图附件 Markdown。
   - 本地测试环境重启、端口、URL 和版本校验证据。
   - 剩余风险和下一步。
8. 用户验收反馈“不对 / 不是这样 / 还缺 / 重新改 / 截图不对”等，视为新一轮反馈；必须评论 `wms-issue-agent:revision-proposal:v1` 修正方案，等待新的 `确认方案`，不得复用旧确认继续执行。

## 执行约束

- 禁止用 tmux 承载 issue 修复任务。
- 禁止因为 issue 有历史确认就自动继续跑；确认必须晚于最新 proposal 或 revision-proposal。
- 禁止只修当前页面而不判断共性；proposal 判定共性时，必须查共享组件、字段矩阵、prompt / skill / runbook 和治理脚本。
- 禁止把“已创建 PR”写成“已完成”；没有测试、截图和回写证据就只能写阻塞或待验收。
- 禁止自动合并 PR；issue 关闭后的合并规则仍按 runbook 执行。
- 新增字段、状态、角色、模块、业务默认值或跨模块语义变化时，停止并向用户确认。

## 最小检查

- skill 结构：`python3 /home/test1/.codex/skills/yao-meta-skill/scripts/validate_skill.py .agents/skills/wms-issue-codex-exec`
- 资源边界：`python3 /home/test1/.codex/skills/yao-meta-skill/scripts/resource_boundary_check.py .agents/skills/wms-issue-codex-exec`
- 触发评估：`python3 /home/test1/.codex/skills/yao-meta-skill/scripts/trigger_eval.py --description-file .agents/skills/wms-issue-codex-exec/SKILL.md --cases .agents/skills/wms-issue-codex-exec/evals/trigger_cases.json --semantic-config .agents/skills/wms-issue-codex-exec/evals/semantic_config.json`
