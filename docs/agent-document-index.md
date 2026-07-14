# AI 常用文档索引

> 本索引承接根 `AGENTS.md` 下沉的长清单。根文件只保留生成前必须知道的约束；业务、架构、治理和运行背景按需从这里继续查。

## 业务文档

| 文档 | 用途 |
|---|---|
| [domain/user-stories-m1-master-data-product.md](domain/user-stories-m1-master-data-product.md) | M1 基础档案：商品资料 |
| [domain/user-stories-m1-master-data-warehouse.md](domain/user-stories-m1-master-data-warehouse.md) | M1 基础档案：仓库资料 |
| [domain/user-stories-m2-inbound-asn.md](domain/user-stories-m2-inbound-asn.md) | M2 入库：到货通知 |
| [domain/user-stories-m2-inbound-verify.md](domain/user-stories-m2-inbound-verify.md) | M2 入库：验收入库 |
| [domain/user-stories-m3-inventory-query.md](domain/user-stories-m3-inventory-query.md) | M3 库存：查询和可视化 |
| [domain/user-stories-m3-inventory-operation.md](domain/user-stories-m3-inventory-operation.md) | M3 库存：操作和调整入口 |
| [domain/user-stories-m4-outbound-order.md](domain/user-stories-m4-outbound-order.md) | M4 出库：订单 |
| [domain/user-stories-m4-outbound-pick.md](domain/user-stories-m4-outbound-pick.md) | M4 出库：拣货 |
| [domain/user-stories-m4-outbound-return.md](domain/user-stories-m4-outbound-return.md) | M4 出库：退货 |
| [domain/user-stories-m5-cold-chain.md](domain/user-stories-m5-cold-chain.md) | M5 冷链数据集成 |
| [domain/user-stories-m6-audit-report.md](domain/user-stories-m6-audit-report.md) | M6 报表 |
| [domain/user-stories-m8-retail-chain.md](domain/user-stories-m8-retail-chain.md) | M8 连锁 |
| [domain/user-stories-m9-billing.md](domain/user-stories-m9-billing.md) | M9 计费 |
| [domain/user-stories-m10-tms-plus.md](domain/user-stories-m10-tms-plus.md) | M10 运输管理增强 |
| [domain/user-stories-m11-regulatory-edi.md](domain/user-stories-m11-regulatory-edi.md) | M11 监管 EDI 历史边界 |
| [domain/user-stories-mte-task-engine.md](domain/user-stories-mte-task-engine.md) | M-TE 任务引擎 |
| [domain/user-stories-mrp-replenishment.md](domain/user-stories-mrp-replenishment.md) | M-RP 补货 |
| [domain/user-stories-mpk-packing-station.md](domain/user-stories-mpk-packing-station.md) | M-PK 包装站 |
| [domain/user-stories-mvr-validation-rules.md](domain/user-stories-mvr-validation-rules.md) | M-VR 规则引擎 |
| [domain/user-stories-mtc-traceability-code.md](domain/user-stories-mtc-traceability-code.md) | M-TC 追溯码 |
| [domain/user-stories-mql-quality-liaison.md](domain/user-stories-mql-quality-liaison.md) | M-QL 质量联系单 |
| [domain/user-stories-mcg-code-generator.md](domain/user-stories-mcg-code-generator.md) | M-CG 编码生成 |
| [domain/user-stories-msa-stock-adjustment.md](domain/user-stories-msa-stock-adjustment.md) | M-SA 报损报溢 |
| [domain/user-stories-mba-batch-adjustment.md](domain/user-stories-mba-batch-adjustment.md) | M-BA 批号调整 |
| [domain/user-stories-mrc-reconciliation.md](domain/user-stories-mrc-reconciliation.md) | M-RC 对账 |
| [domain/user-stories-mdi-drug-inspection.md](domain/user-stories-mdi-drug-inspection.md) | M-DI 药检单 |
| [domain/user-stories-mpm-parameter-mapping.md](domain/user-stories-mpm-parameter-mapping.md) | M-PM 参数映射 |
| [domain/user-stories-h1-auth-tenant.md](domain/user-stories-h1-auth-tenant.md) | H1 权限与多租户 |
| [h1-006-api-key-lifecycle-slice.md](h1-006-api-key-lifecycle-slice.md) | H1-006 API Key 生命周期实现与验收切片 |
| [h1-006-api-key-lifecycle-web-design-plan.md](h1-006-api-key-lifecycle-web-design-plan.md) | H1-006 管理端页面字段、动作与证据设计 |
| [domain/user-stories-h2-audit-trail.md](domain/user-stories-h2-audit-trail.md) | H2 审计追踪与事件总线 |
| [domain/user-stories-h3-contract.md](domain/user-stories-h3-contract.md) | H3 跨端契约 |
| [domain/user-stories-h4-wechat-notify.md](domain/user-stories-h4-wechat-notify.md) | H4 企业微信 |
| [domain/user-stories-h5-express.md](domain/user-stories-h5-express.md) | H5 快递 |
| [domain/user-stories-h9-print-template.md](domain/user-stories-h9-print-template.md) | H9 打印模板引擎 |
| [domain/user-stories-h-driver.md](domain/user-stories-h-driver.md) | H-Driver 司机端 |
| [domain/user-stories-h-store.md](domain/user-stories-h-store.md) | H-Store 门店用户端 |
| [domain/user-stories-h-dock-management.md](domain/user-stories-h-dock-management.md) | H-DOCK 月台预约管理 |
| [domain/user-stories-h-alert.md](domain/user-stories-h-alert.md) | H-AL 告警引擎 |
| [domain/clarifications.md](domain/clarifications.md) | 业务澄清记录 |
| [domain/todo-legacy-comparison.md](domain/todo-legacy-comparison.md) | 旧需求对照 |

## 架构和治理文档

| 文档 | 用途 |
|---|---|
| [agent-collaboration.md](agent-collaboration.md) | AI 协作细则、确认流程和版本控制协作规则 |
| [agent-commit-rules.md](agent-commit-rules.md) | AI 默认本地提交条件、禁止项和后续迭代规则 |
| [agent-loop-engineering.md](agent-loop-engineering.md) | AI 闭环执行规范，定义目标、检查、反馈和停止条件 |
| [governance.md](governance.md) | 治理体系、提交规范、分层门禁和验证分级 |
| [coding-standards.md](coding-standards.md) | 代码书写规范 |
| [frontend-coding-standards.md](frontend-coding-standards.md) | 前端编码规范 |
| [requirements-traceability-matrix.md](requirements-traceability-matrix.md) | 项目级 RTM 标准，覆盖故事、前端、后端、测试和合规风险矩阵 |
| [h1-menu-management-design.md](h1-menu-management-design.md) | H1 PC 三层菜单管理、草稿发布、版本回滚和按钮权限点设计 |
| [m2-inbound-web-design-plan.md](m2-inbound-web-design-plan.md) | M2 PC Web 收货、验收、上架三页真实截图与设计规划 |
| [m4-outbound-web-design-plan.md](m4-outbound-web-design-plan.md) | M4 PC Web 出库订单、波次、复核发货、退货页面设计规划 |
| [layered-design.md](layered-design.md) | 前后端分层设计规范 |
| [architecture-dependencies.md](architecture-dependencies.md) | 模块依赖图和波次依赖 |
| [infra/technical-specs.md](infra/technical-specs.md) | 基础设施技术规格 |
| [database/database-design-standards.md](database/database-design-standards.md) | PostgreSQL 表、字段、索引、约束和 migration 命名规范 |
| [database/table-catalog.md](database/table-catalog.md) | 从 migrations 生成的数据库表、字段、索引目录 |
| [database/database-design-review.md](database/database-design-review.md) | 基于当前数据库表的设计审查矩阵 |
| [concept-audit.md](concept-audit.md) | 概念审计报告 |
| [glossary.md](glossary.md) | 术语表 |
| [adr/README.md](adr/README.md) | ADR 总索引 |
| [error-codes.md](error-codes.md) | 错误码字典 |
| [compliance/README.md](compliance/README.md) | GSP 合规追溯矩阵总索引 |
| [compliance/gsp-field-traceability.md](compliance/gsp-field-traceability.md) | GSP 字段追溯矩阵 |
| [compliance/gsp-business-rules-registry.md](compliance/gsp-business-rules-registry.md) | GSP 业务规则字段注册表 |

## 原型和运行资料

| 文档 | 用途 |
|---|---|
| [prototypes/prototype-to-production.md](prototypes/prototype-to-production.md) | 原型转生产迁移清单 |
| [prototypes/matrix-e2e-screenshot-gate.md](prototypes/matrix-e2e-screenshot-gate.md) | Matrix E2E 截图门禁 |
| [prototypes/component-registry.md](prototypes/component-registry.md) | 原型组件注册表 |
| [prototypes/prototype-proof-report.md](prototypes/prototype-proof-report.md) | 原型证明报告 |
| [runbooks/wave-1-runtime-evidence.md](runbooks/wave-1-runtime-evidence.md) | Wave 1 运行证据 |
| [runbooks/gitea-issue-agent.md](runbooks/gitea-issue-agent.md) | Gitea issue 自动判断、方案确认、codex exec 执行和证据回写流程 |
| [runbooks/wave-2-runtime-evidence.md](runbooks/wave-2-runtime-evidence.md) | Wave 2 运行证据 |
| [runbooks/wave-3-pda-readiness.md](runbooks/wave-3-pda-readiness.md) | Wave 3 PDA 就绪 |
| [runbooks/wave-4-external-dependencies.md](runbooks/wave-4-external-dependencies.md) | Wave 4 外部依赖证据 |
| [runbooks/wave-5-hardware-evidence.md](runbooks/wave-5-hardware-evidence.md) | Wave 5 硬件证据 |
| [runbooks/wave-5-tms-evidence.md](runbooks/wave-5-tms-evidence.md) | Wave 5 运输管理证据 |
| [runbooks/wave-6-closeout.md](runbooks/wave-6-closeout.md) | Wave 6 收口标准 |
| [runbooks/wave-6-deploy-evidence.md](runbooks/wave-6-deploy-evidence.md) | Wave 6 部署证据 |
| [runbooks/wave-6-evidence-preflight.md](runbooks/wave-6-evidence-preflight.md) | Wave 6 证据预检 |

## 仓库根文件和治理配置

| 文档 | 用途 |
|---|---|
| [../ROADMAP.md](../ROADMAP.md) | 长期路线 |
| [../TODO.md](../TODO.md) | 当前任务 |
| [../CHANGELOG.md](../CHANGELOG.md) | 版本变更记录 |
| [../governance/gate-rules.toml](../governance/gate-rules.toml) | 门禁触发规则 |
| [../governance/baselines/README.md](../governance/baselines/README.md) | 基线机制说明 |
