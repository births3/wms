# wms TODO（当前 Wave）

> 任务追踪粒度：当前 Wave 内的具体任务。
> Wave 切换时归档当前 TODO 并启动新 TODO。
> 长期路线见 [ROADMAP.md](ROADMAP.md)。

---

## 当前 Wave：Wave 6 — 预发布证据与外部依赖收口（启动中）

**目标**：不新增业务模块，集中关闭 Wave 1-5 后移的真实 dev/staging、硬件、TMS、码上放心和灰度发布 evidence gate。范围定义见 [ADR-0035](docs/adr/0035-wave-6-pre-release-evidence-closeout.md)。

### 当前阻塞 / 外部依赖

- [ ] 稳定 dev/staging 环境仍未就绪；Wave 6 的真实 runtime evidence 每个引用必须包含当前 `environment` 标记，不能用 localhost / stub / mock / fake / example / prod 替代
- [ ] M-PK 电子秤 / 蓝牙打印机 / 面单打印真实设备未接入
- [ ] 外部 TMS dev/staging 接口、回调鉴权、调度结果格式仍需确认
- [ ] “码上放心”账号、正式接口文档、鉴权方式、错误码、频率限制和 dev/staging 回执仍需补齐
- [ ] 首次试运行投产灰度发布环境和回滚链路需按 ADR-0016 准备

### 进行中 / 待做

- [x] W6 scope：ADR-0035 已定义 Wave 6 为预发布证据收口波次，不新增业务功能
- [x] W6 status / complete check：建立 Wave 6 证据收口报告与 `just wave-6-status` / `just wave-6-complete-check`
- [x] W6 closeout runbook：建立 [Wave 6 Closeout Runbook](docs/runbooks/wave-6-closeout.md)，集中 8 个 evidence gate 的记录与验证顺序
- [ ] W6.A Wave 1 H2 runtime evidence：按 [Wave 1 Pre-release Runtime Evidence Runbook](docs/runbooks/wave-1-runtime-evidence.md) 采集 `docs/retros/wave-1-h2-runtime-evidence.json`，通过 `just wave-1-runtime-evidence-validate`
- [ ] W6.B Wave 1 W1.D 自动回滚 evidence：按同一 runbook 采集 `docs/retros/wave-1-runtime-evidence.json`，通过 `just wave-1-runtime-evidence-validate`
- [x] W6.C tooling：在 [Wave 2 Pre-release Runtime Evidence Runbook](docs/runbooks/wave-2-runtime-evidence.md) 补 record 命令，建立 `record_wave2_runtime_evidence.py` 与 `just wave-2-runtime-evidence-record`
- [ ] W6.C Wave 2 配置中心 Feature Flag evidence：按 runbook 采集 `docs/retros/wave-2-runtime-evidence.json`，通过 `just wave-2-runtime-evidence-validate`
- [x] W6.D tooling：在 [Wave 3 PDA Readiness Runbook](docs/runbooks/wave-3-pda-readiness.md) 补 evidence schema / record 命令，建立 `record_wave3_pda_runtime_evidence.py`、`validate_wave3_pda_runtime_evidence.py` 与对应 `just` 入口
- [ ] W6.D Wave 3 真 PDA + L7 evidence：按 runbook 补齐 `docs/retros/wave-3-pda-runtime-evidence.json`，通过 `just wave-3-pda-runtime-evidence-validate`
- [ ] W6.E Wave 4 M-TC “码上放心” external evidence：按 [Wave 4 External Dependency Evidence Runbook](docs/runbooks/wave-4-external-dependencies.md) 补齐 `docs/retros/wave-4-external-dependencies.json`，通过 `just wave-4-external-dependencies-validate`
- [x] W6.F tooling：建立 [Wave 5 Hardware Evidence Runbook](docs/runbooks/wave-5-hardware-evidence.md)、`record_wave5_hardware_evidence.py`、`validate_wave5_hardware_evidence.py` 与对应 `just` 入口
- [ ] W6.F Wave 5 M-PK hardware evidence：按 runbook 补齐 `docs/retros/wave-5-hardware-evidence.json`，通过 `just wave-5-hardware-evidence-validate`
- [x] W6.G tooling：建立 [Wave 5 TMS+ Evidence Runbook](docs/runbooks/wave-5-tms-evidence.md)、`record_wave5_tms_evidence.py`、`validate_wave5_tms_evidence.py` 与对应 `just` 入口
- [ ] W6.G Wave 5 M10 TMS+ evidence：按 runbook 补齐 `docs/retros/wave-5-tms-evidence.json`，通过 `just wave-5-tms-evidence-validate`
- [x] W6.H tooling：建立 [Wave 6 Gray Release Evidence Runbook](docs/runbooks/wave-6-deploy-evidence.md)、`record_wave6_deploy_evidence.py`、`validate_wave6_deploy_evidence.py` 与对应 `just` 入口
- [ ] W6.H 首次试运行灰度发布 evidence：按 runbook 补齐 `docs/retros/wave-6-deploy-evidence.json`，通过 `just wave-6-deploy-evidence-validate`
- [ ] W6 retro：Wave 6 完成后写 `docs/retros/wave-6-retro.md`

### 非范围

- 不新增业务模块
- 不启动 v26 GSP 字段命名规范化
- 不补 i18n
- 不把本地 PostgreSQL runtime test 当作 dev/staging evidence

Wave 1 / Wave 2 / Wave 3 / Wave 4 / Wave 5 开发完成状态仍分别以 `just wave-1-complete-check` / `just wave-2-complete-check` / `just wave-3-complete-check` / `just wave-4-complete-check` / `just wave-5-complete-check` 为准；Wave 6 只关闭预发布 evidence。

---

## 已归档：Wave 5 — 增值模块全面铺开（开发完成）

**目标**：增值业务模块生产可用，覆盖包装站、连锁专有、3PL 计费和 TMS+ 协作。

### 已完成

- [x] W5 completion report：`report_wave5_completion.py` 与 `just wave-5-complete-check` 已建立
- [x] W5.A M-PK 包装站：工位、装箱、称重、面单打印生产接口、PostgreSQL 表和 OpenAPI 契约已落地
- [x] W5.B M8 连锁专有：门店水位补货建议、越库作业生产接口、PostgreSQL 表和 OpenAPI 契约已落地
- [x] W5.C M9 3PL 计费：自动计费、计费明细、月结账单生产接口、PostgreSQL 表和 OpenAPI 契约已落地
- [x] W5.D M10 TMS+：调度接收、在途温控关联、容器回收生产接口、PostgreSQL 表和 OpenAPI 契约已落地
- [x] W5 tenant isolation：`wave5_owner_isolation` 真实 PostgreSQL 测试覆盖 M-PK / M8 / M9 / M10 owner_id 隔离
- [x] W5 chain scenario：`chain_store_replenishment_to_packing_tms_and_billing` 真实 PostgreSQL 测试覆盖门店补货 → 越库 → 装箱 → TMS/快递 → 计费链路
- [x] W5 H3 同步：`shared/openapi/openapi.json` 与 `packages/api-client/src/schema.ts` 已同步 Wave 5 path/schema

### 验证

- `just wave-5-complete-check`：通过
- `cargo fmt --check --all`：通过
- `cargo check --manifest-path backend/Cargo.toml -p wms-api`：通过
- `cargo test --manifest-path backend/Cargo.toml -p wms-api --lib -- --skip postgres_`：66 passed
- 临时 PostgreSQL 执行 `cargo test --manifest-path backend/Cargo.toml -p wms-api --test wave5_postgres -- --nocapture`：3 passed
- `just openapi-check`：通过
- `python3 -m pytest scripts/governance/tests/test_core_logic.py -q`：140 passed
- `just gov-t1`：30/30 ok
- `just task-check`：6/6 ok
- `git diff --check`：通过

### 后续跟踪（进入 Wave 6）

- W5.A hardware evidence gate：电子秤、蓝牙打印机、面单打印真实设备联调证据
- W5.D TMS evidence gate：真实 dev/staging TMS 推送、回调、失败重试和 audit_event 查询证据

---

## 已归档：Wave 4 — 完整闭环 + 横向叠加（开发完成）

**目标**：单货主下完整业务闭环（采购入库 → 库存 → 销售出库 → 冷链监控 → GSP 报表）可上线试运行。

### 当前阻塞 / 外部依赖

- [x] Wave 4 开发完成阻塞已关闭；W4.D "码上放心"真实 dev/staging 外部 evidence 按 `docs/domain/clarifications.md` #50 延期，不阻塞 Wave 4，但后续仍必须单独补齐并验证（见下方后续跟踪）

### 进行中 / 待做

- [x] W4 completion report：`report_wave4_completion.py` 与 `just wave-4-complete-check` 已建立，后续随实现更新证据
- [x] W4 scope：W4.F M-PK 不纳入 Wave 4，已延后到 Wave 5.A（见 `docs/domain/clarifications.md` #48）
- [x] W4.A 短拣决策：采用 `clarifications.md` #43 C 方案，发货前必须补拣补齐
- [x] W4.A M4 出库：订单 / 波次 / 拣选 / 复核 / 打印随货同行单 / 发货交接
- [x] W4.B M5 冷链业务联动：温度超标事件待处置列表 + 主管勾选隔离批次
- [x] W4.C M6 报表实现：GSP 入库 / 出库 / 库存法定台账
- [x] W4.D M-TC 码上放心：追溯码出库核销事件实时上报（内部契约、待补报持久化队列、审计与 OpenAPI 已完成；正式平台 evidence 按 #50 后续关闭）
- [x] W4.E 司机端 / 门店端：主动故事生产接口落地
- [x] W4 audit invariant：Wave 4 关键写操作保持 H2 append-only 审计不变量

### 后续跟踪（不计入 Wave 4 开发完成）

- W4 pre-release deploy gate：首次试运行投产必须使用 ADR-0016 灰度发布链路，不允许全量直发
- W4.D external evidence gate："码上放心"账号 / 正式接口文档 / 鉴权方式 / 错误码 / 频率限制 / dev/staging 成功回执 / 失败重试 / audit_event 查询证据后续补齐；使用 `just wave-4-external-dependencies-record ...` 记录，再跑 `just wave-4-external-dependencies-validate`
- W3 PDA production gate：生产 PDA app 等真 PDA 与 SPIKE-005 验证后启动
- W3 L7 pre-release gate：有稳定 dev/staging + 真 PDA 后，按 [Wave 3 PDA Readiness Runbook](docs/runbooks/wave-3-pda-readiness.md) 启动 SPIKE-005，采集 M2/M3 性能/易用性证据
- W2.G pre-release runtime gate：有稳定 dev/staging 后，按 [Wave 2 Pre-release Runtime Evidence Runbook](docs/runbooks/wave-2-runtime-evidence.md) 验证配置中心版 Feature Flag 迁移、对账、切源、旧文件归档和 smoke，写入 `docs/retros/wave-2-runtime-evidence.json`
- W1.B / W1.D pre-release runtime gate：仍按 [Wave 1 Pre-release Runtime Evidence Runbook](docs/runbooks/wave-1-runtime-evidence.md) 在真实环境补齐
- W2-external：人工确认“码上放心”账号外部开通状态；外部依赖仍按 [ROADMAP.md](ROADMAP.md) 的外部依赖追踪表跟进

Wave 1 / Wave 2 / Wave 3 开发完成状态仍分别以 `just wave-1-complete-check` / `just wave-2-complete-check` / `just wave-3-complete-check` 为准；上述 runtime gate 在预发布前单独验证，每个 evidence 引用必须包含当前 `environment` 标记，禁止用 localhost / stub / mock / fake / example / prod 代替。

---

## 已归档：Wave 3 — 核心业务规则铺开（开发完成）

**目标**：M2 入库业务规则、M3 库存模型与状态规则、M5 外部冷链数据接入 schema、M9 计费账户/合同模型逐步落地。

### 已完成（Wave 3 第一批后端切片）

- [x] W3.A backend：M2 收货闭环校验、验收效期校验、双人签字、上架记录服务与单测
- [x] W3.B backend：M3 库存批次模型、入库上架增加库存、可用库存、状态机审批源约束与单测
- [x] W3.C backend：M5 外部温控数据/超标事件接入 schema 与幂等服务；保持 WMS 不采集、不判定超标的边界
- [x] W3.D backend：M9 计费账户 / 合同 / 规则模型与基础校验
- [x] W3 handler shell：Wave 3 第一批 Axum handler 组合已接入权限检查、错误响应与 H2 审计写入；覆盖 M2 workflow / M3 inventory / M5 cold-chain / M9 billing
- [x] W3 repository design：ADR-0034 已 Accepted，确认 Wave 3 第一批 PostgreSQL 表结构与事务边界
- [x] W3 repository first slice：新增 Wave 3 core tables migration；M2 收货闭环一单一次 + 幂等、M2 上架与 M3 库存/流水同事务、M9 生效期冲突均有真实 PostgreSQL 测试覆盖
- [x] W3.A handler persistence：M2 receive/inspect/sign/putaway handler 已接 PostgreSQL repository；业务表 + `idempotency_request` + H2 `audit_event` 同事务落库，并有真实 PostgreSQL handler 测试覆盖
- [x] W3.B repository：M3 库存查询与状态变更 handler 已接 PostgreSQL repository；状态变更写 `inventory_status_changes` + `idempotency_request` + H2 `audit_event`，覆盖 owner 隔离、缺审批源错误、幂等 replay 与真实 PostgreSQL 测试
- [x] W3.C external auth：M5 外部冷链 readings/excursions 使用 `X-WMS-API-Key` + `Idempotency-Key`，接 PostgreSQL repository、`idempotency_request` 与 H2 `audit_event`，并有真实 PostgreSQL handler 测试覆盖
- [x] H3 同步：`shared/openapi/openapi.json` 与 `packages/api-client/src/schema.ts` 已反映 Wave 3 第一批 path/schema
- [x] 治理：`check_openapi_contract.py` 要求 Wave 3 第一批 path/schema
- [x] W3 completion report：新增 `report_wave3_completion.py` 与 `just wave-3-complete-check`，汇总 M2/M3 11 层证据和 Wave 3 阻塞项
- [x] W3.A PDA readiness：按 SPIKE-005 先落设备清单与 runbook，不引入 RN 依赖，不创建生产 app
- [x] W3 完成门禁：M2 / M3 关键路径 L1-L6/L8-L11 已有静态证据，L7 为预发布 gate；GSP 资质有效期校验来源冻结为 M1 本地资质档案 + M-VR 校验规则执行

### 后续 / 不阻塞 Wave 3 开发完成

- [ ] W3.A PDA 生产端：`apps/pda-mobile` 目前只有 `.gitkeep`，生产 app 等真 PDA 与 SPIKE-005 验证后启动，作为预发布 gate 跟踪
- [x] W3.D 后续：M9 自动计费与账单管理已在 Wave 5.C 完成；`just wave-5-complete-check` 通过

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

- [x] 【低优】**原型 mock 占位数据语义错位**（`prototypes/src/prototype-kit/prototype-model.ts`）
  - 现状：`sampleValue` 采用"列名关键字猜测"生成占位值；部分列名（货主 / 控制属性 / 承运商 / 周转箱 / 客户门店 / 波次号 等）未命中关键字，回落到示例池，导致列头与单元格语义不符（如货主列显示商品名）。
  - 根治方案：将 `MODULE_BLUEPRINTS` 的列定义从 `string[]` 升级为"列名 + 示例值同源"结构（约 30 个 blueprint）。
  - 影响范围：仅原型占位数据，不影响布局 / 控件 / 业务流；转生产阶段会被真实样例替换。
  - 核对结果：`prototype-model.ts` 已改为 `rowSample` / `fieldSample` 与列/字段逐项对应，不再使用关键字猜值；`check_prototype_fidelity.py` 已随 `just gov-t1` 通过。
- [x] 【中优】**m4-manifest 随货同行单 PDF 中文竖排**（`prototypes/src/pages/m4-manifest/M4Manifest.tsx`）
  - 现状：商品明细表 9 列在 A4 画布内被挤压，中文品名 / 生产企业逐字竖排堆叠（已加 `table-fixed` + 百分比列宽未见改善）。
  - 处理方向：单独重构 `PrintPreview` 布局或缩字号。GSP 法定打印件，视觉要求高。
  - 核对结果：`PrintPreview` 已改为按真实纸张尺寸排版后缩放显示，`m4-manifest` 去除重复内边距；`just matrix-e2e-full` 已验证 204/204 通过，随货同行单启用 `detect_vertical_cjk_table`。

---

## 后续 Wave 预告（不在当前 TODO，仅参考）

- **Wave 1**：H1 权限/多租户、H2 审计追踪、H3 OpenAPI 工具链；`apps/web-admin/` 壳工程启动并复用 `@wms/ui`；`packages/api-client/` 自动生成
- **Wave 1 前端边界**：业务页从 `prototypes/` 迁移生产必须走 ADR-0029 checklist
- **Wave 2**：M1.a 基础档案 + M2 入库 schema + M6 报表骨架
- **Wave 3**：M2/M3 业务规则 + M5 冷链 schema + M9 计费账户

详见 [ROADMAP.md](ROADMAP.md)。
