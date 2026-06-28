---
name: wms-worktree-subagent
description: WMS 仓库用独立 worktree 和 codex exec 运行子代理、复盘输出并迭代子代理提示词的流程。用户要求建立 subagent、用 worktree 跑并行任务、codex exec 子任务、先跑一个再迭代 subagent、或让主代理负责 review/merge 子代理结果时使用。
---

# WMS Worktree Subagent

用于把一个 WMS 缺口拆给独立子代理执行：每个子代理只拥有一个 worktree、一个任务、一个写入范围。主代理负责拆题、发任务、审查、合并和迭代本技能。

## 子代理原则

- 一个任务一个 worktree，不共享主工作区。
- 子代理只改明确授权的文件范围；不推送、不改 main、不跨任务抢文件。
- 子代理必须知道自己不是唯一修改者：不得回滚他人变更，遇到冲突要适配。
- 默认要求子代理使用 `wms-loop-engineering` 和 `wms-review-fix-commit`。`workspace-write` 子代理只留下可审查修改和最终报告，由主代理复查后显式暂存/提交；只有 sandbox 明确可写 `.git/worktrees/<worktree>` 时才要求子代理本地提交。
- 只读校准任务用于先跑通子代理和收敛切片：使用 `read-only` sandbox，不改文件、不提交，只输出下一轮切片、允许文件、停止条件、验证命令和技能缺口。
- 外部设备、TMS、冷链平台、生产数据和凭据类 evidence 不能交给子代理伪造；只能让子代理整理采集步骤或验证已有证据。

## 建立子代理

1. 主工作区先运行：
   - `git status --short --branch`
   - `git worktree list`
2. 任务命名用短 slug，例如 `m2-inbound-pc`。
3. 建 worktree：

```bash
git worktree add -b agent/<slug> ../wms-agent-<slug> HEAD
```

4. 在子 worktree 跑：

```bash
codex exec -C ../wms-agent-<slug> -s workspace-write -o ../wms-agent-<slug>.out.md "<任务提示词>"
```

当前 `codex exec` 不接受顶层交互命令的 `-a/--ask-for-approval` 参数；需要加新参数前先用 `codex exec --help` 核对。
如果要求子代理本地提交，sandbox 必须允许写入主仓库的 `.git/worktrees/<worktree>` 元数据；否则子代理只能留下工作区修改，主代理负责 `git add`/`git commit`。主工作区有 `??` 未跟踪文件且子代理任务依赖这些文件时，不要只用 `git stash create` 做基线；先把文件内容显式纳入子代理输入，或由主代理先落最小文件。

只读校准命令：

```bash
codex exec -C ../wms-agent-<slug> -s read-only -o ../wms-agent-<slug>.out.md "<只读校准提示词>"
```

## 子代理任务提示词模板

```text
你是 WMS 子代理，工作区是独立 worktree。

目标：<一句话目标>

写入范围：
- 允许：<目录/文件>
- 禁止：主工作区、main 分支、推送、真实凭据、生产数据、未授权模块

必须先读：
- AGENTS.md
- 相关 */AGENTS.override.md
- docs/requirements-traceability-matrix.md
- 任务相关用户故事、设计文档、ADR 或 runbook

执行规则：
1. 使用 wms-loop-engineering 定义目标、输入、检查、反馈和停止条件。
2. 复用现有前后端模块、组件、API client、测试夹具和治理脚本。
3. 只做本任务最小闭环；新增字段、状态、角色、模块或业务默认值时停止并说明需要主代理/用户确认。
4. 检索时限制范围，优先 `rg -n "<关键词>" <相关目录> --glob '!node_modules/**' --glob '!target/**'`，避免把无关大输出塞进结果；长文档只读相关章节，禁止反复 `cat` 整份大文档或整份 diff。
5. 输出保持可审查：不要反复打印完整 `git diff`；需要复盘时用 `git diff --stat`、文件清单和关键错误摘要。
6. API 变更必须同步 `shared/openapi/openapi.json` 和 `packages/api-client/src/schema.ts`，并运行 `just openapi-sync`、`just openapi-check`。如果 pnpm/corepack/网络导致生成器失败，记录失败点和已同步文件，禁止盲目重试。
7. Rust 命令必须在 `backend/` 下运行，或使用 `cargo --manifest-path backend/Cargo.toml ...`；从仓库根目录直接跑 `cargo test` 视为无效验证。
8. 重编译、截图或大验证前先跑 `df -h . /tmp`；可用空间不足 2GiB 时停止重型命令，只跑轻量检查并把磁盘阻断写入最终输出。
9. 真实前端任务必须在提示词里明确 9002 端口、截图路径、视口、是否提交 artifact；禁止用原型图代替真实页面截图。
10. 非平凡逻辑留下最小测试。
11. 运行 git diff --check、just gov-t1 和任务相关测试；任一失败时最终输出必须写“不可合并”，不得建议主代理合并。
12. 使用 wms-review-fix-commit 做 review→修复→review；验证通过且 Git 元数据可写时本地分组提交，否则停止在可审查工作区。
13. 不推送。

最终输出：
- 子 worktree 路径
- 提交哈希；未提交时写 `无` 并说明阻塞
- 修改文件
- 验证命令和退出码
- 是否可合并
- 剩余问题/需要确认事项
```

## 主代理复盘与合并

子代理完成后，主代理在主工作区检查：

```bash
git status --short --branch
git -C ../wms-agent-<slug> status --short --branch
git -C ../wms-agent-<slug> log --oneline -5
git -C ../wms-agent-<slug> diff --stat
git -C ../wms-agent-<slug> show --stat --oneline <hash>  # 子代理有提交时
```

主代理只在以下条件全部满足时考虑合并：

- 主工作区没有无关脏改动；如有，先停下说明，不用 stash 掩盖。
- 子代理最终输出写明“可合并”。
- 子代理列出的验证命令和退出码满足任务提示词。
- 子代理修改文件都在授权写入范围内。
- 子代理没有推送、没有改 main、没有提交真实凭据或生产数据。

合并方式按产物形态选择：

- 子代理有本地提交：优先在主工作区 `git merge --no-ff agent/<slug>`；只取单个提交时用 `git cherry-pick <hash>`。
- 子代理验证通过但因 Git 元数据只读无法提交：先在主工作区审查 `git -C ../wms-agent-<slug> diff --stat` 和具体 diff，再用 `git -C ../wms-agent-<slug> diff --binary | git apply --3way` 接入；只允许接入授权范围文件。
- 子代理最终输出“不可合并”、必需验证失败、写入越权或业务语义需确认：不合并，只把产物当草稿或问题报告。

合并或接入 diff 后，主代理必须立即进入 `wms-review-fix-commit`：

1. 在主工作区运行 `git status --short` 和 `git diff --stat`。
2. 按 `wms-review-fix-commit` 做 review → 修复 → 再 review。
3. 重新运行主工作区验证：至少 `git diff --check`、`just gov-t1`，以及本任务相关测试；子代理验证不能替代主工作区验证。
4. 验证通过后由主代理按主题显式 `git add <file...>` 并提交；禁止 `git add .`。
5. 验证失败则不提交，保留主工作区差异并报告失败命令、退出码和下一步；不要推送。

主代理最终汇报必须区分：

- 子代理结果：路径、提交哈希或无、是否可合并。
- 主工作区结果：是否已合并、主工作区验证、主代理提交哈希。
- 未合并产物：原因和保留路径。

## 迭代本技能

每次子代理跑完，按结果只改本技能中必要的一两条规则：

- 子代理漏读文档：补到“必须先读”。
- 子代理改超范围：收紧“写入范围”模板。
- 子代理没验证：收紧“执行规则”第 5 条。
- 子代理没提交：检查 `.git/worktrees/<worktree>` 写权限，必要时由主代理接管提交。
- 子代理遇到低磁盘、pnpm、本地凭据或外部服务阻断：补停止条件，禁止原地循环重试。
- 子代理输出不可审查：收紧“最终输出”字段。

迭代后运行 `git diff --check` 和 `just gov-t1`，再按 `wms-review-fix-commit` 提交。
