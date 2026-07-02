---
name: wms-worktree-subagent
description: WMS 仓库用独立 worktree 和 codex exec 运行子代理、复盘输出、主代理合并、清理收尾并迭代子代理提示词的流程。用户要求建立 subagent、用 worktree 跑并行任务、codex exec 子任务、先跑一个再迭代 subagent、让主代理负责 review/merge、清理 worktree、检查遗留 agent worktree 时使用。
---

# WMS Worktree Subagent

用于把一个 WMS 缺口拆给子代理执行：每个子代理只拥有一个 worktree、一个任务、一个写入范围。主代理负责拆题、发任务、审查、本地接入和迭代本技能。

## 子代理原则

- 一个任务一个 worktree，不共享主工作区。
- 子代理只改授权范围；不推送、不改 main。
- 对话中启动的普通 worktree 子代理默认不创建远端 PR；只交付本地 diff、验证结果和清理建议。只有 Gitea issue-agent 异步任务才创建 PR。
- 默认使用 `wms-loop-engineering` 和 `wms-review-fix-commit` 做自审；子代理默认不 `git add` / `git commit`，只留可审查 diff、验证结果和可合并结论，由主代理提交。
- 只读校准用 `read-only`，只输出下一轮切片、允许文件、停止条件、验证命令和技能缺口。
- 外部设备、TMS、冷链平台、生产数据和凭据类 evidence 不能交给子代理伪造；只能让子代理整理采集步骤或验证已有证据。
- 构建缓存和大产物不得合并；污染必须在最终输出标记。
- `pnpm`、OpenAPI 生成器、数据库连接、`.env`、网络或外部服务类必需验证失败时，最多做一次明确修复/重试；第二次仍失败必须输出“不可合并”，禁止继续包装命令、手写生成物或原地循环。
- 子代理输出必须以结论和摘要为主；禁止在最终报告粘贴大段 diff，完整 diff 由主代理用 `git diff` 审查。

## 建立子代理

1. 主工作区先运行：
   - `git status --short --branch`
   - `git worktree list`
2. 任务命名用短 slug，例如 `m2-inbound-pc`。
3. 如果本技能或任务依赖的未跟踪文件未提交，新 worktree 默认看不到；先提交、先落最小文件，或把当前规则显式写入提示词。
4. 主代理先写清任务完整性边界；没有边界，不建 worktree：
   - 按 [references/module-slice-boundary.md](references/module-slice-boundary.md) 填覆盖矩阵。
   - 本轮切片覆盖哪些层，不覆盖哪些层。
   - 子代理最终不得把“切片完成”写成“模块完成”；未覆盖层必须写入剩余问题。
5. 建 worktree：

```bash
git worktree add -b agent/<slug> ../wms-agent-<slug> HEAD
```

6. 在子 worktree 跑：

```bash
codex exec -C ../wms-agent-<slug> -s workspace-write -o ../wms-agent-<slug>.out.md "<任务提示词>"
```

当前 `codex exec` 不接受顶层交互命令的 `-a/--ask-for-approval` 参数；需要加新参数前先用 `codex exec --help` 核对。
主工作区有未跟踪文件且子代理依赖它们时，先显式纳入输入，或由主代理先落最小文件。

只读校准命令：

```bash
codex exec -C ../wms-agent-<slug> -s read-only -o ../wms-agent-<slug>.out.md "<只读校准提示词>"
```

## 子代理任务提示词

主代理按 [references/subagent-task-template.md](references/subagent-task-template.md) 生成提示词；不得省略边界、范围、验证和输出契约。

## 主代理复盘与合并

子代理完成后，主代理在主工作区按 [references/closeout.md](references/closeout.md) 检查 worktree、提交、diff、忽略产物、tmux 和分支；PR 只按 issue-agent 例外流程处理。

主代理只在以下条件全部满足时考虑合并：

- 主工作区没有无关脏改动；如有，先停下说明。
- 子代理最终输出写明“模块完成”或“本切片可合并”，且“是否可合并”为“是”。
- 覆盖矩阵没有把缺口误写成完成；存在未覆盖层时只能汇报“切片已合并”。
- 子代理列出的验证命令和退出码满足任务提示词。
- 子代理修改文件都在授权写入范围内。
- 子代理没有推送、没有改 main、没有提交真实凭据或生产数据。

合并方式按产物形态选择：

- 子代理有本地提交：优先在主工作区 `git merge --no-ff agent/<slug>`；只取单个提交时用 `git cherry-pick <hash>`。
- 子代理验证通过且没有本地提交：先在主工作区审查 `git -C ../wms-agent-<slug> diff --stat` 和具体 diff，再用 `git -C ../wms-agent-<slug> diff --binary | git apply --3way` 接入；只允许接入授权范围文件。
- 子代理最终输出“不可合并”、必需验证失败、写入越权或业务语义需确认：不合并，只把产物当草稿或问题报告。

## 旧 worktree 迁移

遗留 worktree 基线落后、与主线脏改重叠、形态过期、不可合并或缺验证时，禁止直接合并旧 diff。先提取保留能力清单，再从当前 `HEAD` 新建迁移 worktree，并按现有结构重做。

合并或接入 diff 后，主代理必须立即进入 `wms-review-fix-commit`：

1. 在主工作区运行 `git status --short` 和 `git diff --stat`。
2. 按 `wms-review-fix-commit` 做 review → 修复 → 再 review。
3. 重新运行主工作区验证：至少 `git diff --check`、`just gov-t1`，以及本任务相关测试；子代理验证不能替代主工作区验证。
4. 验证通过后由主代理按主题显式 `git add <file...>` 并提交；禁止 `git add .`。
5. 验证失败则不提交，保留主工作区差异并报告失败命令、退出码和下一步；不要推送。

## PR、tmux 与 worktree 收口

详细矩阵、命令和分支清理规则见 [references/closeout.md](references/closeout.md)。每次合并、放弃或审查结束后，主代理必须归类 tmux/worktree/agent 分支；issue-agent PR 另按 `docs/runbooks/gitea-issue-agent.md` 收口。前端或用户可见修复必须把截图作为 Gitea 附件评论到对应 issue；如存在 issue-agent PR，也同步评论到 PR。清理前必须完成主工作区 review、验证和提交；最终汇报要区分子代理结果、主工作区结果、未合并产物和收尾状态。

## 迭代本技能

每次子代理跑完，按结果只改本技能中必要的一两条规则：

- 子代理漏读文档：补到“必须先读”。
- 子代理改超范围：收紧“写入范围”模板。
- 子代理没验证：收紧任务模板里的验证规则。
- 子代理尝试提交或因 Git 元数据只读卡住：收紧任务模板，默认禁止子代理提交，由主代理接管提交。
- 子代理把切片误报为模块完成：收紧“完整性边界”和“覆盖矩阵”。
- 子代理遇到低磁盘、pnpm、本地凭据或外部服务阻断：补停止条件，禁止原地循环重试。
- 子代理生成链路失败后手写生成产物：补强契约规则，默认不可合并，必须由主代理复跑生成器。
- 子代理切片过大：补写文件数、行数和行为预算，超预算必须停下汇报。
- 子代理输出不可审查：收紧“最终输出”字段。
- 子代理或主代理留下未解释的 worktree：收紧“主代理强制收尾门禁”，必要时补清理矩阵字段。

迭代后运行 `git diff --check` 和 `just gov-t1`，再按 `wms-review-fix-commit` 提交。
