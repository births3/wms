---
name: wms-issue-codex-exec
description: WMS Gitea issue 的 codex exec 两段式确认流程。用户要求先发方案、确认后执行、不用 tmux、验收反馈重提案，或处理 issue-agent 闭环时使用。
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
3. 评论 `wms-issue-agent:proposal:v1`，包含问题判断、根因文件、共性判断、分层改动计划、验证计划、风险、停止条件，并等待裸一行 `确认方案`。
4. 只有在最新 proposal 之后出现人工评论且正文整行等于 `确认方案` 时，才执行。`/confirm`、`开始处理`、旧确认、否定句或 agent 自己的评论都不能触发。
5. 执行时新建独立 worktree 和分支，直接运行 `codex exec -C <worktree> - < <prompt>`；禁止 tmux、禁止交互式 Codex TUI、禁止在主工作区直接修 issue。
6. `codex exec` prompt 必须带上 issue、评论、附件 URL、根因文件、proposal URL、共性判断、写入范围、验证命令和禁止项。
7. 执行前消费本次确认，落到 `.codex/issue-agent/consumed-confirmations.json`；删除 issue 评论不能让旧确认再次生效。
8. 后台 watcher 由 cron / nohup 拉起时，先用 `.codex/issue-agent/env` 显式带上必要代理，并用 `just issue-agent-codex-smoke` 证明最小 `codex exec` 能跑通。
9. 当前暂停 Gitea PR；修复完成后回写 `wms-issue-agent:delivery:v1`：本地 worktree、分支、提交或 diff 状态、验证结果、截图附件、本地重启和版本校验证据、剩余风险、下一步。
10. `codex exec` 空闲超时、执行失败或收到外部终止信号只写状态更正，不视为用户验收反馈；不得自动重发修正方案，也不得复用旧确认继续执行。
11. 用户验收反馈“不对 / 不是这样 / 还缺 / 重新改 / 截图不对”等，视为新一轮反馈；必须评论 `wms-issue-agent:revision-proposal:v1` 修正方案，等待新的 `确认方案`，不得复用旧确认继续执行。
12. 暂停 PR 期间如果存在 open PR，先评论暂停原因，再关闭 PR；对应 head 分支进入主代理本地 review / 合并队列。
13. 主代理处理已关闭 issue 的本地分支时，先收口主工作区已有脏区；主工作区干净后再合入 closed issue 分支。禁止把 issue diff 直接揉进未提交脏区，除非只是为了解冲突且随后必须拆回独立提交。

## 执行约束

- 禁止用 tmux 承载 issue 修复任务。
- 禁止因为 issue 有历史确认就自动继续跑；确认必须晚于最新 proposal 或 revision-proposal。
- 禁止依赖 issue 评论是否还存在来判断旧确认是否可复用；以本地 consumed-confirmations 状态为准。
- 禁止把进程退出码、超时、`SIGTERM` / `SIGKILL` 当成业务修正需求；这类只写状态更正，等待人工补充新评论。
- 禁止只检查 watcher 进程存在；后台环境变更后必须跑 `just issue-agent-codex-smoke`。
- 禁止把 token、密钥、账号密码写入 `.codex/issue-agent/env`；该文件只用于代理变量。
- 禁止让 `codex exec` 承载长驻前端服务；exec 完成后会退出，人工测试统一用主工作区 `just dev-web-restart` 的 9002 tmux 会话。
- issue worktree 前端端口必须独立使用 9003-9099；后端端口只在改后端 / API / 数据库时独立使用 18081-18099，纯前端修复共用主后端 18080。
- worktree 预览或联调必须用 `just dev-web-worktree-restart/verify` 和 `just dev-api-worktree-restart/verify`，校验端口、LAN URL、`/healthz` 和进程 cwd 与 worktree 一致；不能只证明端口可访问。
- 禁止只修当前页面而不判断共性；proposal 判定共性时，必须查共享组件、字段矩阵、prompt / skill / runbook 和治理脚本。
- 禁止推送远端、创建 Gitea PR 或自动合并远端 PR；当前只允许本地分支交付，由主代理本地 review 后合并。
- 禁止把 open PR 留作备用合并入口；暂停 PR 期间关闭 open PR 后，必须登记本地分支、worktree、提交哈希或 diff 状态。
- 禁止把“已交付本地分支”写成“已完成”；没有测试、截图和回写证据就只能写阻塞或待验收。
- 禁止在主工作区存在无关脏区时合入 issue 分支；先用 `wms-review-fix-commit` 把现有脏区按主题提交，再合入 closed issue 分支并单独验证、单独提交。
- 新增字段、状态、角色、模块、业务默认值或跨模块语义变化时，停止并向用户确认。

## 最小检查

- `validate_skill.py`
- `resource_boundary_check.py`
- `trigger_eval.py` 使用 `evals/`
