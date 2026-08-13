---
name: wms-worktree-subagent
description: 用户要求在 WMS 仓库使用 subagent、codex exec、worktree、并行拆分任务、只读审查当前未提交差异、指定子代理模型、由主代理 review/merge，或清理遗留 agent worktree 时使用。
---

# WMS Worktree Subagent

用于把 WMS 缺口安全拆给子代理；主代理负责模式选择、并行调度、审查、接入和收口。

## 启动前选择模式

| 模式 | 使用场景 | 隔离要求 |
|---|---|---|
| `write-worktree` | 子代理需要修改文件 | 一个任务一个独立 worktree；写入范围不得与其他写任务重叠 |
| `read-only-worktree` | 审查已提交的 `HEAD` | 使用可丢弃 worktree；不修改、不合并 |
| `read-only-current-diff` | 审查主工作区未提交或未跟踪文件 | 直接在主工作区以 `read-only` 运行；不建分支、不建 worktree、不合并 |

启动前必须记录模式、模型、可见快照、读写范围、依赖和输出文件。`read-only-current-diff` 运行期间主代理不得修改工作区；全部子代理退出后重新核对 `git status --short`。

`write-worktree` 启动前必须逐个检查授权写入路径在主工作区的状态：已修改、已暂存或未跟踪的目标路径都视为“脏基线”。脏基线不能直接从旧 `HEAD` 新建实现 worktree；必须先由主代理审查并提交/建立明确的 reviewed baseline，或改为 `read-only-current-diff` 只读审查。禁止让子代理基于旧 `HEAD` 重建与主区未提交 schema/接口并行的第二套实现。

用户指定模型时使用 `-m <model>`；未指定时沿用本机 Codex 默认值。启动前用 `codex exec --help` 确认参数，模型不可用时停止并报告，禁止静默降级。

## 并行规则

- 默认最多 3 个子代理并行；任务存在依赖时按顺序运行。
- 写任务必须使用互不重叠的授权路径；只读任务可以读取同一文件，但审查目标必须不同。
- 每个子代理使用唯一 slug 和输出文件；主代理记录任务、模式、模型、范围、依赖、进程状态和退出码。
- 启动延期故事任务前必须从当前 `governance/quality-matrix.toml` 读取目标 ID 的实际分区：只有位于 `deferred_stories` 的故事才能按“延期故事推进”启动；已经位于 `stories` 且状态为 `verified` 的故事不得重复启动。若任务只是补充已验证故事的证据，提示词必须明确写成“证据补强”，不能计入延期故事完成数。
- 新 worktree 只包含 `HEAD` 的技能版本；若主区 `.agents/skills/wms-worktree-subagent/SKILL.md` 或任务模板有未提交差异，启动前必须比较并把本轮硬门禁摘要写入提示词，或先提交技能基线；不能假定子代理能看到主区未提交规则。
- 主代理等待本批全部进程退出后再修改当前工作区、汇总结论或启动下一批。
- 原生 subagent 的 `close_agent` / `resume_agent` 不等价于底层 `codex exec` 进程收口。恢复同一 agent、复用同一 worktree 或同步主区快照前，必须按真实 worktree 路径检查 `/proc/<pid>/cwd`，确认没有属于该 worktree 的 `codex exec`、timeout、sandbox 或编译子进程；发现残留时先按 PGID 终止并复查，禁止直接恢复或同步。

## 硬停止门禁

- 启动前和每轮验证前运行 `df -h . /tmp`；可用空间低于 5GiB 不得启动写代理，低于 2GiB 立即停止重型验证。
- 主代理每 60 秒轮询一次；单个切片默认最长运行 20 分钟。超过时限、连续两轮没有新验证证据，或 `git diff --stat` 超出任务预算，立即终止代理并标记“不可合并”。
- 写任务必须在启动提示词中写出数字预算；未写时默认最多 8 个文件、净增删 350 行、3 个可测试行为。一个切片同时改变共享 Domain DTO、两个业务模块、OpenAPI、PC 页面和 E2E 时，必须拆成多个有依赖顺序的切片。
- 任务涉及页面时，目标页面或新增页面预计达到 600 行必须先拆出页面私有组件；达到 800 行禁止继续，不能用豁免注释掩盖切片过大。
- 任务涉及 PostgreSQL、pnpm、OpenAPI 生成器或真实浏览器时，先做依赖预检再改码：数据库命令必须通过 `set -a; source /home/test1/workspace/wms/.env; set +a` 注入环境但不得打印连接串；前端优先复用已有 `node_modules`，不得先运行会触发安装的脚本。
- 依赖预检失败最多明确修复/重试一次；不得创建 `/tmp` shim、包装命令、手写生成物或继续扩大代码差异。第二次失败立即标记“不可合并”。
- 子代理退出时必须存在指定输出文件且包含最终输出契约；无报告、被终止、退出码非 0、磁盘/依赖/凭据阻断，均不可合并，worktree 保留到分流队列。
- 读取范围也是硬门禁：除“必须先读”文档外，只能读取任务授权路径和覆盖矩阵列出的文件；禁止对仓库根目录、`backend/`、`apps/` 或 `tests/` 使用无范围的 `rg --files`、`find`、`rg`/`grep`。任务文件最多读取 12 个，单条命令输出超过 400 行必须立即停止并改用局部 `sed`/`rg`；再次超出或违反授权范围时直接写“不可合并”，不得继续实现。
- 授权路径预检是必做项：开始分析前逐个执行 `test -f` / `test -d` 检查提示词列出的路径；路径不存在时必须用限定范围的 `rg -n` 检查同模块的 `*_part*.rs`、子目录、`bin/<binary>/`、`tests/<module>/` 和 router/repository 注册点。不能把“未授权读取”或“路径写错”推断成“实现不存在”；若无法确定规范路径，立即报告“审查阻断”并要求主代理修正范围。
- 过程输出也是硬门禁：启动命令必须丢弃 JSON stdout，只把 stderr 重定向到唯一 `/tmp/wms-agent-<slug>.log`；`-o` 指定的最终报告是唯一事实输出。使用下方“启动封装”实时检查 stderr 行数和字节数；stderr 超过 600 行或 200 KiB、最终报告未生成、或进程超过 20 分钟，立即终止整个进程组并标记“不可合并”。代理提示词必须要求单次命令最多输出 120 行，禁止粘贴完整源文件、完整 diff 或递归扫描结果；JSON 事件流不得落盘或读取，主代理只读取最终报告和退出码。
- 子代理不得把 `governance/quality-matrix.toml` / `docs/governance/quality-matrix.md` 中延期故事提前改为 `verified`，也不得用“计划覆盖”填证据；只有主代理在真实验证、截图和 review 证据齐全后才能迁移矩阵状态。
- 发现 `No space left on device` 时先停止所有写代理，再只清理 `backend/target`、pnpm store、Playwright/Node 临时缓存等可重建产物；禁止删除源码、未合并 diff、主工作区 `target` 或用 `git clean` 掩盖污染。
- 同批代理中任一任务超预算或无报告，不得启动下一批；先完成 closeout、记录根因，再重新拆分更小切片。

## 子代理原则

- 写任务一个任务一个 worktree；只有 `read-only-current-diff` 可以读取主工作区。
- Rust 编译共享：所有 worktree（含 EnterWorktree 原生 worktree）复用主工作区 `backend/target`，完整规则见仓库根 `CLAUDE.md`「Rust 编译资源共享」（唯一事实源）。要点：`justfile` 统一注入 `CARGO_TARGET_DIR`（不自动回退）、经 just 编译统一关闭增量、禁止各 worktree 独立 target / `cargo clean` / 删共享 target / 装 sccache、并行等锁错峰、构建前后 `df -h . /tmp` 检查（< 5GiB 不启动重型验证，< 2GiB 停止）。
- 子代理只改授权范围；不推送、不改 main。
- 子代理不创建远端 PR；只交付本地 diff、验证结果和清理建议。Gitea issue-agent 当前也走本地分支交付。
- 写任务默认用 `wms-loop-engineering`，并按 `wms-review-fix-commit` 的 review → fix → review 检查项自审；子代理不 `git add` / `git commit`，由主代理提交。
- `read-only-worktree` 输出下一轮切片、允许文件、停止条件、验证命令和技能缺口；`read-only-current-diff` 输出按严重度排序的发现、证据、最小修复和验证命令。
- 外部设备、TMS、冷链平台、生产数据和凭据类 evidence 不能交给子代理伪造。
- 构建缓存和大产物不得合并；污染必须在最终输出标记。
- `pnpm`、OpenAPI 生成器、数据库、`.env`、网络或外部服务验证失败时最多重试一次；仍失败则不可合并。
- 最终输出只写结论、摘要、验证和清理建议；完整 diff 由主代理审查。
- 前端 worktree 预览端口只能用 9003-9099，9002 保留给主工作区固定会话；用 `just dev-web-worktree-restart <worktree> <port>` 启动并用 LAN URL 验证。
- 后端 worktree 联调端口只能用 18081-18099，18080 保留给主工作区固定后端；纯前端任务共用 18080，改后端 / API / 数据库时才用 `just dev-api-worktree-restart <worktree> <port>` 启动独立后端。
- worktree 服务验证必须证明端口对应进程 cwd 来自该 worktree；不能只证明 URL 可访问。

## 建立子代理

1. 主工作区先运行：
   - `git status --short --branch`
   - `git worktree list`
2. 任务命名用短 slug，例如 `m2-inbound-pc`。
3. 新 worktree 看不到未提交文件；需要审查当前脏区时改用 `read-only-current-diff`，需要基于脏区写入时先由主代理审查并提交相关基线。
   - 启动前用 `git status --short -- <授权路径>` 逐项核对；任一授权路径非干净，登记“基线阻断”，不得用“当前 HEAD 已有基础”替代主区实际代码。目标是未跟踪文件时同样阻断。
4. 先写清任务边界；没有边界，不建 worktree：
   - 按 [references/module-slice-boundary.md](references/module-slice-boundary.md) 填覆盖矩阵。
   - 本轮切片覆盖哪些层，不覆盖哪些层。
   - 未覆盖层必须写入剩余问题，禁止把切片写成模块完成。
5. 建 worktree：

```bash
git worktree add -b agent/<slug> ../wms-agent-<slug> HEAD
```

6. 在子 worktree 跑：

```bash
timeout --foreground --signal=TERM --kill-after=10s 1200s codex exec --json -C ../wms-agent-<slug> -s workspace-write --ephemeral -m <model> -o ../wms-agent-<slug>.out.md "<任务提示词>" > /tmp/wms-agent-<slug>.log 2>&1
```

上面的单进程命令只用于串行、低输出校准；正式并行批次必须使用实时日志门禁封装，不能只依赖主代理轮询。

并行调度时每个封装调用必须后台启动（`run_agent ... &`），批次末尾再统一 `wait`；启动后逐个核对 slug、PID、PGID、报告路径，禁止因第一个同步调用阻塞而遗漏后续代理：

```bash
log=/tmp/wms-agent-<slug>.log
report=../wms-agent-<slug>.out.md
setsid bash -c 'exec timeout --foreground --signal=TERM --kill-after=10s 1200s codex exec --json -C ../wms-agent-<slug> -s workspace-write --ephemeral -m <model> -o "$1" "$2" >/dev/null 2>"$3"' _ "$report" "<任务提示词>" "$log" &
agent_pid=$!
agent_pgid=$(ps -o pgid= -p "$agent_pid" 2>/dev/null | tr -d ' ')
stop_agent() {
  if [ -n "${agent_pgid:-}" ]; then kill -TERM -- -"$agent_pgid" 2>/dev/null || true; fi
  kill -TERM "$agent_pid" 2>/dev/null || true
  sleep 2
  if [ -n "${agent_pgid:-}" ]; then kill -KILL -- -"$agent_pgid" 2>/dev/null || true; fi
}
trap 'stop_agent; exit 143' TERM INT
while kill -0 "$agent_pid" 2>/dev/null; do
  state=$(ps -o stat= -p "$agent_pid" 2>/dev/null | tr -d ' ')
  case "$state" in
    Z*|"") break ;;
  esac
  lines=$(wc -l < "$log" 2>/dev/null || printf '0')
  bytes=$(wc -c < "$log" 2>/dev/null || printf '0')
  if [ "$lines" -gt 600 ] || [ "$bytes" -gt 204800 ]; then
    stop_agent
    printf 'log gate exceeded: lines=%s bytes=%s\n' "$lines" "$bytes" >> "$log"
    break
  fi
  sleep 2
done
wait "$agent_pid" || true
```

`read-only` 和 `read-only-current-diff` 只替换 sandbox 参数；每个并行代理必须使用独立 `log/report`，记录 `agent_pid/PGID`，并在门禁触发后删除或归档临时输出。若封装本身不可用，停止启动代理，不能退回到无监控后台命令。启动前必须先执行 `df -h . /tmp`，启动后每 30 秒记录一次进程状态；门禁触发时不得读取过程日志或报告并继续采用结论。需要人工中止时必须调用封装内的 `stop_agent`，同时终止 wrapper、timeout、Codex、sandbox 和编译子进程，不能只终止外层等待 shell。

加新 `codex exec` 参数前先用 `codex exec --help` 核对。未跟踪输入必须显式纳入。

只读校准命令：

```bash
timeout --foreground --signal=TERM --kill-after=10s 1200s codex exec --json -C ../wms-agent-<slug> -s read-only --ephemeral -m <model> -o ../wms-agent-<slug>.out.md "<只读校准提示词>" > /tmp/wms-agent-<slug>.log 2>&1
```

审查当前未提交差异：

```bash
codex exec -C "$PWD" -s read-only --ephemeral -m <model> -o /tmp/wms-agent-<slug>.out.md "<只读审查提示词>"
```

没有指定模型时省略 `-m <model>`。

## 子代理任务提示词

按 [references/subagent-task-template.md](references/subagent-task-template.md) 生成提示词；不得省略边界、范围、验证和输出契约。

## 主代理复盘与合并

子代理完成后，主代理按 [references/closeout.md](references/closeout.md) 检查输出、worktree、diff、忽略产物、tmux 和分支；当前不创建 Gitea PR。`read-only-current-diff` 只汇总发现，不进入合并流程。

主代理只在 `write-worktree` 以下条件全部满足时考虑合并：

- 主工作区没有无关脏改动；否则先按 `wms-review-fix-commit` 把已有脏区 review、验证并按主题提交，再回到当前合并。
- 子代理输出写明“本切片可合并”，且“是否可合并”为“是”。
- 覆盖矩阵没有把缺口误写成完成。
- 子代理列出的验证命令和退出码满足任务提示词。
- 子代理修改文件都在授权写入范围内。
- 子代理没有推送、没有改 main、没有提交真实凭据或生产数据。

合并方式按产物形态选择：

- 有提交：`git merge --no-ff agent/<slug>` 或 `git cherry-pick <hash>`。
- 无提交：审查 diff 后用 `git diff --binary | git apply --3way` 接入授权文件。
- 不可合并、验证失败、越权或需业务确认：不合并。
- 已关闭 issue 分支：主工作区先干净化，再合入 issue 分支；合入后必须单独跑相关测试、`git diff --check` 和 `just gov-t1`，再单独提交或保留合并提交。

## 旧 worktree 迁移

遗留 worktree 基线落后、与主线重叠、形态过期、不可合并或缺验证时，禁止直接合并旧 diff；从当前 `HEAD` 新建迁移 worktree 重做。

合并或接入 diff 后，主代理必须立即进入 `wms-review-fix-commit`：

1. 在主工作区运行 `git status --short` 和 `git diff --stat`。
2. 按 `wms-review-fix-commit` 做 review → 修复 → 再 review。
3. 主工作区重新跑 `git diff --check`、`just gov-t1` 和相关测试；子代理验证不能替代。
4. 验证通过后由主代理按主题显式 `git add <file...>` 并提交；禁止 `git add .`。
5. 验证失败不提交；报告失败命令、退出码和下一步。

## PR、tmux 与 worktree 收口

按 [references/closeout.md](references/closeout.md) 归类 tmux/worktree/agent 分支。issue-agent 按 `docs/runbooks/gitea-issue-agent.md` 本地收口。前端修复必须上传真实截图。`codex exec` 不保活前端；看 worktree 分支用 `just dev-web-worktree-restart <worktree> <port>`。清理前必须完成主工作区 review、验证和提交。

## 迭代本技能

每次子代理跑完，只补必要规则：

- 子代理漏读文档：补到“必须先读”。
- 子代理改超范围：收紧“写入范围”模板。
- 子代理没验证：收紧任务模板里的验证规则。
- 子代理尝试提交或 Git 元数据只读卡住：收紧模板，主代理接管提交。
- 子代理把切片误报为模块完成：收紧“完整性边界”和“覆盖矩阵”。
- 低磁盘、pnpm、本地凭据或外部服务阻断：补停止条件。
- 生成链路失败后手写产物：默认不可合并，主代理复跑生成器。
- 子代理切片过大：补写文件数、行数和行为预算，超预算必须停下汇报。
- 子代理依赖缺失仍继续造临时 shim 或把完整 diff 写入过程输出：收紧预检和输出契约，主代理不可合并该批改动。
- 子代理输出不可审查：收紧“最终输出”字段。
- 子代理从旧 `HEAD` 启动漏读主区未提交技能，或过程输出污染主会话：启动前比较技能基线；`codex exec` 必须重定向日志，使用 20 分钟 timeout 和 600 行/200 KiB 日志硬上限，主代理只读取最终报告并按退出码收口。
- 子代理用全仓 glob/递归检索替代授权读取，或过程输出超过门限：把检索范围、文件数和输出行数写入启动提示词；主代理终止该代理并标记不可合并。
- 仅在提示词中要求“少输出”仍可能因代理持续命令输出而失效；并行批次必须使用实时日志门禁封装，超限要自动终止整个进程组，禁止读取超限过程日志并继续采用其结论。
- 调度了已在 `stories`/`verified` 中的故事：启动前强制读取质量矩阵分区并把 `deferred` / `verified` 状态写入任务登记；误选任务不可合并，重新选择延期故事。
- 人工停止只结束外层等待 shell、留下 Codex 或 Rust 编译子进程：记录真实 `agent_pgid`，用 `stop_agent` 先终止 PGID 再终止 wrapper；收口前用 `ps` 确认没有残留 `codex exec`、sandbox 或 worktree `target` 编译进程。
- 原生 subagent `close_agent` 后仍可能保留底层执行进程；`resume_agent` 前必须执行 worktree 路径级残留扫描。若同一 worktree 出现两个 `codex exec`，立即终止整个旧 PGID，当前批次标记“不可合并”，不得读取并发写入产生的 diff 或报告作为证据。
- 并行启动脚本若把代理调用写成同步序列，后续任务未启动也必须记录为调度失败；修正为每个任务后台启动、批次统一等待，并重新核对全部报告，不得用单个代理结果代表整批。
- 留下未解释 worktree：收紧收尾门禁。

技能文件实际修改后运行 `git diff --check` 和 `just gov-t1`，再由主代理按 `wms-review-fix-commit` 提交；只读模式本身不触发提交。
