# 全项目治理真实缺口修复计划（2026-07-11）

## 目标

以严格门禁的实际输出为任务事实源，不通过批量标记 `verified`、泛化延期、占位脚本或自动接受视觉基线制造假绿。每个任务批次必须同时关闭实现、测试、证据和治理登记。

## 本轮已关闭

| 任务 | 修复 | 验证 |
|---|---|---|
| G0-01 接口覆盖判定 | strict handler 覆盖只认可与具体请求变量关联的 2xx 状态断言，兼容 `Method::POST` 和 `.method("POST")`；404/405 路由盘点不冒充行为覆盖 | 治理回归测试通过，未覆盖 operation 保持可见 |
| G0-02 M2 修改安全底线 | 收货单 `PATCH` 只允许草稿态修改，禁止直接写状态和跨货主主数据引用；业务必填的供应商、预计到货时间不可清空，仅外部单号支持显式清空；PostgreSQL 更新与包含前后值的 H2 审计同事务强制提交，内存校验失败不产生部分修改 | 内存处理器测试 + PostgreSQL 集成测试；完整状态机仍由 G0-F / G1-B 收口 |
| G0-03 API 兼容基线 | 从本轮修改前的已发布 OpenAPI 建立 operation、参数、请求/响应和组件字段兼容面，覆盖响应必填字段变可选；保留既有 `DELETE` 契约 | `check_api_compat.py` 通过，OpenAPI 与 TS schema 同步 |
| G0-04 T3 调度 | `just preflight` 显式执行 T3 strict diff 治理；API 写路径匹配审计与幂等门禁，脚本缺失时 fail closed | 调度契约测试通过；当前缺失脚本保持阻断 |
| G0-05 页面 E2E 接线 | M1 系统字典、M2 收货、M3 批号、M4 出库订单增加三层菜单进入测试 | Playwright 10/10 通过 |
| G0-06 L7 证据复用 | T4 直接复用已有 W6.A 真实 `wrk` / 60M 基线验证器 | H2 runtime evidence 通过 |

## 待修任务

### G0 API 生产闭环

strict 检查仍应失败并暴露以下真实缺口，禁止挂载返回空台账或进程内数据的占位实现：

| 批次 | 缺口 | 完成条件 |
|---|---|---|
| G0-A | ~~M6 报表占位~~ **已关闭（最小生产闭环）** | 已挂载 `reports_handlers`：`/reports/query` 与 GSP 入/出/库存台账按 `owner_id` 查询 PostgreSQL；`m6.report.read` 权限；integration 测试覆盖汇总与入库台账 |
| G0-B | M-PM OpenAPI 已声明，但现有服务没有持久化字典、规则、队列和五年追溯 | 完成数据库迁移、配置 API、缓存失效、权限授权、审计和多实例测试后再挂载 |
| G0-C | 收货单 `DELETE` 是已发布契约但没有运行时实现，且业务故事要求作废而非物理删除 | 先确定兼容迁移，再实现带状态机、权限、幂等和审计的作废语义；不得物理删除历史单据 |
| G0-D | 多数 OpenAPI operation 只有路径提及或未授权状态测试 | 每个 operation 至少覆盖成功路径、权限、业务错误和关键事务；route inventory 只能作为挂载检查 |
| G0-E | ~~T3 审计/幂等脚本未实现~~ **已关闭** | 已实现真实规则：`check_audit_trail_coverage`（OpenAPI 写路径 + HTTP 写成功测试 → audit_event 证据）与 `check_idempotency_test`（Idempotency-Key 写操作 → 测试幂等证据）；已知缺口入 `governance/baselines/`，禁止新增回退 |
| G0-F | ~~ASN 无放行入口~~ **已关闭（最小生产闭环）** | 新增 `POST /receiving-orders/{id}/release`：draft→released，校验 active 供应商/仓库，幂等 + 同事务审计；内存路径要求 supplier_id；完整 M-VR 资质引擎仍待后续增强 |

退出命令：

```bash
python3 scripts/governance/check_runtime_route_mounts.py --strict
python3 scripts/governance/check_handler_test_coverage.py --strict
```

### G1 质量矩阵与菜单闭环

当前严格输出：38 个活跃模块故事未登记、12 个菜单页未进入质量矩阵，另有 4 个已登记前端故事缺业务 E2E 证据。菜单可达性检查只能证明入口存在，不能替代业务 E2E。

| 批次 | 范围 | 完成条件 |
|---|---|---|
| G1-A | M1 的 10 个未登记故事和 6 个菜单页 | 逐故事核对前端、API、后端、数据库、权限、审计、测试；缺功能先实现；真实页面 E2E 通过 |
| G1-B | M2 的 9 个未登记故事和 2 个菜单页 | 收货、验收、上架状态与字段矩阵一致；导航可达性与业务 E2E 分开登记，真实 API + 业务 E2E + 截图证据通过 |
| G1-C | M3 的 8 个未登记故事 | 库存、批号、盘点、移库、冻结等逐故事闭环，不得用一个列表页代替全部故事 |
| G1-D | M4 的 11 个未登记故事和 3 个菜单页 | 订单、波次、拣选、复核、发货、采购退货逐故事闭环 |
| G1-E | H4 的 1 个菜单页及 M1/M2/M3/M4 共 4 个故事的业务 E2E | H4 参数页进入矩阵；4 个故事分别登记并执行真实业务 E2E，不得复用菜单导航命令冒充 |

退出命令：

```bash
python3 scripts/governance/check_scope_gap_discovery.py --strict
python3 scripts/governance/check_quality_matrix.py
pnpm --dir apps/web-admin run test:e2e:shell-dev
```

### G2 限界上下文声明

当前严格输出：ADR-0012 定义 24 个限界上下文，仅 M-TC 有 manifest，缺 23 个。

按业务模块、横向业务和横向能力分三批补 `docs/domain/<module>/module-manifest.toml`。每份必须从 ADR-0012、架构依赖图和对应用户故事提取依赖、集成模式、共享内核和 owner，不得复制 M-TC 示例后只改名称。

退出命令：

```bash
python3 scripts/governance/check_bounded_contexts.py --strict
```

### G3 多端一致性规则

当前严格输出：14 个核心故事文件没有 A/B/C 规则分类，共 28 个提示。

每个验收标准按 ADR-0015 分类：A 类要求服务端强一致；B 类允许端侧校验但服务端复核；C 类只影响展示。分类后必须抽样检查实现位置，不能只加标签。

退出命令：

```bash
python3 scripts/governance/check_multi_end_consistency.py --strict
```

### G4 可观测性闭环

当前严格输出：15 个核心故事文件缺 KPI；运行时另有 3 个缺口：未接入 `tracing + OpenTelemetry`、无 `tracing::instrument`、无 W3C `traceparent` 传播。现有 `/metrics` 只覆盖 H3 弹性计数器。

执行顺序：先接通 trace context 和结构化 span，再按真实业务写点实现指标，最后把已实现的 KPI 回写故事文件。禁止先写 75 个 KPI 名称让文档检查通过。

退出命令：

```bash
python3 scripts/governance/check_observability.py --strict
```

### G5 视觉基线复核

本轮已重新采集 207 个页面：1 个通过、194 个阈值错误、6 个警告、6 个尺寸不一致导致的对比错误、0 个截断。后续由视觉审查确认差异；禁止脚本自动接受全部基线，只有确认后的页面可更新 manifest 和 PNG。

退出命令：

```bash
python3 scripts/governance/capture_visual_snapshots.py --port 15173 --start-server
python3 scripts/governance/check_visual_regression.py
```

### G6 外部与预发布证据

W6.D-H 依赖真 PDA、码上放心、硬件、TMS 和 staging 灰度环境，保持 fail closed。不得用 localhost、mock、fake、example 或生产环境替代。

退出命令：

```bash
just wave-6-complete-check
```

## 提交规则

每个 G1-G6 批次独立 review loop 和分组提交。`governance/baselines/openapi-compat-v1.json` 只有在兼容性审查确认后才能通过 `python3 scripts/governance/check_api_compat.py --update-baseline` 更新；普通功能提交不得顺手刷新基线。
