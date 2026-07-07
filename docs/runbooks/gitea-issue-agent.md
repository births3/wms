# Gitea Issue Agent 运行手册

> 目标：定时读取 Gitea issue，先发方案提案，等待用户回复裸一行 `确认方案` 后，直接用 `codex exec` 执行。执行细则见 `.agents/skills/wms-issue-codex-exec/SKILL.md`；禁止再用 tmux 承载 issue 修复任务。

## 流程

1. `scripts/agents/issue_runner.py once` 读取 open issue 和评论。
2. 没有最新 agent 方案提案时，生成 `wms-issue-agent:proposal:v1`：
   - 复述 issue。
   - 摘要已有人工评论。
   - 列出根因文件、关键行号、改动计划、验证计划、风险和停止条件。
   - 列出相似 / 共性问题判断：是否影响同类页面、组件、字段、流程或治理脚本；是否需要一起修改；预防规则落到 prompt、skill、runbook、规范还是 T1 治理脚本。
   - 明确本轮不改代码、不提交、不推送。
   - 只有判断为可执行或用户接受风险后，才提示回复裸一行 `确认方案`。
3. 最新 proposal 之后出现人工评论且正文整行等于 `确认方案`，才生成修复 prompt；`/confirm`、`开始处理`、历史确认、否定句和 agent 评论都不能触发。
4. `--apply` 模式下，脚本创建独立 worktree 和分支，直接运行 `codex exec -C <worktree> - < <prompt>`；禁止 tmux 和交互式 Codex TUI。
5. 执行前把本次确认评论写入 `.codex/issue-agent/consumed-confirmations.json`；即使 Gitea 上 exec / status 评论被删除，旧确认也不能再次触发。
6. `codex exec` 不按固定总时长判定卡住；默认用 `WMS_ISSUE_AGENT_CODEX_IDLE_SECONDS=900` 检查日志、HEAD 和 worktree 状态是否持续无进展，可选 `WMS_ISSUE_AGENT_CODEX_MAX_SECONDS` 作为人工配置的硬上限。
7. `codex exec` 空闲超时、失败或收到 `SIGTERM` / `SIGKILL` 等外部终止信号时，只评论失败状态，不自动重发方案，不复用旧确认；需要重跑时由人工补充新评论，再进入下一轮 proposal。
8. 当前暂停 Gitea PR：Codex exec 只在本地 worktree / 本地分支修复、验证和回写 issue；禁止推送远端、禁止创建 Gitea PR。
9. Codex 把本地 worktree、分支、提交或 diff 状态、验证结果、截图附件、本地测试环境重启结果、版本校验证据、本地合并前置条件和剩余风险评论回 issue。
10. 主代理把 issue-agent 本地分支纳入本地收口队列：先收口主工作区已有脏区，再复审 issue 分支、补证据、修冲突、合并到主工作区、再决定是否关闭 issue。
11. 已回写本地交付、等待主代理本地合并、阻塞或状态更正属于状态评论，不是新需求评论；watcher 不能因此再次生成“请回复确认方案”的判断评论。
12. 用户验收反馈不对时，必须生成 `wms-issue-agent:revision-proposal:v1` 修正方案，等待新的 `确认方案` 后再执行；禁止复用旧确认自动继续跑。

## 前置条件

- 已安装 `tea`、`codex`。
- `tea` 已登录当前 Gitea，并能在仓库目录下调用：

```bash
tea api '/repos/{owner}/{repo}/issues?state=open&limit=1'
```

## 命令

默认 dry-run，只生成本地预览，不评论 issue、不执行 codex：

```bash
just issue-agent-once
just issue-agent-once --issue 1
```

确认预览后执行写操作：

```bash
just issue-agent-once --apply
just issue-agent-once --issue 1 --apply
```

循环运行：

```bash
just issue-agent-watch --interval 60 --apply
```

当前暂停 Gitea PR，不启用 `--merge-closed`；历史 `merge-closed` 子命令仅保留为旧流程排查工具，默认 watcher 不调用。暂停期间发现 open PR 时，先评论“暂停 PR、转本地合并”的原因，再关闭 PR；对应分支进入主代理本地 review / 合并队列。

长期运行 watcher（pid 文件后台进程 + cron watchdog，不使用 tmux）：

```bash
just issue-agent-install-watchdog
just issue-agent-restart
just issue-agent-verify
```

本地自检：

```bash
python3 scripts/agents/issue_runner.py self-test
just issue-agent-codex-smoke
```

后台环境自检：

- 长期 watcher 由 cron 拉起时不会自动继承交互 shell 的代理变量。
- 如果本机访问 Codex 需要代理，把非密钥代理变量写入 `.codex/issue-agent/env`，例如：

```bash
http_proxy=http://127.0.0.1:7894
https_proxy=http://127.0.0.1:7894
all_proxy=socks5://127.0.0.1:7894
no_proxy=localhost,127.0.0.1,::1,192.168.0.0/16
```

- `.codex/issue-agent/env` 只允许放 `http_proxy` / `https_proxy` / `all_proxy` / `no_proxy` 及大写同名变量；禁止放 token、密钥、账号密码。
- 修改后台环境后先运行 `just issue-agent-codex-smoke`，再运行 `just issue-agent-restart` 和 `just issue-agent-status`。

## 安全规则

- 默认 dry-run。
- 一轮只处理一个 issue。
- 未确认前只评论判断，不改代码。
- 默认创建 `../wms-agent-issue-<编号>-<时间>` worktree，再在该 worktree 中直接运行 `codex exec`；禁止在主工作区直接执行 issue 修复。
- 禁止使用 tmux、`--tmux-mode session`、`--tmux-mode paste` 或交互式 Codex TUI 承载 issue 修复任务。
- 长期 watcher 必须用 `just issue-agent-install-watchdog` 安装 cron watchdog；watchdog 每分钟执行 `just issue-agent-ensure`，进程不存在时自动 `issue-agent-restart`。
- `just issue-agent-verify` 必须确认 pid 文件对应 `scripts/agents/issue_runner.py watch` 进程。
- `just issue-agent-codex-smoke` 必须能跑通最小 `codex exec`；只看 watcher 进程存在不代表 Codex 连接可用。
- prompt 要求 Codex 禁止推送远端、禁止创建 Gitea PR、禁止推 main、禁止强推。
- prompt 要求 Codex 禁止自行合并到主工作区；本地分支交付后必须写清本地合并前置条件和 codex exec 日志位置。
- prompt 要求 Codex 先执行 proposal 中的相似 / 共性问题判断；共性成立时必须一起修共享组件、字段矩阵、规范或治理脚本，不能只修当前页面。
- issue-agent 本地分支只有一个合并 owner：主代理。watcher 只启动执行和回写状态，不做本地合并。
- 主代理合并 closed issue 分支前，必须先处理主工作区脏区：已有脏区按主题 review、验证、提交；主工作区干净后再合入 issue 分支。禁止把 issue diff 混入已有脏区提交。
- 本地分支交付不等于 issue 完成。issue 评论只能写“已交付本地分支 / 等待主代理本地 review 合并 / 阻塞”，禁止在主工作区未验证和未收口前写“已完成”。
- 自动 PR 合并暂停：长期 watcher 不带 `--merge-closed`，不得因为 issue 关闭就合并远端 PR。
- 暂停 PR 期间不得保留 open PR 作为合并入口；关闭前必须写明原因，关闭后把 head 分支、worktree、提交哈希或 diff 状态登记为本地待合并对象。
- issue 评论包含截图 / 附件时，prompt 必须列出附件下载 URL；Codex 必须先打开或下载附件辅助定位。
- 前端或用户可见行为修复必须重启本地测试前后端，并把端口、URL、`/healthz` 或等价健康检查结果写回 issue。
- 9002 只允许主工作区固定会话 `wms-web-admin-9002` 占用；子 worktree 不得启动或占用 9002。
- `codex exec` 是短生命周期任务，执行完会退出；可人工测试的前端必须由主工作区 `just dev-web-restart` 启动的 `wms-web-admin-9002` 长驻会话提供，不能依赖 exec 进程保活。
- issue 分支 / worktree 临时预览使用 `just dev-web-worktree-restart <worktree> <端口>`，端口范围 9003-9099；默认 9003，必须输出 LAN URL，并校验端口对应进程 cwd 来自该 worktree。
- issue 分支 / worktree 后端联调只在改后端 / API / 数据库时启动，使用 `just dev-api-worktree-restart <worktree> <端口>`，端口范围 18081-18099；纯前端修复共用主后端 18080。后端校验必须包含 `/healthz`、LAN URL 和进程 cwd 一致性。
- 重启后必须校验运行的是本次修复版本：主代理在主工作区运行 `just dev-web-restart` 和 `just dev-web-verify`，记录提交哈希，并用校验输出、页面可见变更、接口响应版本字段或等价证据证明不是旧进程缓存。
- 前端或用户可见行为修复必须采集真实前端截图，用 `POST /repos/{owner}/{repo}/issues/<编号>/assets` 上传为 Gitea 附件，并用 Markdown 图片评论到 issue；不能只写本地路径。
- 新增字段、状态、角色、模块或业务默认值时，Codex 必须停止并询问用户。
- 用户验收反馈不对时，必须重新评论修正方案并等待新的 `确认方案`；旧确认只对旧方案有效。

## Loop 迭代检查

每轮运行后检查：

| 检查 | 通过标准 |
|---|---|
| issue 读取 | 能读取标题、正文、评论 |
| 判断评论 | 内容中文，包含结论、置信度、代码核查证据、依据、预计影响范围、建议动作、验证要求和停止条件；不能只要求确认，不能只复述 issue |
| 真实分析 | proposal 必须把模板字段填成当前 issue 的具体判断；遇到勾选、全选、选中、取消勾选、第一个等选择状态问题时，必须追踪 `selectedRowKeys`、`onSelectedRowKeysChange`、页面自动首选逻辑和共享 DataGrid，不能只按“按钮 / 菜单”泛化 |
| 共性判断 | proposal 必须说明是否存在相似 / 共性问题、是否一起修改、规则落到 prompt / skill / runbook / 规范 / 治理脚本中的哪一类 |
| 输入附件 | issue 本体和 issue 评论中的截图 / 附件必须以可下载 URL 写入判断评论和执行 prompt |
| 确认识别 | 最新 proposal 或 revision-proposal 之后，人工评论裸一行 `确认方案` 才触发执行；否定句和旧确认不能触发 |
| 确认消费 | 已消费确认必须落到 `.codex/issue-agent/consumed-confirmations.json`；删除 issue 评论不能让旧确认再次生效 |
| codex exec 执行 | 创建独立 worktree 和分支，直接在该 worktree 运行 `codex exec`；禁止 tmux、paste 模式和交互式 TUI |
| codex exec 卡住 | 依据日志、HEAD 和 worktree 状态的空闲时间判断；超时、失败或外部终止只评论状态，不自动重发方案，旧确认不能复用 |
| codex exec 连接 | cron / nohup 后台环境必须通过 `.codex/issue-agent/env` 显式带上必要代理；`just issue-agent-codex-smoke` 必须通过 |
| watcher 保活 | `just issue-agent-status` 必须显示 cron watchdog；杀掉 watcher 后，下一分钟应由 `issue-agent-ensure` 自动拉起 |
| worktree 端口 | 前端 9003-9099、后端 18081-18099；验证必须证明端口对应进程来自 issue worktree，不能只看 URL 可访问 |
| WMS 执行 | Codex 使用 WMS skills，完成验证；前端截图和 9002 重启证据由主代理在主工作区校验后补齐，且不自行合并主工作区 |
| 证据回写 | issue 包含本地 worktree / 分支 / 提交或 diff 状态、真实截图附件、本地测试环境重启结果、重启后版本校验、codex exec 日志位置、本地合并前置条件和剩余风险 |
| 状态评论 | 已交付本地分支、等待主代理合并、阻塞、状态更正等评论不能触发新一轮判断；只有后续人工补充真实新需求时才重新判断 |
| 合并 owner | issue-agent 本地分支只由主代理 review 后本地合并；watcher 不合并 |
| 合并顺序 | 主工作区脏区先按主题提交，closed issue 分支后合入；两者不得混成一个提交 |
| 验收反馈 | 用户说不对、还缺或截图不符时，必须重新发修正方案并等待新确认，不得复用旧确认自动继续跑 |

失败时只修失败点，不扩大流程。
