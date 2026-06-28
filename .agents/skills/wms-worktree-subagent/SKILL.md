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
- 默认要求子代理使用 `wms-loop-engineering` 和 `wms-review-fix-commit`，验证通过后在子 worktree 本地分组提交。
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
如果要求子代理本地提交，sandbox 必须允许写入主仓库的 `.git/worktrees/<worktree>` 元数据；否则子代理只能留下工作区修改，主代理负责 `git add`/`git commit`。

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
11. 运行 git diff --check、just gov-t1 和任务相关测试。
12. 使用 wms-review-fix-commit 做 review→修复→review；验证通过后本地分组提交。
13. 不推送。

最终输出：
- 子 worktree 路径
- 提交哈希
- 修改文件
- 验证命令和退出码
- 剩余问题/需要确认事项
```

## 主代理复盘

子代理完成后，主代理在主工作区检查：

```bash
git -C ../wms-agent-<slug> status --short --branch
git -C ../wms-agent-<slug> log --oneline -5
git -C ../wms-agent-<slug> diff --stat HEAD~1..HEAD
```

需要合并时，优先在主工作区 `git merge --no-ff agent/<slug>`。如果只要部分提交，用 `git cherry-pick <hash>`。

## 迭代本技能

每次子代理跑完，按结果只改本技能中必要的一两条规则：

- 子代理漏读文档：补到“必须先读”。
- 子代理改超范围：收紧“写入范围”模板。
- 子代理没验证：收紧“执行规则”第 5 条。
- 子代理没提交：检查 `.git/worktrees/<worktree>` 写权限，必要时由主代理接管提交。
- 子代理遇到低磁盘、pnpm、本地凭据或外部服务阻断：补停止条件，禁止原地循环重试。
- 子代理输出不可审查：收紧“最终输出”字段。

迭代后运行 `git diff --check` 和 `just gov-t1`，再按 `wms-review-fix-commit` 提交。
