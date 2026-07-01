---
name: wms-worktree-subagent
description: WMS 仓库用独立 worktree 和 codex exec 运行子代理、复盘输出、主代理合并、清理收尾并迭代子代理提示词的流程。用户要求建立 subagent、用 worktree 跑并行任务、codex exec 子任务、先跑一个再迭代 subagent、让主代理负责 review/merge、清理 worktree、检查遗留 agent worktree 时使用。
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
- 子代理不得把 `node_modules/`、`.vite-temp/`、临时 pnpm store、`target/` 以外的大型构建产物留作待合并内容；如验证命令产生缓存污染，最终输出必须显式列出并标记为“缓存污染，不能合并”。

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
9. 真实前端任务必须在提示词里明确 9002 端口、截图路径、视口、是否上传 Gitea 附件；禁止用原型图代替真实页面截图。issue 或 PR 已有截图 / 附件时，必须先打开或下载附件辅助定位。
10. 非平凡逻辑留下最小测试。
11. 运行 git diff --check、just gov-t1 和任务相关测试；任一失败时最终输出必须写“不可合并”，不得建议主代理合并。
12. 使用 wms-review-fix-commit 做 review→修复→review；验证通过且 Git 元数据可写时本地分组提交，否则停止在可审查工作区。
13. 不推送。
14. 不运行 `git clean -f`、`git reset --hard` 或删除分支；依赖安装和构建缓存优先使用 `/tmp` 或任务限定目录，避免污染子 worktree。

最终输出：
- 子 worktree 路径
- 提交哈希；未提交时写 `无` 并说明阻塞
- 修改文件
- `git status --short` 输出；无输出也写“干净”
- `git diff --stat` 摘要；只读审查任务写“不适用”
- 未跟踪/忽略产物摘要，特别是 `node_modules/`、构建缓存、截图和测试报告
- 验证命令和退出码
- 是否可合并
- 剩余问题/需要确认事项
- 清理建议：`可普通移除`、`已合并可强制移除`、`不可合并需保留` 或 `只读审查可移除`
```

## 主代理复盘与合并

子代理完成后，主代理在主工作区检查：

```bash
git status --short --branch
git -C ../wms-agent-<slug> status --short --branch
git -C ../wms-agent-<slug> log --oneline -5
git -C ../wms-agent-<slug> diff --stat
git -C ../wms-agent-<slug> status --short --ignored
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

## 旧 worktree 迁移

遗留 worktree 满足任一条件时，禁止在主工作区直接 `merge`、`cherry-pick` 或 `git apply` 旧 diff：

- worktree 的基线落后当前主工作区，目标文件在主线已有后续提交。
- worktree 修改文件与主工作区现有脏改重叠。
- worktree 的页面、组件、API 或字段形态已被主线重构。
- 子代理最终输出写明“不可合并”或缺少完整验证。

处理方式：

1. 把旧 worktree 只当参考材料，先提取“保留能力清单”：用户可见行为、API 调用、组件拆分、测试和文档价值。
2. 从当前主工作区 `HEAD` 新建迁移 worktree：`git worktree add -b agent/<slug>-migrate ../wms-agent-<slug>-migrate HEAD`。
3. 迁移提示词必须写清“禁止整文件复制旧实现；必须先读当前主线文件，再把保留能力按现有结构重新实现”。
4. 迁移时禁止复制旧 worktree 的 `node_modules/`、构建产物、截图缓存和 `.vite-temp/`。
5. 迁移完成后按“主代理复盘与合并”重新审查、验证、提交；旧 worktree 只有在迁移提交落地或用户确认丢弃后才能强制移除。

合并或接入 diff 后，主代理必须立即进入 `wms-review-fix-commit`：

1. 在主工作区运行 `git status --short` 和 `git diff --stat`。
2. 按 `wms-review-fix-commit` 做 review → 修复 → 再 review。
3. 重新运行主工作区验证：至少 `git diff --check`、`just gov-t1`，以及本任务相关测试；子代理验证不能替代主工作区验证。
4. 验证通过后由主代理按主题显式 `git add <file...>` 并提交；禁止 `git add .`。
5. 验证失败则不提交，保留主工作区差异并报告失败命令、退出码和下一步；不要推送。

## PR 与 tmux 收口

子代理或 issue agent 创建 PR 后，PR 不是完成态。主代理必须把每个 PR 分到以下状态之一，并在 issue / PR 评论中写清：

| 状态 | 条件 | 下一步 |
|---|---|---|
| 可合并待确认 | 主工作区复审和验证通过，但合并会进入远端主线 | 等用户明确确认合并；确认前不 merge |
| 已合并待清理 | 用户已确认合并，PR 已 merge，主工作区验证通过 | 进入 worktree、agent 分支和 tmux 清理 |
| 阻塞 | 验证失败、冲突、缺截图、缺前后端重启证据、缺用户业务确认 | 写明阻塞命令、退出码、owner 和下一步 |
| 放弃待清理 | 用户确认放弃，或 PR 被更完整实现替代 | 清理 worktree 和分支；未确认前不强删 |

tmux 收口规则：

- `wms-issue-<issue>-<时间>` 任务会话正常跑完后应自然退出；主代理用 `tmux has-session -t <session>` 或 `tmux ls` 记录结果。
- 会话仍在运行时，先 `tmux capture-pane -pt <session>` 看是否仍在执行、等待输入或卡死；不能把正在执行的会话当垃圾清理。
- 已合并或已放弃且不再需要保留日志时，用户已要求“清理/收尾”即可 `tmux kill-session -t <session>`；未确认放弃的阻塞会话只记录，不强杀。
- 最终汇报必须写：PR 状态、是否已 merge、tmux 会话是否仍存在、清理动作或保留原因。
- 前端或用户可见修复的截图必须用 `POST /repos/{owner}/{repo}/issues/<编号>/assets` 上传为 Gitea 附件，并用 Markdown 图片同时评论到 PR 和 issue；只写本地截图路径不算闭环。

## 主代理强制收尾门禁

每次子代理合并、放弃或审查结束后，主代理必须做 worktree 收尾；这一步是本技能的停止条件之一，不能省略。

1. 在主工作区列出所有 agent worktree：

```bash
git worktree list
for d in ../wms-agent-*; do
  [ -e "$d" ] || continue
  git -C "$d" rev-parse --is-inside-work-tree >/dev/null 2>&1 || continue
  printf '\n== %s ==\n' "$d"
  git -C "$d" status --short --branch
  git -C "$d" diff --stat
  git -C "$d" status --short --ignored | sed -n '1,40p'
done
```

2. 输出清理矩阵：

| 分类 | 判断 | 处理 |
|---|---|---|
| 干净可删 | `status --short` 无源码/未跟踪输出，且不是当前任务保留基线 | `git worktree remove <path>` |
| 已合并可强制移除 | 子代理修改已由主代理合并并提交，或用户明确放弃；但子 worktree 仍脏 | 用户已明确要求清理/丢弃时才 `git worktree remove --force <path>` |
| 不可合并需保留 | 子代理报告不可合并、验证失败、越权或需要业务确认 | 保留路径和原因，不删除 |
| 缓存污染 | 只有 `node_modules/`、pnpm store、`.vite-temp/`、构建缓存等无业务产物 | 记录污染来源；用户明确清理时优先用 `git worktree remove --force <path>` 移除整个 worktree，不手工删 `.git` |
| 未判断 | 缺少最终报告、缺少验证结果或无法确认是否已合并 | 保留并列下一步审查命令 |

3. 禁止事项：
   - 禁止用 `git clean -f` 掩盖子代理污染。
   - 禁止手工删除 worktree 目录里的 `.git` 文件或主仓库 `.git/worktrees/*` 元数据。
   - 禁止删除 `agent/*` 分支，除非用户显式要求。
   - 禁止把清理 worktree 当作合并验证；清理前必须先完成主工作区 review、验证和提交。

4. 遗留 worktree 分流：

   清理矩阵中所有“保留”的 worktree 必须进入分流队列，不能只写“保留”。主代理在同一轮最终汇报中输出“遗留 worktree 队列”。

   | 队列 | 进入条件 | 下一步动作 |
   |---|---|---|
   | 待主代理合并 | 子代理报告可合并，验证为 0，但尚未接入主工作区 | 本轮继续复审 diff；能合并就接入、验证、提交、再清理；不能合并则降级到待用户确认取舍或待修复再验证 |
   | 待修复再验证 | 子代理不可合并，但失败点明确且可修，例如缺 env、构建失败、截图失败 | 建新修复任务或主代理接手修复；修复后重新跑相关验证；不允许长期只保留 |
   | 待用户确认取舍 | 与主工作区已有实现重叠、方向冲突、可能覆盖用户改动 | 输出对比点、风险和建议；用户确认前不合并也不强删 |
   | 待用户确认丢弃 | 产物过期、已被主工作区更完整实现替代、只有缓存污染或方向错误 | 用户确认丢弃后 `git worktree remove --force <path>`；未确认前保留 |

   用户要求“收尾”“清理”“全部处理”时，`待主代理合并` 不能长期停留；主代理必须处理到“已合并清理”或写出具体阻塞。`待修复再验证` 必须写 owner、动作和验证命令。若同一队列超过 3 个 worktree，优先拆成批处理或子代理任务，禁止用笼统的“后续处理”结束。

5. 已合并分支清理：

   worktree 目录清理后，还必须检查对应 `agent/*` 分支是否已经完全并入当前主工作区分支。

   - 已通过主工作区 review、验证和提交，且 `git branch --merged HEAD` 包含该 `agent/*` 分支时，用户已要求“清理/收尾/删除已合并分支”即可用 `git branch -d <branch>` 普通删除。
   - 未出现在 `git branch --merged HEAD` 的分支禁止用 `git branch -d` 假装完成；输出未合并原因和下一步合并或取舍动作。
   - `git branch -D <branch>` 只允许在用户明确确认丢弃未合并产物时使用。
   - 删除分支前后都要输出 `git branch --list '<branch>'` 或等价核查，证明删除对象准确。

   批量清理已合并 `agent/*` 分支时使用以下命令块；它只会普通删除已并入当前 `HEAD` 的分支，并把未合并分支留给后续取舍：

```bash
git status --short --branch
git worktree list

mapfile -t merged_agent_branches < <(
  git branch --merged HEAD --list 'agent/*' --format='%(refname:short)'
)
mapfile -t unmerged_agent_branches < <(
  git branch --no-merged HEAD --list 'agent/*' --format='%(refname:short) %(objectname:short) %(subject)'
)

if ((${#unmerged_agent_branches[@]} > 0)); then
  printf '未合并 agent 分支，禁止自动删除：\n'
  printf '  %s\n' "${unmerged_agent_branches[@]}"
fi

if ((${#merged_agent_branches[@]} > 0)); then
  printf '删除已合并 agent 分支：\n'
  printf '  %s\n' "${merged_agent_branches[@]}"
  git branch -d "${merged_agent_branches[@]}"
else
  printf '没有已合并 agent 分支需要删除。\n'
fi

printf '清理后残留 agent 分支：\n'
git branch --list 'agent/*' --format='%(refname:short) %(objectname:short) %(subject)'
```

6. 最终汇报必须包含：
   - PR 收口状态：已合并、可合并待确认、阻塞或放弃待清理。
   - tmux 任务会话状态：已自然退出、已清理、仍运行或需用户确认。
   - 已移除 worktree 列表。
   - 已删除 agent 分支列表；若用户要求删除但未删除，说明未合并或未确认原因。
   - 保留 worktree 列表和保留原因。
   - 遗留 worktree 队列、下一步动作和需要用户确认的项。
   - 是否还有脏 worktree。
   - 如果用户要求“全部清理”，但存在不可合并或未判断 worktree，必须停止并逐项说明，不能静默强删。

主代理最终汇报必须区分：

- 子代理结果：路径、提交哈希或无、是否可合并。
- 主工作区结果：是否已合并、主工作区验证、主代理提交哈希。
- 未合并产物：原因和保留路径。
- worktree 收尾结果：已移除、仍保留、是否需要用户确认强制移除。

## 迭代本技能

每次子代理跑完，按结果只改本技能中必要的一两条规则：

- 子代理漏读文档：补到“必须先读”。
- 子代理改超范围：收紧“写入范围”模板。
- 子代理没验证：收紧“执行规则”第 5 条。
- 子代理没提交：检查 `.git/worktrees/<worktree>` 写权限，必要时由主代理接管提交。
- 子代理遇到低磁盘、pnpm、本地凭据或外部服务阻断：补停止条件，禁止原地循环重试。
- 子代理输出不可审查：收紧“最终输出”字段。
- 子代理或主代理留下未解释的 worktree：收紧“主代理强制收尾门禁”，必要时补清理矩阵字段。

迭代后运行 `git diff --check` 和 `just gov-t1`，再按 `wms-review-fix-commit` 提交。
