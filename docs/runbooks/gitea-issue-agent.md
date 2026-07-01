# Gitea Issue Agent 运行手册

> 目标：定时读取 Gitea issue，先判断并评论，等待确认后在 tmux 中启动 Codex 任务。默认使用 `tmux + codex exec`，避免依赖交互式 TUI 的输入状态。

## 流程

1. `scripts/agents/issue_runner.py once` 读取 open issue 和评论。
2. 没有 agent 判断评论时，生成判断评论：
   - 复述 issue。
   - 摘要已有人工评论。
   - 给出结论、置信度、判断依据、预计影响范围、建议动作、验证要求和停止条件。
   - 明确本轮不改代码、不提交、不推送。
   - 只有判断为可执行或用户接受风险后，才提示用 `/confirm` 确认。
3. issue 出现 `/confirm` 评论，或带 `codex:confirmed` 标签后，脚本生成修复 prompt。
4. `--apply` 模式下，脚本默认新建 `wms-issue-<issue>-<时间>` tmux 会话，并在其中运行 `codex exec`。
5. Codex 任务会话按 WMS `AGENTS.md` 和 skills 修复、验证、重启本地测试前后端、采集截图、分支推送并创建 PR。
6. Codex 最后把 PR 链接、提交哈希、验证结果、截图证据、本地测试环境重启结果和剩余风险评论回 PR 与 issue。

## 前置条件

- 已安装 `tea`、`tmux`、`codex`。
- `tea` 已登录当前 Gitea，并能在仓库目录下调用：

```bash
tea api '/repos/{owner}/{repo}/issues?state=open&limit=1'
```

- 有可用 tmux 服务：

```bash
tmux ls
```

默认投递模式是 `--tmux-mode exec`，会为每个确认后的 issue 新开任务会话。仍可用下面两种调试备用模式：

- `--tmux-mode session`：新开交互式 Codex TUI 会话。
- `--tmux-mode paste`：把 prompt 粘贴到固定 pane。

```bash
tmux new -s wms-codex
export WMS_ISSUE_AGENT_TMUX_TARGET='wms-codex:0.0'
just issue-agent-once --issue 1 --apply --tmux-mode paste
```

## 命令

默认 dry-run，只生成本地预览，不评论 issue、不发送到 tmux：

```bash
just issue-agent-once
just issue-agent-once --issue 1
```

确认预览后执行写操作：

```bash
just issue-agent-once --apply
just issue-agent-once --issue 1 --apply
```

如果之前已经产生过 tmux 标记，但需要重新发送一次确认后的 issue：

```bash
just issue-agent-once --issue 1 --apply --force-dispatch
```

循环运行：

```bash
just issue-agent-watch --interval 60 --apply
```

在 tmux 中长期运行：

```bash
tmux new -d -s wms-issue-agent 'cd /home/test1/workspace/wms && just issue-agent-watch --interval 60 --apply 2>&1 | tee -a .codex/issue-agent/watch.log'
tmux attach -t wms-issue-agent
```

本地自检：

```bash
python3 scripts/agents/issue_runner.py self-test
```

## 安全规则

- 默认 dry-run。
- 一轮只处理一个 issue。
- 未确认前只评论判断，不改代码。
- 默认新开独立 tmux 任务会话并运行 `codex exec`，避免依赖交互式 TUI 粘贴状态。
- 使用 `--tmux-mode paste` 时，发送前必须能找到目标 pane。
- prompt 要求 Codex 禁止推 main、禁止强推。
- 前端或用户可见行为修复必须重启本地测试前后端，并把端口、URL、`/healthz` 或等价健康检查结果写回 PR 与 issue。
- 重启后必须校验运行的是本次修复版本：记录提交哈希，并用页面可见变更、接口响应版本字段或等价证据证明不是旧进程缓存。
- 前端或用户可见行为修复必须采集真实前端截图，并把截图路径、视口、页面 URL、结论同时写回 PR 与 issue。
- 新增字段、状态、角色、模块或业务默认值时，Codex 必须停止并询问用户。
- `session` 和 `paste` 只用于调试；无人值守定时任务必须使用默认 `exec` 模式。

## Loop 迭代检查

每轮运行后检查：

| 检查 | 通过标准 |
|---|---|
| issue 读取 | 能读取标题、正文、评论 |
| 判断评论 | 内容中文，包含结论、置信度、依据、预计影响范围、建议动作、验证要求和停止条件；不能只要求 `/confirm` |
| 确认识别 | `/confirm` 或 `codex:confirmed` 才触发 tmux |
| tmux 投递 | 默认新建 issue 任务会话并运行 `codex exec`；paste 模式只发送到固定 pane，目标不存在则失败停止 |
| WMS 执行 | Codex 使用 WMS skills，完成验证、截图、前后端重启后创建 PR |
| 证据回写 | PR 与 issue 都包含截图证据、本地测试环境重启结果、重启后版本校验和剩余风险 |

失败时只修失败点，不扩大流程。
