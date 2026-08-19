---
name: wms-herdr-subtask
description: 使用 Herdr workspace 标签和 Tab 编号识别、启动、提示、等待及收口 WMS 交互式 Claude/Codex 子任务，并为写任务建立 worktree、注入 Rust 共享构建目录和执行磁盘门禁。用户说 `herdr wms 2`、开一个 Herdr Tab、在指定 Tab 启动 Claude/Codex、让某个 Tab 执行子任务、使用 Claude worktree，或询问 Herdr 中 Agent 状态时使用。
---

# WMS Herdr 标签子任务

使用 Herdr 管理终端和交互式 Agent，使用 Git worktree 隔离写任务。Herdr 地址只标识运行位置，不代表进程、端口、tmux 会话或 Agent 序号。

## 选择执行模式

| 请求 | 模式 | worktree |
|---|---|---|
| 识别 `herdr <workspace 标签> <tab 编号>`、查询状态 | `inspect` | 不创建 |
| 向已有 Tab 提问、继续已启动任务 | `control-existing` | 沿用该 Tab 当前目录 |
| 审查主区未提交差异 | `read-only-current-diff` | 不创建；禁止写入 |
| 启动交互式 Claude/Codex 写任务 | `write-worktree` | 一个任务一个独立 worktree |
| 非交互 `codex exec`、并行批处理或自动合并队列 | 路由到 `wms-worktree-subagent` | 按该技能执行 |

不得让两个写 Agent 使用同一 worktree。Herdr 已负责终端生命周期，不再启动 tmux；在 Herdr Tab 中使用 Claude worktree 时也不得追加 `claude --tmux`。

`write-worktree` 的实现、测试和自审必须由 Herdr 中的 subagent 执行；主代理只负责审查基线、定义边界、启动与监督、review、接入、主区复验和清理。禁止主代理借用本技能名义跳过 subagent 直接实现。

## 记录启动契约

启动前记录：

- workspace 标签、workspace ID、Tab 编号、Tab ID、pane ID。
- 模式、Agent 类型、模型、任务 slug。
- 可见 Git 快照、授权读写路径、依赖和验证命令。
- worktree 路径、分支、Rust 构建目录。
- 完成条件、停止条件和输出要求；成功任务默认进入接入与清理，不停在“可接入”。

用户只提供地址时执行 `inspect`；识别和读状态不授权发消息、启动、停止、关闭或删除。只有用户明确要求对应控制动作时才执行。

## 实时解析 Herdr 地址

1. 验证 `HERDR_ENV=1`；不在 Herdr 管理环境中时停止实时控制，不猜测映射。
2. 运行 `herdr workspace list`，按标签取得 workspace ID。
3. 运行 `herdr tab list --workspace <workspace-id>`，按 number 取得 Tab ID。
4. 运行 `herdr agent list`、`herdr pane list --workspace <workspace-id>`，必要时运行 `herdr api snapshot`。
5. 返回实时 pane、Agent、状态和目录；不得复用上次会话结论。

标签不存在或同名时停止控制并报告候选项。Tab 编号不存在时不得自动改用其他 Tab。

## 启动前门禁

在主工作区运行：

```bash
git status --short --branch
git worktree list
df -h . /tmp
```

逐项运行 `git status --short -- <授权写入路径>`。目标路径已修改、已暂存或未跟踪时，不得从旧 `HEAD` 创建写 worktree；先由主代理审查并形成明确基线，或切换到 `read-only-current-diff`。

可用空间低于 5GiB 时不得启动写任务或重型 Rust 验证；低于 2GiB 时停止所有重型验证。不得为绕过磁盘门禁改用另一个私有 `target`。

## 建立写任务

默认采用 Agent 无关的 Git worktree：

```bash
git worktree add -b agent/<slug> ../wms-agent-<slug> HEAD
```

然后在目标 workspace 中创建 Tab，并注入共享 Rust 构建配置：

```bash
herdr tab create \
  --workspace <workspace-id> \
  --cwd ../wms-agent-<slug> \
  --label <slug> \
  --env CARGO_TARGET_DIR=/home/test1/workspace/wms/backend/target \
  --env CARGO_INCREMENTAL=0 \
  --no-focus
```

从命令结果读取真实 Tab ID 和 pane ID，禁止根据创建顺序猜编号。

本机 Herdr 启动 Claude 必须使用交互式 shell 中的 `claudep` alias；该 alias 展开为 `claude --dangerously-skip-permissions`，用于避免子任务对每条 Bash 命令重复等待人工确认。禁止用 `herdr agent start --kind claude`、裸 `claude` 或 `--permission-mode acceptEdits` 代替。

仅当用户明确要求“Claude 原生 worktree”时，不预建 Git worktree；让 Tab 从主工作区 shell 启动：

```bash
herdr pane send-text <pane-id> 'claudep -w <slug>'
herdr pane send-keys <pane-id> enter
```

两种 worktree 方式只能选择一种，禁止在已有任务 worktree 中再次运行 `claude -w`。

默认 worktree 中启动 Claude：

```bash
herdr pane send-text <pane-id> claudep
herdr pane send-keys <pane-id> enter
```

等待 Herdr 实时识别该 pane 中的 Claude 并确认状态为 `idle` 后再提交提示词。若 `claudep` 不存在，停止启动并检查交互式 shell alias；不得降级为会逐次审批的 Claude 启动方式。

启动 Codex 时仍使用 Herdr 原生入口：

```bash
herdr agent start <slug> --kind codex --pane <pane-id>
```

用户指定模型或参数时放在 `--` 之后；启动前用对应 CLI `--help` 验证参数，不得静默降级。

## 提交任务提示词

提示词必须包含：目标、模式、可见基线、授权读写路径、禁止路径、数字预算、验证命令、是否允许提交、最终输出契约。写任务默认不提交、不推送、不合并主分支，由主代理接入。

```bash
herdr agent prompt <agent-id> '<任务提示词>' \
  --wait --until working --timeout 30000
```

启动后重新运行 `herdr agent get <agent-id>` 和 `herdr pane process-info <pane-id>`，确认 Agent 与 cwd 对应本任务。提示词要求子 Agent 按 `wms-loop-engineering` 小步执行，并按 `wms-review-fix-commit` 的 review → fix → review 检查项自审。

## Rust worktree 构建规则

- 所有 Rust worktree 复用主工作区 `/home/test1/workspace/wms/backend/target`；不得在每个 worktree 生成独立 `backend/target`。
- 一次性审查/验证任务使用 `CARGO_INCREMENTAL=0`，避免累积增量对象；只有明确的长期开发 Tab 才可保留增量编译。
- 先运行最小 `cargo check -p <package>`，再运行相关测试；只有验收确需二进制时才执行完整 `cargo build`。
- 共享目录可能让并行 Cargo 等待构建锁；等待或错峰执行，不得临时切回私有 `target`。
- 子 Agent 禁止运行 `cargo clean`、删除共享 `target` 或安装 `sccache`。共享缓存清理由主代理先执行 `cargo clean --target-dir <path> --dry-run` 评估，并取得明确清理授权后执行。
- 构建前后运行 `df -h . /tmp`；`target`、增量对象和编译日志不得进入 Git diff。

## 监控与反馈

- 用 `herdr agent get/list` 查询实时状态，用 `herdr agent wait <agent-id> --timeout <ms>` 等待状态变化。
- 只在诊断需要时读取有限终端输出：`herdr pane read <pane-id> --source recent --lines <N>`。
- Agent 阻塞时先报告具体审批、凭据、路径、磁盘或测试错误；不得盲目重复提示。
- 长任务至少每 60 秒向用户提供一次简短进展，并核对 Agent、cwd、磁盘和 diff 范围。
- Agent 越权写文件、共享路径出现第二个 writer、磁盘低于门禁或任务范围失控时立即停止该任务并标记不可接入。

## 审查与收口

Agent 完成后：

1. 查询 Agent 最终状态并读取最终结论。
2. 在任务 worktree 运行 `git status --short`、`git diff --check` 和范围内测试。
3. 主代理审查全部 diff；子 Agent 的通过结论不能替代主代理复验。发现问题时把修复反馈给同一 subagent，不得由主代理静默重写实现。
4. 子任务有提交时按审查后的提交合并；无提交时把授权 diff 接入主工作区。接入冲突由主代理处理，但不得夹带其他主题。
5. 在主工作区重新运行相关测试、`git diff --check`、`just gov-t1`。
6. 逐个核对 worktree 变更路径都在授权范围，并确认其最终源码与主区接入结果一致；未跟踪项只能是已识别的依赖 symlink、构建缓存或临时产物。
7. 让 Agent 退出，确认没有进程 cwd 位于任务 worktree，再关闭对应 Herdr Tab。
8. 主区复验通过后删除任务 worktree；因为 subagent 默认不提交，允许在第 6 步核对完成后用 `git worktree remove --force <path>` 删除已接入的脏 worktree。不得删除分支，除非用户另行明确授权。
9. 报告 Tab、Agent、已删除 worktree、保留分支、共享 Cargo target、验证退出码和剩余问题。

调用本技能执行写任务即授权在“改动已接入且主区复验通过”后关闭该任务 Tab 并删除该 worktree，不再额外询问。任务失败、diff 未接入、路径越权或主区验证失败时不得删除，必须保留现场并报告阻断；禁止删除未审查 diff。
