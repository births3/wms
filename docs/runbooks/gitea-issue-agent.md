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
6. Codex 把 PR 链接、提交哈希、验证结果、截图附件、本地测试环境重启结果、tmux 任务会话状态、PR 合并前置条件和剩余风险评论回 PR 与 issue。
7. 主代理把 PR 纳入收口队列：复审、合并或等待用户确认合并；合并后清理 worktree、agent 分支和已结束的 tmux 任务会话。
8. 启用 `--merge-closed` 后，watcher 还会扫描已关闭 issue；只有关联 PR 满足闭环证据、可合并、无阻塞评论且分支为 `agent/*` 时，才会自动合并并写回 marker。

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

循环运行并在 issue 关闭后自动合并满足条件的 PR：

```bash
just issue-agent-watch --interval 60 --apply --merge-closed
```

单独 dry-run 检查 closed issue 合并候选：

```bash
python3 scripts/agents/issue_runner.py merge-closed
python3 scripts/agents/issue_runner.py merge-closed --issue 3
```

在 tmux 中长期运行：

```bash
just issue-agent-restart
just issue-agent-verify
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
- 长期 watcher 必须用 `just issue-agent-restart` 启动，并用 `just issue-agent-verify` 确认；只看到 `wms-issue-agent` tmux 会话存在不算健康，必须有 `scripts/agents/issue_runner.py watch` 进程。
- prompt 要求 Codex 禁止推 main、禁止强推。
- prompt 要求 Codex 禁止自行合并 PR；PR 创建后必须写清合并前置条件和 tmux 任务会话状态。
- 自动合并只由 issue watcher 在 `--merge-closed --apply` 下执行，合并策略固定为 squash；子代理仍禁止自行合并 PR。
- 自动合并前必须满足：issue 已关闭、PR open、PR `mergeable=true`、PR 未合并、PR head 分支为 `agent/*`、已有验证 / 截图 / 重启证据、没有 `/reject` / `不要合并` / `阻塞` / `blocked` 评论。
- issue 评论包含截图 / 附件时，prompt 必须列出附件下载 URL；Codex 必须先打开或下载附件辅助定位。
- 前端或用户可见行为修复必须重启本地测试前后端，并把端口、URL、`/healthz` 或等价健康检查结果写回 PR 与 issue。
- 9002 只允许主工作区固定会话 `wms-web-admin-9002` 占用；子 worktree 不得启动或占用 9002。
- 重启后必须校验运行的是本次修复版本：主代理在主工作区运行 `just dev-web-restart` 和 `just dev-web-verify`，记录提交哈希，并用校验输出、页面可见变更、接口响应版本字段或等价证据证明不是旧进程缓存。
- 前端或用户可见行为修复必须采集真实前端截图，用 `POST /repos/{owner}/{repo}/issues/<编号>/assets` 上传为 Gitea 附件，并用 Markdown 图片同时评论到 PR 与 issue；不能只写本地路径。
- 新增字段、状态、角色、模块或业务默认值时，Codex 必须停止并询问用户。
- `session` 和 `paste` 只用于调试；无人值守定时任务必须使用默认 `exec` 模式。

## Loop 迭代检查

每轮运行后检查：

| 检查 | 通过标准 |
|---|---|
| issue 读取 | 能读取标题、正文、评论 |
| 判断评论 | 内容中文，包含结论、置信度、代码核查证据、依据、预计影响范围、建议动作、验证要求和停止条件；不能只要求 `/confirm`，不能只复述 issue |
| 输入附件 | issue 评论中的截图 / 附件必须以可下载 URL 写入判断评论和执行 prompt |
| 确认识别 | `/confirm` 或 `codex:confirmed` 才触发 tmux |
| tmux 投递 | 默认新建 issue 任务会话并运行 `codex exec`；paste 模式只发送到固定 pane，目标不存在则失败停止 |
| WMS 执行 | Codex 使用 WMS skills，完成验证；前端截图和 9002 重启证据由主代理在主工作区校验后补齐，且不自行合并 PR |
| 证据回写 | PR 与 issue 都包含真实截图附件、本地测试环境重启结果、重启后版本校验、tmux 任务会话状态、PR 合并前置条件和剩余风险 |
| PR 收口 | issue 关闭后，watcher dry-run 能发现关联 PR；`--apply --merge-closed` 下满足条件才自动合并并写回 marker，不能停在“已创建 PR” |
| tmux 清理 | 任务会话完成后应自然退出；未退出时记录会话名、原因和下一步，用户确认清理后再 `tmux kill-session -t <session>` |

失败时只修失败点，不扩大流程。
