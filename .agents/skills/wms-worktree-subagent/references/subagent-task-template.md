# 子代理任务提示词模板

```text
你是 WMS 子代理。

目标：<一句话目标>
执行模式：<write-worktree | read-only-worktree | read-only-current-diff>
模型：<model>
工作区：<绝对路径>
可见快照：<HEAD 提交，或“主工作区当前未提交差异；运行期间冻结”>

模式约束：
- `write-worktree`：只在独立 worktree 的授权范围写入。
- `read-only-worktree`：只审查已提交基线，不修改文件。
- `read-only-current-diff`：只审查主工作区当前差异，不修改文件、不建分支、不合并。

完整性边界（仅 `write-worktree`）：
- 完整模块需要覆盖：<用户故事/RTM/DB/后端/API/OpenAPI/api-client/PC前端/PDA/E2E截图/治理脚本/runbook 中相关项>
- 本轮切片覆盖：<本轮实际要做的层>
- 本轮明确不覆盖：<不做的层及原因>
- 禁止结论：写模式如果还有未覆盖层，最终只能写“本切片可合并”，不能写“模块已完成”。

写入范围：
- 允许：<目录/文件>
- 禁止：主工作区、main 分支、推送、远端 PR、真实凭据、生产数据、未授权模块

切片预算：
- 预计修改文件：<= <数量> 个；预计新增/删除总行数：<= <数量> 行；预计新增行为：<1-3 个可测试行为>
- 超出预算、需要手工补生成文件、或发现本轮其实需要 PC 前端/E2E/治理脚本时，停止并输出“不可合并”，不要继续扩大范围。

必须先读：
- AGENTS.md
- 相关 */AGENTS.override.md
- docs/requirements-traceability-matrix.md
- docs/frontend-coding-standards.md
- .agents/skills/wms-worktree-subagent/references/module-slice-boundary.md
- 任务相关用户故事、设计文档、ADR 或 runbook
- 任务相关 PlantUML 图文；PlantUML 只作为设计辅助输入，事实源仍以用户故事、ADR、RTM、OpenAPI、数据库文档和代码为准

执行规则：
1. 使用 wms-loop-engineering 定义目标、输入、检查、反馈和停止条件。
2. 复用现有前后端模块、组件、API client、测试夹具和治理脚本。
3. 先按 module-slice-boundary.md 补覆盖矩阵再改代码；不相关项写“不适用”和原因。
4. 只做本任务最小闭环；新增字段、状态、角色、模块或业务默认值时停止并说明需要主代理/用户确认。
5. 检索限制范围，优先 `rg`；长文档只读相关章节。
6. 禁止打印完整 diff/patch 正文；过程审查只允许 `git diff --stat`、`git diff --check`、`git diff --name-only`，或用 `sed -n`/`rg -n` 输出必要的局部代码片段。必须修改文件时用小补丁，单次 `apply_patch` 尽量控制在 120 行内；若工具自动回显补丁，在最终报告标记为输出噪声，不得追加二次 diff。
7. API 变更必须同步 OpenAPI 和 api-client，并运行 `just openapi-sync`、`just openapi-check`；如果生成器、pnpm、corepack 或网络失败，允许记录失败原因，但不得把手写 `shared/openapi/openapi.json` 或 `packages/api-client/src/schema.ts` 标为可合并证据。
8. Rust 命令在 `backend/` 下运行，或使用 `cargo --manifest-path backend/Cargo.toml ...`。
9. 重编译、截图或大验证前先跑 `df -h . /tmp`；可用空间不足 2GiB 时停止重型命令。
10. 真实 PostgreSQL / `sqlx::test` 相关 Rust 测试缺本 worktree `.env` 时，统一用 `set -a; source /home/test1/workspace/wms/.env; set +a; <命令>` 包裹；禁止打印连接串。
11. 真实前端任务必须明确截图路径、视口、是否上传 Gitea 附件；子代理不得占用 9002。
12. 管理端页面任务必须先输出页面设计契约：页面类型、主信息载体、标准动作入口、私有动作入口、详情展示方式和禁止常驻区域；列表型页面不得常驻轨迹、审计、明细、当前处理对象、节点状态或动作表单。
13. 管理端页面任务必须运行 `python3 scripts/governance/check_admin_page_design_contract.py --json`；失败时先修页面或写“不可合并”，不得只登记矩阵。
14. 非平凡逻辑留下最小测试。
15. 实现与 PlantUML/docu 不一致时，能确定图文过期才同步；否则停止确认。
16. `pnpm`、OpenAPI 生成器、数据库连接、`.env`、网络或外部服务类必需验证失败时，最多做一次明确修复/重试；第二次仍失败写“不可合并”，停止执行，禁止临时包装命令绕过、手写生成物或继续压 diff。
17. 运行 `git diff --check`、`just gov-t1` 和任务相关测试；任一失败时写“不可合并”。
18. `write-worktree` 按 wms-review-fix-commit 的检查项做 review→修复→review，但不 `git add` / `git commit`；只读模式只输出按严重度排序的发现，不修复。仍然禁止推送或合并主分支。
19. 提交 scope 必须使用当前治理白名单允许值；模块名不在白名单时按变更层选择既有 scope，例如后端 API / OpenAPI 用 `接口`，管理端页面用 `管理端`，治理脚本用 `治理`。
20. 不推送，不创建远端 PR，不运行 `git clean -f`、`git reset --hard` 或删除分支。
21. 最终报告只贴摘要、状态、验证和 `git diff --stat`；禁止粘贴大段 patch/diff。需要审查 diff 时交给主代理运行 `git diff`。

最终输出（仅 `write-worktree`）：
- 执行模式、模型和可见快照
- 子 worktree 路径
- 提交哈希；子代理默认写 `无：主代理提交`
- 完整性结论：从 `模块完成`、`本切片可合并`、`不可合并` 三选一
- 覆盖矩阵：逐项列“需要/本轮覆盖/证据/缺口”
- 修改文件
- `git status --short` 输出；无输出也写“干净”
- `git diff --stat` 摘要
- 是否超出切片预算；超出时必须写“不可合并”
- 未跟踪/忽略产物摘要
- 已读取的 PlantUML/docu 文件；如未读取，说明原因
- 数据库验证环境来源：`当前环境`、`主工作区 .env` 或 `缺失`
- 验证命令和退出码
- 是否需要主代理重启并校验 9002
- 是否可合并
- 剩余问题/需要确认事项
- 清理建议：`可普通移除`、`已合并待清理`、`不可合并需保留` 或 `只读审查可移除`
```

只读模式输出契约：

- 只读模式统一写 `审查完成` 或 `审查阻断`，并列模式、模型、可见快照、读写范围、依赖、输出文件、进程退出码和验证命令。
- `read-only-worktree`：再输出下一轮切片、允许文件、停止条件和技能缺口。
- `read-only-current-diff`：再输出按 P0-P2 排序的发现、路径与行号、证据、最小修复；无发现时明确写“未发现 P0-P2”。
- 两种只读模式都不输出模块完成、切片合并、提交哈希或合并判断；清理建议只针对输出文件或可丢弃 worktree。
