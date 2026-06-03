# wms TODO（当前 Wave）

> 任务追踪粒度：当前 Wave 内的具体任务。
> Wave 切换时归档当前 TODO 并启动新 TODO。
> 长期路线见 [ROADMAP.md](ROADMAP.md)。

---

## 当前 Wave：Wave 3 — 核心业务规则铺开（进行中）

**目标**：M2 入库业务规则、M3 库存模型与状态规则、M5 外部冷链数据接入 schema、M9 计费账户/合同模型逐步落地。

### 已完成（Wave 3 第一批后端切片）

- [x] W3.A backend：M2 收货闭环校验、验收效期校验、双人签字、上架记录服务与单测
- [x] W3.B backend：M3 库存批次模型、入库上架增加库存、可用库存、状态机审批源约束与单测
- [x] W3.C backend：M5 外部温控数据/超标事件接入 schema 与幂等服务；保持 WMS 不采集、不判定超标的边界
- [x] W3.D backend：M9 计费账户 / 合同 / 规则模型与基础校验
- [x] W3 handler shell：Wave 3 第一批 Axum handler 组合已接入权限检查、错误响应与 H2 审计写入；覆盖 M2 workflow / M3 inventory / M5 cold-chain / M9 billing
- [x] W3 repository design：ADR-0034 已 Accepted，确认 Wave 3 第一批 PostgreSQL 表结构与事务边界
- [x] H3 同步：`shared/openapi/openapi.json` 与 `packages/api-client/src/schema.ts` 已反映 Wave 3 第一批 path/schema
- [x] 治理：`check_openapi_contract.py` 要求 Wave 3 第一批 path/schema

### 进行中 / 待做

- [ ] W3.A PDA 端：`apps/pda-mobile` 目前只有 `.gitkeep`，生产 PDA app 未启动；SPIKE-005 RN 扫枪仍需按 ROADMAP 启动条件重开
- [ ] W3.A repository / idempotency：把 M2 workflow handler 接入 PostgreSQL repository、HTTP-level 幂等键、H2 PostgreSQL 审计落库
- [ ] W3.B repository：把 M3 库存服务接入 PostgreSQL repository，补 L3-L5/L8/L11 关键路径测试
- [ ] W3.C external auth：外部冷链系统 API Key 接入、H-INT 契约与 PostgreSQL 审计落库
- [ ] W3.D 后续：M9 自动计费与账单管理仍在 Wave 5；当前只完成账户/合同/规则模型
- [ ] W3 完成门禁：M2 / M3 关键路径 11 层测试覆盖；GSP 资质有效期校验生效

### 后续跟踪（不计入 Wave 3 开发完成）

- W2.G pre-release runtime gate：有稳定 dev/staging 后，按 [Wave 2 Pre-release Runtime Evidence Runbook](docs/runbooks/wave-2-runtime-evidence.md) 验证配置中心版 Feature Flag 迁移、对账、切源、旧文件归档和 smoke，写入 `docs/retros/wave-2-runtime-evidence.json`
- W1.B / W1.D pre-release runtime gate：仍按 [Wave 1 Pre-release Runtime Evidence Runbook](docs/runbooks/wave-1-runtime-evidence.md) 在真实环境补齐
- W2-external：人工确认“码上放心”账号外部开通状态；外部依赖仍按 [ROADMAP.md](ROADMAP.md) 的外部依赖追踪表跟进

Wave 1 / Wave 2 开发完成状态仍分别以 `just wave-1-complete-check` / `just wave-2-complete-check` 为准；上述 runtime gate 在预发布前单独验证，禁止用 localhost / stub / mock / fake / example 代替。

---

## 已归档：Wave 0.5 — 原型 + 技术 Spike + 组件库抽离

**目标**：组件库骨架 + P0 原型 + 技术 Spike 验证 + Wave 1 复用准备。

### 已完成

#### 治理 / 文档（Wave 0 续）
- [x] T1 治理脚本扩到 24 个（含 component-* 4 个 + visual-* 3 个 + prototype-* 5 个）
- [x] `governance/gate-rules.toml` 路径触发表（含 prototypes/* 与 packages/ui/src/business/**）
- [x] `governance/baselines/` baseline 机制
- [x] ADR-0021..0023, 0028 起 7 份新 ADR（含 ADR-0008 Odoo 借鉴 / ADR-0019 货主自定义）

#### 高保真原型（ADR-0021）
- [x] `prototypes/` Vite + React + TS + shadcn/ui 工程骨架
- [x] **全量高保真原型覆盖**：37 个手工高保真页 + 167 个全量矩阵页，共 204 个可走查 tab
- [x] **16 个 Layer 2 业务复合组件**（StatusBadge / ScanInput / DualSignPanel / AuditTimeline / ApprovalFlow / KanbanBoard / PrintPreview / RuleEditor / TempChart / OfflineIndicator / FieldTable / StepFlow / DiffPanel / PageHeader / DataTable / EmptyState）
- [x] **9 个 shadcn primitive**（Button / Input / Label / Card / Tabs / Select / Checkbox / Table / Dialog）
- [x] **16 个组件 Stories.tsx**（与组件 1:1）+ Storybook 8.6 接入（build 通过 7.65s）
- [x] 视觉基线治理：`accept_baseline.py` + `manifest.toml` + 204 个 baseline 全签字
- [x] OCR 关键字校验（`check_visual_keywords.py`）

#### 跨包共享（ADR-0028 / commit e3ce5a0）
- [x] **packages/ui 抽离**：16 业务组件 + 9 shadcn primitive + lib/utils + globals.css → `@wms/ui`
- [x] 仓库根 `pnpm-workspace.yaml`（packages/* + prototypes + apps/*）
- [x] Tailwind preset 化（`packages/ui/tailwind-preset.cjs`）
- [x] 153 处 import 改为 `@wms/ui`，治理脚本路径同步

#### 技术 Spike 计划（commit b2e84eb）
- [x] `docs/spikes/README.md` + 流程规范（accept/reject/defer 三态、时间盒、与 ADR 的关系）
- [x] SPIKE-001 Axum + JWT（2 天，关联 W1.A）
- [x] SPIKE-002 H2 append-only（2 天，关联 W1.B）
- [x] SPIKE-003 utoipa → OpenAPI → TS（1.5 天，关联 W1.C）
- [x] SPIKE-004 SQLx offline（1 天）
- [x] SPIKE-005 RN 扫枪（2 天，关联 PDA）

### 进行中 / 待做

- [x] **跑 5 个 Spike 验证**（实工 8.5h vs 时间盒 8.5d；4 accept + 1 defer）
  - [x] SPIKE-003 utoipa→OpenAPI→TS（accept，commit b3df10d → ADR-0026 Accepted）
  - [x] SPIKE-001 Axum + JWT（accept，commit 288a21c → ADR-0024 Accepted v0.2）
  - [x] SPIKE-002 H2 append-only（accept → ADR-0025 Accepted v0.2）
  - [x] SPIKE-004 SQLx offline（accept → ADR-0001 §SQLx 附录 v0.2）
  - [x] SPIKE-005 RN 扫枪（**deferred** → 推迟到 Wave 3 启动前；启动条件见 ROADMAP §v25 backlog + spike-005 §7.2）
- [x] 产出 ADR-0024 (Accepted v0.2) / 0025 (Accepted v0.2) / 0026 (Accepted)+ ADR-0001 §SQLx 附录 v0.2（已合入）；ADR-0027 推迟随 spike-005
- [x] **Wave 0.5 retro**（`docs/retros/wave-0.5-retro.md`，含 §11 持续演进补记）
- [x] 重新 capture visual snapshot 验证 e3ce5a0 后渲染（commit 62bf9eb：37 baseline 全部完美 mean_diff=0.00）
- [x] **前端原型先行工作流**：ADR-0029 + `docs/prototypes/prototype-to-production.md` 落地，确认原型可先行但不得直接复制为生产页
- [x] **全量矩阵原型补齐**：`docs/prototypes/index.toml` 167 个 required 原型页，`Tabs.tsx` + `manifest.toml` + baseline PNG 三同步，T3 视觉回归 204/204 通过

---

## Wave 0.5 退出条件（Wave 1 准入）

- [x] Storybook 可运行（commit 8cce777，build 通过）
- [x] 原型 ≥1 次走查 approved（manifest 204 tab 全签字）
- [x] packages/ui 抽离（commit e3ce5a0，ADR-0028 备案）
- [x] Spike 计划落盘（commit b2e84eb，5 项 docs/spikes/*.md）
- [x] **5 项 Spike 全部进入三态之一**（4 accepted: 001/002/003/004 / 1 deferred: 005）
- [x] 任一 accept 的 Spike 都有对应 ADR（0024 (Accepted v0.2) / 0025 (Accepted v0.2) / 0026 (Accepted) + 0001 §SQLx 附录 v0.2 已合入；0027 随 spike-005 推迟）
- [x] Wave 0.5 retro 写完（`docs/retros/wave-0.5-retro.md`，含 §11 持续演进补记）
- [x] T1 治理 24/24 全绿（持续条件）

**Wave 0.5 退出条件全部满足，可推进 Wave 1。**

---

## 原型前端检查遗留项（2026-05-31）

> 本次原型检查中**已完成且验证**的修复：
> - prototype-kit 7 个 tsc 类型错误已清零
> - 表格 / 看板类页面 mock 语义错位已修复（命中关键字的列）
>
> 以下两项为**已知、推迟**的技术债，留待后续有前端权限的执行者处理。

- [ ] 【低优】**原型 mock 占位数据语义错位**（`prototypes/src/prototype-kit/prototype-model.ts`）
  - 现状：`sampleValue` 采用"列名关键字猜测"生成占位值；部分列名（货主 / 控制属性 / 承运商 / 周转箱 / 客户门店 / 波次号 等）未命中关键字，回落到示例池，导致列头与单元格语义不符（如货主列显示商品名）。
  - 根治方案：将 `MODULE_BLUEPRINTS` 的列定义从 `string[]` 升级为"列名 + 示例值同源"结构（约 30 个 blueprint）。
  - 影响范围：仅原型占位数据，不影响布局 / 控件 / 业务流；转生产阶段会被真实样例替换。
- [ ] 【中优】**m4-manifest 随货同行单 PDF 中文竖排**（`prototypes/src/pages/m4-manifest/M4Manifest.tsx`）
  - 现状：商品明细表 9 列在 A4 画布内被挤压，中文品名 / 生产企业逐字竖排堆叠（已加 `table-fixed` + 百分比列宽未见改善）。
  - 处理方向：单独重构 `PrintPreview` 布局或缩字号。GSP 法定打印件，视觉要求高。

---

## 后续 Wave 预告（不在当前 TODO，仅参考）

- **Wave 1**：H1 权限/多租户、H2 审计追踪、H3 OpenAPI 工具链；`apps/web-admin/` 壳工程启动并复用 `@wms/ui`；`packages/api-client/` 自动生成
- **Wave 1 前端边界**：业务页从 `prototypes/` 迁移生产必须走 ADR-0029 checklist
- **Wave 2**：M1.a 基础档案 + M2 入库 schema + M6 报表骨架
- **Wave 3**：M2/M3 业务规则 + M5 冷链 schema + M9 计费账户

详见 [ROADMAP.md](ROADMAP.md)。
