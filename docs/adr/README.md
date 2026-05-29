# ADR 索引（Architecture Decision Records）

本目录存放 wms 项目的架构决策记录。

> 详细规则：见 `../governance.md` §3.5 ADR 制度

## 目录约定

- 文件名格式：`NNNN-<slug>.md`（4 位顺序号 + 短横线 + slug）
- 编号永不复用；删除的 ADR 仍占用编号
- 状态：`Proposed` / `Accepted` / `Deprecated` / `Superseded by ADR-XXXX`
- 必填段：背景 / 候选方案 / 决策 / 后果 / 参考

## 已 Accepted

| 编号 | 标题 | 状态 | 摘要 |
|------|------|------|------|
| [ADR-0001](0001-tech-stack.md) | 技术栈选型 | Accepted | Rust+Axum / SQLx / PostgreSQL / Vite+React+shadcn/ui / Zustand / TanStack Query / RN PDA / OpenAPI 契约 |
| [ADR-0002](0002-monorepo-structure.md) | 仓库结构 | Accepted | monorepo + Cargo workspace + pnpm workspace；依赖红线 domain ⊥ infra |
| [ADR-0003](0003-governance-model.md) | 治理模型 | Accepted | 5 类 + 4 Tier + Baseline + diff 触发；与 ADR-0006 集成 |
| [ADR-0006](0006-tdd-and-test-layers.md) | TDD + 11 层测试维度 | Accepted | outside-in 双层 TDD；L1-L11 测试维度；Tier×Layer 执行矩阵 |
| [ADR-0007](0007-roadmap-v03-boundary-alignment.md) | 波次路线 v0.3 边界对齐 | Accepted | M11 移除；码上放心归 M-TC；药监 EDI 由 ERP/H8 边界承接；当前路线入口 |
| [ADR-0008](0008-borrow-from-odoo.md) | 借鉴 Odoo 的 9 个设计 | Accepted | mail.thread/stock.move/ir.sequence/manifest/GS1 + ir.rule/access.csv/TransientModel/state button；分散到 Wave 0-4 落地，约 2.5 个月增量 |
| [ADR-0010](0010-error-codes.md) | 错误码体系与字典 | Accepted | 三段式 `<MODULE>_<CATEGORY>_<DETAIL>` + 4 级严重度 + 50 错误码字典 + check_error_codes.py 治理 |
| [ADR-0011](0011-observability.md) | 可观测性方案 | Accepted | OpenTelemetry + Prometheus + Loki + Grafana 四件套 + KPI 命名约定 + SLO 告警 + check_observability.py 治理 |
| [ADR-0012](0012-bounded-contexts.md) | 限界上下文与 Context Map | Accepted | 24 个 BC 显式声明 + 8 种 DDD 集成模式 + 9 类 Shared Kernel + check_bounded_contexts.py 治理 |
| [ADR-0013](0013-config-secrets.md) | 配置与 secrets 管理 | Accepted | 三层配置（编译/环境/运行时）+ Vault/k8s Secret + 90 天密钥轮换 + check_secrets.py 治理 |
| [ADR-0014](0014-data-migration.md) | 数据迁移策略（legacy → wms）| Accepted | Debezium CDC + 双写 + 货主级灰度 + 4 维校验 + 30 分钟回滚 RTO |
| [ADR-0015](0015-multi-end-rules.md) | 多端业务规则放置 | Accepted | A/B/C 三级规则分类 + OpenAPI schema 单一事实之源 + PDA 离线扩展 + check_multi_end_consistency.py 治理 |
| [ADR-0016](0016-deployment.md) | 部署形态（Docker / k8s）| Accepted | docker-compose（小型 3PL）+ Kubernetes（大型连锁）双轨 + Dockerfile 模板 + Migration 4 步走 |

## 已取代

| 编号 | 标题 | 状态 | 取代方 |
|------|------|------|--------|
| [ADR-0004](0004-phase-roadmap.md) | 波次路线（v0.2 取代 v0.1 三阶段） | Superseded by ADR-0007 | [ADR-0007](0007-roadmap-v03-boundary-alignment.md) |

## 编号空缺与未来计划

| 编号 | 状态 | 说明 |
|------|------|------|
| ADR-0005 | 预留 | Git Worktree 工作流（命名 / target/ / node_modules / baseline 协调）。计划 Wave 1 启动前补写。 |
| ADR-0009 | 未分配 | 已被合并到 ADR-0010（错误码体系含 API 错误响应规范）+ coding-standards §3.5 已修复 URL 版本统一。如未来 API 版本策略有更复杂决议（如 GraphQL/gRPC 多协议）再单独补 |
| ADR-0017 | 未分配 | 预留 i18n（启动条件见 ROADMAP.md §国际化 backlog） |
| [ADR-0018](0018-resilience-engineering.md) | 弹性工程 | Accepted | 6 大能力统一方案：幂等分级 / 重试 4 级（L0-L3）/ 限流 3 维度 / 熔断状态机 / 降级 4 级（D0-D3）/ 死信处理流程 + DLQ 表；继承 coding-standards §3.3 + ADR-0006 L11 + technical-specs §H8 + clarifications #36 |
| [ADR-0019](0019-tenant-custom-fields.md) | 货主自定义属性 | Accepted | 简化版借鉴 Odoo Studio：JSONB 列 + 配置中心 schema + `x_` 前缀命名 + 6 聚合根白名单 + Rust 运行时校验；ADR-0008 第 10 项；Wave 5 实施 |
| [ADR-0021](0021-high-fidelity-prototype-strategy.md) | 高保真原型策略 | Accepted | Storybook + shadcn/ui 真组件；Wave N 前必须有原型 + 走查；5 个治理脚本 |
| [ADR-0022](0022-prototype-component-spec.md) | 原型组件规范 | Accepted | 三层架构（ui/business/pages）+ cva variants + tailwind + forwardRef + 文档头规范 + 4 治理脚本规划 |
| [ADR-0023](0023-business-report-strategy.md) | 业务报表方案选型 | Accepted | 混合方案：A. GSP 法定后端实现 + B. 业务报表 Metabase 嵌入（Wave 5）+ C. 业务页快捷入口 + D. 订阅 WMS 自实现 |
| [ADR-0028](0028-component-library-extraction.md) | 组件库抽离至 packages/ui | Accepted | 16 业务组件 + 9 shadcn primitive + 设计 token + cn 工具从 prototypes/src/components 抽到 packages/ui/src，构成 @wms/ui 共享包；prototypes 与未来 apps/web-admin 共用；commit e3ce5a0 实施 |
| [ADR-0024](0024-auth-model.md) | 鉴权模型（JWT+AuthContext+多租户）| Accepted v0.2 | SPIKE-001 验证 accept；JWT claims (sub/iat/exp/jti/owner_id/user_name/permissions) + 双 token (access 1h / refresh 24h) + Redis blacklist + 混合失效模式 (permissions_changed_at) + 故障降级 + handler 模板（ctx: AuthContext 自动注入 owner_id 过滤）+ AUTH-001..009 错误码 |
| [ADR-0025](0025-audit-storage-model.md) | 审计存储模型（PG append-only）| Accepted v0.2 | SPIKE-002 验证 accept；audit_event 表 + 月分区 (PARTITION BY RANGE) + trigger + wms_app 角色权限 + JSONB diff (jsonb_path_ops 索引) + 每日封档 hash chain (方案 C) + spike-002b 并发 fallback + 5 年保留与归档 |
| [ADR-0026](0026-cross-end-contract-pipeline.md) | 跨端契约管线 | Accepted | utoipa→OpenAPI→openapi-typescript→openapi-fetch 全链路；SPIKE-003 验证 accept；Wave 1 W1.C 实施 checklist；含数据类型映射表 + utoipa 编码约束 + CI sync 治理 |
| ADR-0027 | Spike 拟产出 | 待写 | 0027 PDA 离线（SPIKE-005，可推迟到 Wave 3 启动前）|
| [ADR-0029](0029-frontend-as-prototype-workflow.md) | 前端原型先行工作流 | Accepted | 前端高保真原型先行，业务走查后经 checklist 迁移生产；原型页不得直接复制为生产页，`apps/web-admin` Wave 1 只启动 H1/H2/H3 壳工程 |
| [ADR-0030](0030-integration-capability.md) | H-INT 统一外部集成能力 | Accepted | 外部对接（H8/H5/M5/M10/M-TC/H4）确立统一接入契约（复用 ADR-0018 弹性 + M-PM 规整 + ADR-0013 凭证 + H2 审计）；契约先行随 Wave 1、引擎延后；H8 首个参考实现 |
| [ADR-0031](0031-file-attachment-capability.md) | H-FILE 统一附件/文件能力 | Accepted | 将 infra/file-storage.md（MinIO/S3 + attachments 表 + presigned URL）提升登记为横向能力 H-FILE + 接入契约；复核 15 命中→9 真需求模块；成本最低 |
| [ADR-0032](0032-approval-engine.md) | H-APV 审批引擎（契约先行） | Accepted | 审批编排确立横向能力（复用 M3-003 approval_source 锚点 + H4 通道 + H6 状态机）；复核 27 命中→22 真需求（多走 M-QL→企微审批→回写链路）；引擎延后 |
| [ADR-0033](0033-scheduler-engine.md) | H-SCH 调度引擎（契约先行） | Accepted | 定时/周期触发统一注册（复用 ADR-0018 重试 + H-AL 告警 + H2 审计）；复核 25 命中→14 真定时需求；不接管 M-TE 作业调度/H10 备份；引擎延后，优先级最低 |
| ADR-0034+ | 未分配 | 后续按需新建 |

> 编号永不复用。空缺编号必须在此表登记原因。

## 模板

新建 ADR 时使用本模板（保存为 `NNNN-<slug>.md`）：

```markdown
# ADR-NNNN: <决策标题>

- 状态：Proposed
- 日期：YYYY-MM-DD
- 决策者：<人名>
- 关联：<引用相关 ADR、文档>

## 背景
（为什么要做这个决策；当时面临的问题/约束）

## 候选方案
1. 方案 A — 优点 / 缺点
2. 方案 B — 优点 / 缺点

## 决策
（选了哪个，为什么）

## 后果
- 正面：
- 负面：
- 风险：

## 实施约束
（落地时的强制规则）

## 参考
- ...
```

## 强制 ADR 的场景

下列变更**必须**走 ADR：

- 引入新框架 / 中间件 / 数据库 / 第三方服务
- 改变分层架构或限界上下文边界
- 跨上下文集成方式（同步调用 / 事件 / 编排）
- 安全 / 合规相关决策（审计、加密、监管对接）
- 不可逆决策（数据迁移、协议选定）

## 治理校验

`scripts/governance/validate_adr_index.py` 会自动检查：
- 文件名格式
- 编号唯一性
- 状态合法性
- 必填段完整性
- 本索引文件是否登记所有 ADR
