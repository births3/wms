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

用户指定模型时使用 `-m <model>`；未指定时沿用本机 Codex 默认值。启动前用 `codex exec --help` 确认参数，模型不可用时停止并报告，禁止静默降级。

## 并行规则

- 默认最多 3 个子代理并行；任务存在依赖时按顺序运行。
- 写任务必须使用互不重叠的授权路径；只读任务可以读取同一文件，但审查目标必须不同。
- 每个子代理使用唯一 slug 和输出文件；主代理记录任务、模式、模型、范围、依赖、进程状态和退出码。
- 主代理等待本批全部进程退出后再修改当前工作区、汇总结论或启动下一批。

## 子代理原则

- 写任务一个任务一个 worktree；只有 `read-only-current-diff` 可以读取主工作区。
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
codex exec -C ../wms-agent-<slug> -s workspace-write --ephemeral -m <model> -o ../wms-agent-<slug>.out.md "<任务提示词>"
```

加新 `codex exec` 参数前先用 `codex exec --help` 核对。未跟踪输入必须显式纳入。

只读校准命令：

```bash
codex exec -C ../wms-agent-<slug> -s read-only --ephemeral -m <model> -o ../wms-agent-<slug>.out.md "<只读校准提示词>"
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
- 子代理输出不可审查：收紧“最终输出”字段。
- 留下未解释 worktree：收紧收尾门禁。

技能文件实际修改后运行 `git diff --check` 和 `just gov-t1`，再由主代理按 `wms-review-fix-commit` 提交；只读模式本身不触发提交。
