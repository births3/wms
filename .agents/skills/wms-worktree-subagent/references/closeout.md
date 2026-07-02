# WMS Worktree Subagent 收口参考

本参考承接 `wms-worktree-subagent` 的 PR、tmux、worktree 和 `agent/*` 分支收口细节。`SKILL.md` 只保留路由和停止条件；执行到收口阶段时读取本文件。

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
| 已合并可强制移除 | 子代理修改已由主代理合并并提交，或用户明确放弃；但子 worktree 仍脏 | 归入“已合并待清理”；用户已明确要求清理/丢弃时才 `git worktree remove --force <path>` |
| 不可合并需保留 | 子代理报告不可合并、验证失败、越权或需要业务确认 | 保留路径和原因，不删除 |
| 缓存污染 | 只有 `node_modules/`、pnpm store、`.vite-temp/`、构建缓存等无业务产物 | 记录污染来源；用户明确清理时优先用 `git worktree remove --force <path>` 移除整个 worktree，不手工删 `.git` |
| 未判断 | 缺少最终报告、缺少验证结果或无法确认是否已合并 | 保留并列下一步审查命令 |

3. 禁止事项：

- 禁止用 `git clean -f` 掩盖子代理污染。
- 禁止手工删除 worktree 目录里的 `.git` 文件或主仓库 `.git/worktrees/*` 元数据。
- 禁止删除 `agent/*` 分支，除非用户显式要求。
- 禁止把清理 worktree 当作合并验证；清理前必须先完成主工作区 review、验证和提交。

## 遗留 worktree 分流

清理矩阵中所有“保留”的 worktree 必须进入分流队列，不能只写“保留”。主代理在同一轮最终汇报中输出“遗留 worktree 队列”。

| 队列 | 进入条件 | 下一步动作 |
|---|---|---|
| 待主代理合并 | 子代理报告可合并，验证为 0，但尚未接入主工作区 | 本轮继续复审 diff；能合并就接入、验证、提交、再清理；不能合并则降级到待用户确认取舍或待修复再验证 |
| 待修复再验证 | 子代理不可合并，但失败点明确且可修，例如缺 env、构建失败、截图失败 | 建新修复任务或主代理接手修复；修复后重新跑相关验证；不允许长期只保留 |
| 待用户确认取舍 | 与主工作区已有实现重叠、方向冲突、可能覆盖用户改动 | 输出对比点、风险和建议；用户确认前不合并也不强删 |
| 已合并待清理 | 子代理 diff 已由主代理合入、验证并提交，但子 worktree 仍保留同一批脏改 | 用户确认清理后 `git worktree remove --force <path>`；没有确认前不得长期误列为“未判断” |
| 待用户确认丢弃 | 产物过期、已被主工作区更完整实现替代、只有缓存污染或方向错误 | 用户确认丢弃后 `git worktree remove --force <path>`；未确认前保留 |

用户要求“收尾”“清理”“全部处理”时，`待主代理合并` 不能长期停留；主代理必须处理到“已合并待清理”或写出具体阻塞。`待修复再验证` 必须写 owner、动作和验证命令。若同一队列超过 3 个 worktree，优先拆成批处理或子代理任务，禁止用笼统的“后续处理”结束。

## 已合并分支清理

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

## 最终汇报

最终汇报必须包含：

- PR 收口状态：已合并、可合并待确认、阻塞或放弃待清理。
- tmux 任务会话状态：已自然退出、已清理、仍运行或需用户确认。
- 已移除 worktree 列表。
- 已删除 agent 分支列表；若用户要求删除但未删除，说明未合并或未确认原因。
- 保留 worktree 列表和保留原因。
- 遗留 worktree 队列、下一步动作和需要用户确认的项。
- 是否还有脏 worktree。
- 如果用户要求“全部清理”，但存在不可合并或未判断 worktree，必须停止并逐项说明，不能静默强删。
