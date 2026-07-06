---
name: wms-quality-matrix-governance
description: WMS 全链路质量矩阵治理技能。用户要求建立或维护测试/质量矩阵、检查新增用户故事/页面/API/字段是否进入矩阵、按 S0-S3 分层策略补齐维度、根据 issue/Bug/review 漏检迭代检查维度、修复 check_quality_matrix 失败，或说“缺口闭环 <模块/页面/issue>”时使用。
---

# WMS Quality Matrix Governance

把 WMS 用户故事、页面、接口、字段、后端、数据库、权限、审计、测试、证据和治理脚本纳入同一张可检查矩阵。

## 先读

- `AGENTS.md`
- `docs/governance/quality-matrix-method.md`
- `.agents/skills/wms-loop-engineering/SKILL.md`
- `.agents/skills/wms-execution-retrospective/SKILL.md`
- `docs/agent-loop-engineering.md`
- `governance/quality-matrix.toml`
- 目标故事文件、页面文件、OpenAPI path 或后端模块

## 固定事实源

- 机器事实源：`governance/quality-matrix.toml`。
- 展示页：`docs/governance/quality-matrix.md`，只由脚本生成。
- 检查脚本：`scripts/governance/check_quality_matrix.py`。
- 范围缺口：`scripts/governance/check_scope_gap_discovery.py`；模块验收用 `--strict --module <模块>`。
- MkDocs 入口：`mkdocs.yml` 治理分组。

## 工作流

短触发语：`缺口闭环 <模块/页面/issue>`。等价于先严格发现范围缺口，再用 `wms-worktree-subagent` 补实现，随后用 `wms-review-fix-commit` 复审提交；若发现漏检，用 `wms-execution-retrospective` 反哺规则。

0. 新增维度、漏检复盘、批量补齐或多轮修复时，先按 `wms-loop-engineering` 定义目标、检查和停止条件。
1. 先运行 `python3 scripts/governance/check_quality_matrix.py --json`，确认当前矩阵是否干净。
2. 根据任务判断是否需要新增或修改矩阵行：
   - 新增 / 修改用户故事。
   - 新增 / 修改管理端页面、PDA 页面、OpenAPI path、数据库字段、权限、审计或测试。
   - issue、Bug、review 暴露出前后端未对齐、字段遗漏、页面布局遗漏、脚本漏检。
3. 每个矩阵条目以用户故事 ID 为主键；首版强门禁范围为 M1/M2/M3/M4。
4. 每个故事必须写完整维度：`requirement`、`fields`、`frontend`、`api`、`backend`、`database`、`security`、`audit`、`tests`、`evidence`、`docs`、`governance`。
5. 维度状态只允许 `verified` 或 `not_applicable`；`not_applicable` 必须写原因，不能用来掩盖未完成。
6. 由 `types` 自动推导测试层；不要手工压低 L1-L11 覆盖要求。
7. 改完事实源后运行 `python3 scripts/governance/check_quality_matrix.py --write-doc` 生成展示页。
8. 跑范围缺口：日常 `python3 scripts/governance/check_scope_gap_discovery.py --json`；模块验收或用户问“是否补齐”时加 `--strict --module <模块>`。
9. 严格模式发现缺口时，必须补矩阵、故事范围、页面/API/后端实现，或写入经用户确认的延期范围。
10. 验证：`check_quality_matrix.py --json`、`check_scope_gap_discovery.py --json`、相关 pytest、接线改动跑 dispatch/smoke、最后 `just gov-t1`。

## 页面分类分区联动

新增管理端页面时，同时使用 `wms-page-query-governance`：

- 列表型：页面上方 `QueryPanel`，主体 `DataGrid`。
- 双栏目录型：左侧分类 / 目录 / 树，右侧明细列表或编辑区。
- 配置型：按配置域分区，展示影响范围、启停状态和审计入口。
- 详情弹窗型：按订单信息、商品信息、批号信息、流程信息、审计信息上下分区。

页面分类必须反映到矩阵的 `frontend`、`fields`、`evidence` 和 `governance` 维度。

## 标准线与自我发现

质量标准线分两级：

- 硬失败：已进入质量矩阵的故事、页面或 API 与真实菜单 / 文档 / 接线不一致。
- 发现型缺口：同一活跃模块中还有未登记故事、未覆盖菜单页、用户故事要求的按钮 / 弹窗 / 流程未进入矩阵。

发现型缺口不能在最终汇报中消失；“补齐/闭环/验收/所有模块”任务必须升级为严格模式。

## 运行反馈迭代

出现三页以上同类遗漏、前端有页面但后端/API/权限未跟上、issue/review 暴露脚本漏检、新业务概念影响多模块时，先补矩阵或脚本，再批量修实现。顺序：定义目标 → 补事实源 → 补脚本 → 跑失败 → 修实现 → 生成展示页 → 复跑治理 → 记录漏检规则。

## 自进化

当 `check_quality_matrix.py`、页面自检、真实后端、dev mock、E2E、issue 评论或 review 暴露出“矩阵通过但运行仍失败”时，必须触发 `wms-execution-retrospective`，并把漏检转成下次可复用的检查规则。

最小闭环：归因到矩阵、脚本、页面/API 自检、真实后端、dev mock、证据或 skill 规则；先修当前事故；规则不足就改本 `SKILL.md`；可脚本化就补脚本/self-check，不可脚本化就写入方法文档或 runbook。复跑 `check_quality_matrix.py --json`、`check_scope_gap_discovery.py --json`、新增检查、`git diff --check`、`just gov-t1`。

前端页面登记为 `frontend=verified` 时，还要确认页面数据入口可运行：页面调用的 API 必须在真实后端路由或 `apps/web-admin/vite.config.ts` dev mock 路由中至少一边可达；dev mock 模式下要有 self-check 或 `curl 9002` 证据，防止 `Dev mock route not found`。
