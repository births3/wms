---
name: wms-quality-matrix-governance
description: WMS 全链路质量矩阵治理技能。用户要求建立或维护测试/质量矩阵、检查新增用户故事/页面/API/字段是否进入矩阵、按 S0-S3 分层策略补齐维度、根据 issue/Bug/review 漏检迭代检查维度，或修复 check_quality_matrix 失败时使用。
---

# WMS Quality Matrix Governance

用于把 WMS 用户故事、页面、接口、字段、后端、数据库、权限、审计、测试、证据和治理脚本纳入同一张可检查矩阵。

## 先读

- `AGENTS.md`
- `docs/governance/quality-matrix-method.md`
- `governance/quality-matrix.toml`
- `docs/adr/0006-tdd-and-test-layers.md`
- 目标故事文件、页面文件、OpenAPI path 或后端模块

## 固定事实源

- 机器事实源：`governance/quality-matrix.toml`。
- 展示页：`docs/governance/quality-matrix.md`，只由脚本生成。
- 检查脚本：`scripts/governance/check_quality_matrix.py`。
- MkDocs 入口：`mkdocs.yml` 治理分组。

## 工作流

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
8. 跑验证：
   - `python3 scripts/governance/check_quality_matrix.py --json`
   - `python3 -m pytest scripts/governance/tests/test_quality_matrix.py -q`
   - 涉及接线时跑 `python3 -m pytest scripts/governance/tests/test_governance_dispatch.py scripts/governance/tests/test_smoke.py -q`
   - 最后跑 `just gov-t1`

## 页面分类分区联动

新增管理端页面时，同时使用 `wms-page-query-governance`：

- 列表型：页面上方 `QueryPanel`，主体 `DataGrid`。
- 双栏目录型：左侧分类 / 目录 / 树，右侧明细列表或编辑区。
- 配置型：按配置域分区，展示影响范围、启停状态和审计入口。
- 详情弹窗型：按订单信息、商品信息、批号信息、流程信息、审计信息上下分区。

页面分类必须反映到质量矩阵的 `frontend`、`fields`、`evidence` 和 `governance` 维度。

## 运行反馈迭代

发现共性问题时，先补矩阵或脚本，再批量修实现：

- 三个以上页面出现同类遗漏。
- 前端页面已构建但后端 / OpenAPI / 数据库 / 权限未跟上。
- issue 评论、验收反馈或 review 暴露出脚本没有检查到的问题。
- 新增业务概念影响两个以上模块。

迭代顺序固定为：补事实源 → 补脚本 → 跑失败 → 修实现 → 生成展示页 → 复跑治理。
