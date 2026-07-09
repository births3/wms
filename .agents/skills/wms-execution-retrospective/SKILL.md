---
name: wms-execution-retrospective
description: WMS 执行复盘和流程迭代技能。用户问为什么没检查出来、流程没闭环，或发现 issue-agent、截图证据、PR/tmux/worktree、脚本、skill、runbook、prompt、验证、清理收口缺口时使用；先补事故，再固化最小可复用规则。
---

# WMS Execution Retrospective

把执行漏项变成可复用规则：先补事故，再最小改动固化到脚本、skill、runbook 或 AGENTS。

## 先读

- `AGENTS.md`
- 本次任务实际使用的 `.agents/skills/*/SKILL.md`
- 相关 runbook、脚本和 issue / PR 评论
- 当前失败证据：tmux 日志、agent prompt、截图、PR、worktree 状态或验证输出
- 当前 issue / PR 评论和修复提交

## 闭环步骤

1. 复现漏项：用最小命令证明问题存在，例如查看 prompt、issue 评论、PR 评论、tmux 日志或 `git status`。
2. 从 issue 学习共性：当问题来自 issue 或评论时，先抽取“用户实际抱怨的现象、涉及页面/模块、字段/按钮/流程/证据关键词、截图指向、已有确认结论”，再判断它是单点缺陷还是共性缺口。
3. 定位断点：只选一个主因，归到以下一类：
   - 输入缺失：issue 评论、截图、附件、用户确认没有进入 prompt。
   - 执行缺失：agent 没按 prompt 执行，或 prompt 没写硬约束。
   - 证据缺失：截图、重启结果、版本校验、测试结果没有回写到 PR / issue。
   - 收口缺失：PR、tmux、worktree、agent 分支没有进入明确状态。
   - 字段共性缺失：创建时间、更新时间、创建人、货主、状态、单据类型等公共字段在某页面、接口、矩阵或脚本中缺失。
   - 范围缺口缺失：用户故事已有，但矩阵、页面、按钮、弹窗、API 或后端没有进入执行范围。
   - 前端闭环缺失：矩阵声明了 `frontend_pages`，但 `menuSections`、`defaultMenuTree`、`renderAdminView`、已发布菜单 dev mock、页面查询 / 表格自检或 E2E 证据没有同步检查。
   - 闭环语义缺失：用户要求“缺口闭环 / 补齐 / 验收”，执行却只登记矩阵或写延期，没有推动未实现功能、测试和证据。
   - 验证缺失：现有脚本没覆盖，或 dry-run 产物污染治理检查。
4. 先补当前事故：能补评论、截图附件、重启状态或清理状态的，先补；不能补的写明阻塞。
5. 固化规则：选择最小落点：
   - prompt 生成缺口：改对应脚本，并加自测。
   - 子代理执行缺口：改 `wms-worktree-subagent`。
   - issue agent 流程缺口：改 `docs/runbooks/gitea-issue-agent.md`。
   - 字段共性缺口：先补字段矩阵 / 设计 RTM，再补前后端映射和治理脚本。
   - 范围缺口：先补 `governance/quality-matrix.toml` 或 `scripts/governance/check_scope_gap_discovery.py`，再补实现；模块验收用 `--strict --module <模块>`。
   - 前端闭环缺口：补 `check_scope_gap_discovery.py`、页面级 self-check、`e2e_checks` 或真实 Playwright E2E，再复跑模块严格检查。
   - 闭环语义缺口：改触发技能的闭环定义；没有用户确认时，`deferred_stories` 只能标“待确认”，不能作为完成。
   - 全局协作习惯：改 `AGENTS.md`。
   - 单次口头提醒：不要建规则，直接说明。
6. 验证规则真的生效：
   - 脚本改动：跑编译 / self-test / dry-run，检查生成内容含新规则。
   - 文档或技能改动：跑 `git diff --check` 和 `just gov-t1`。
   - 常驻进程改动：重启对应 tmux watcher，并确认新进程运行。
7. 输出复盘：只写主因、已补事故、已固化规则、验证结果、剩余未闭环项。

## 近邻排除

以下情况不升级为本技能的流程迭代，只修当前问题并记录验证：

- 单次 UI、文案、样式或字段展示缺陷，确认没有跨页面、跨模块、跨脚本复发风险。
- 普通功能开发、需求设计、架构讨论或方案选择，尚未出现执行漏项、证据漏项、收口漏项或共性治理缺口。
- 问题只需要当前代码修复，不需要改 prompt、runbook、skill、矩阵、治理脚本或 AGENTS。

## Issue 共性学习

从 issue 或 PR 评论触发复盘时，必须先做一次“共性学习”，避免只修当前报错。

提取信号：页面 / 模块、业务对象、缺失类型、公共关键词、证据位置。

判断为共性：影响两个以上页面 / 模块 / 对象；用户说“这些、各个、统一、为什么没检查出来”；修复跨矩阵、契约、后端、前端、测试、治理脚本两层以上；现有脚本不能在改动前报错；用户故事或菜单页已存在但矩阵未登记。

共性学习输出：

- `现象`：用户在 issue / 评论中实际指出什么。
- `共性`：它归属于哪类可复发问题。
- `断点`：缺在矩阵、契约、实现、验证、证据还是收口。
- `规则落点`：补哪个 skill、runbook、脚本、矩阵或 AGENTS 规则。
- `治理验证`：新增或扩展哪个命令，下一次同类缺失应能失败。

范围类问题的固定验证：

- 默认发现：`python3 scripts/governance/check_scope_gap_discovery.py --json`
- 模块闭环：`python3 scripts/governance/check_scope_gap_discovery.py --strict --module <模块>`
- 只有当前事故修了、规则落到矩阵或脚本、严格模式能暴露同类缺口，才算自我迭代完成。
- “缺口闭环”默认是功能闭环；若未实现功能只是被登记或延期，必须回到执行技能拆开发任务。

## 共性字段分析

遇到“缺少创建日期 / 创建时间 / created_at”等字段问题时，先判断共性。

按链路检查：需求/矩阵 → OpenAPI/DTO/repository → `apps/web-admin/src/features/**` 映射 → `DataGrid` 列与筛选 → `QueryPanel` 查询 → T1 治理脚本。

公共字段包括 `created_at`、`updated_at`、`created_by`、`updated_by`、`owner_id`、`status`、`document_type`。输出必须写断点层、本次实例、已新增或扩展的防复发脚本。

## 外部 Skill 引入

引入外部 skill 时，先临时目录只读审计脚本、hooks、网络、提交/推送、secret 和跳过确认风险；只装 skill 本体。安装后跑 yao 结构和资源边界检查，入口过重或触发过宽时先收窄。

## 判断标准

- 当前事故没有补证据，不算完成。
- 规则没有进入可复用文件，不算迭代。
- 规则没有验证命令，不算生效。
- PR 只创建未合并、tmux 仍运行、worktree 保留时，必须写清状态和下一步。
- 截图只写本地路径不算证据回写；必须上传附件或说明为什么无法上传。

## 禁止事项

- 不为了一个偶发问题新增复杂状态机。
- 不把主因写成多个泛泛原因。
- 不用 `git clean -f`、`git reset --hard` 或强删 worktree 掩盖流程问题。
- 不提交真实 token、密钥、生产数据或截图里的敏感信息。
