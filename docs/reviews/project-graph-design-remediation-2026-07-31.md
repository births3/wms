# 全项目图谱设计问题整改任务与验收标准（2026-07-31）

> 状态：Wave 6 后待排期；本文只把已复核问题转成可验收任务，不代表对应架构、安全、
> 业务或部署决策已经批准。执行前必须有后继 Wave / 范围决策，不能纳入当前 Wave 6。
>
> 文件规模例外：本文保留唯一任务入口、验收标准和 Review Loop；当前 837 行，拆分会破坏既有锚点，后续仅做内容增量。
>
> 分析基线：`01c7a097cb1843a4427143e7c19d93faf878c49c`；
> `.ua/meta.json` 与该提交一致，分析时没有未提交的产品源码变化。

## 1. 目标与边界

本计划用于关闭项目知识图谱暴露出的真实设计问题，并把“代码修复、测试、证据、治理检查、
图谱更新”放进同一个 Review Loop。

本计划遵守以下边界：

- 只收录已由源码、契约、测试、部署配置或 Accepted ADR 复核的问题；图谱推断单独标记。
- 不重复 [全项目治理真实缺口修复计划](governance-remediation-plan-2026-07-11.md) 已管理的批次。
- 不以拆微服务、增加框架或全仓重写作为默认解法；先修一条真实业务链路的根因。
- 首个正式版本前遵守 [ADR-0038](../adr/0038-pre-v1-compatibility-policy.md)，不新增兼容层。
- 当前 Wave 6 受 [ADR-0035](../adr/0035-wave-6-pre-release-evidence-closeout.md) 限制，只做
  预发布证据收口；本计划的代码、故事和架构任务全部在 Wave 6 后待排期。
- 修改状态、字段、角色、基础设施、跨模块语义或安全降级策略前，必须先取得用户确认。
- 图谱用于发现和复核，不替代 ADR、用户故事、OpenAPI、质量矩阵或真实运行证据。

## 2. 当前基线为什么不能作为完成证据

以下命令在 2026-07-31 均返回退出码 `0`，但仍未覆盖本计划发现的语义缺口：

| 命令 | 当前能证明 | 当前不能证明 |
|---|---|---|
| `python3 scripts/governance/check_layer_dependency.py --json` | `wms-domain` 未依赖 Axum/SQLx/API | `wms-api` 内 service/repository 没有反向依赖 runtime `auth.rs` |
| `python3 scripts/governance/check_idempotency_test.py --json` | 191 个写 operation 都存在幂等测试文本证据 | 20 多套幂等实现的锁、过期、冲突和作用域语义一致 |
| `python3 scripts/governance/check_quality_matrix.py --json` | 质量矩阵结构合法 | H5 的 dev mock 导航不是业务真实 E2E |
| `python3 scripts/governance/check_scope_gap_discovery.py --strict --module H5 --json` | H5 故事已经登记 | 承运商、规则、下单、取消、轨迹和打印经过真实 HTTP/PostgreSQL |
| `python3 scripts/governance/check_openapi_in_sync.py --json` | 主 WMS OpenAPI 与主 api-client 同步 | 调用方未使用裸 `fetch`，以及客户平台契约已生成 TypeScript 类型 |

因此，任何任务都不得仅凭上述命令保持绿色就标记完成。

另有一项已直接失败的基线：
`python3 scripts/governance/generate_table_catalog.py --check --json` 曾返回退出码 `1`；
AR-12 首切片已修复生成链并将目录刷新为 193 张静态表，后续仍需补空库基线与迁移决策证据。

### 2.1 已复核证据锚点

| 问题 | 源码、契约或配置证据 |
|---|---|
| Auth fail-open | [`AuthRuntimePolicy::new`（104 行）](../../backend/crates/api/src/auth.rs)、[存储异常放行（139 行）](../../backend/crates/api/src/auth.rs)、[生产组合（162 行）](../../backend/crates/api/src/bin/wms_api.rs) |
| `AuthContext` 反向依赖 | [`auth.rs` 混合职责](../../backend/crates/api/src/auth.rs)、[审计仓储导入](../../backend/crates/api/src/audit/db.rs)、[打印服务导入（15 行）](../../backend/crates/api/src/print_orchestration/service.rs) |
| H6 与 M4 状态漂移 | [H6 出库定义（260 行）](../../backend/crates/api/src/state_machine.rs)、[M4 状态常量（5 行）](../../backend/crates/api/src/outbound.rs)、[实际短拣状态写入（400 行）](../../backend/crates/api/src/wave4_repository_part1.rs) |
| Wave4 repository 编排 | [handler 持有具体仓储（38 行）](../../backend/crates/api/src/wave4_handlers.rs)、[发运仓储方法（16 行）](../../backend/crates/api/src/wave4_repository_part2.rs) |
| 幂等语义分裂 | [全局唯一约束（3 行）](../../backend/migrations/202606030001_wave3_core_tables.sql)、[Admin Menu 实现（64 行）](../../backend/crates/api/src/admin_menu_idempotency.rs)、[Dock 实现（331 行）](../../backend/crates/api/src/dock_repository.rs)、[企微锁实现（100 行）](../../backend/crates/api/src/wechat_notify_idempotency.rs) |
| M4 查询截断 | [查询 hook 未传参数（21 行）](../../apps/web-admin/src/features/outbound/outbound-queries.ts)、[页面客户端过滤（273 行）](../../apps/web-admin/src/pages/outbound/M4OutboundPage.tsx)、[后端默认 50 条（7 行）](../../backend/crates/api/src/wave4_repository_part1.rs) |
| 前端契约绕过 | [M3 自建 fetch/DTO（8 行）](../../apps/web-admin/src/features/inventory/m3-ops-queries.ts)、[客户平台手写类型](../../apps/customer-portal/src/types.ts)、[客户平台字符串 path（22 行）](../../apps/customer-portal/src/api.ts)、[客户平台 OpenAPI](../../shared/openapi/customer-portal-openapi.yaml) |
| 菜单/视图、示例 KPI 与 M4 循环 | [示例 KPI（49 行）与静态菜单源（81 行）](../../apps/web-admin/src/App.tsx)、[renderer 映射（14 行）](../../apps/web-admin/src/app-shell/AdminViewRenderer.tsx)、[模型反向导入组件（8 行）](../../apps/web-admin/src/pages/outbound/m4-outbound-page-model.ts) |
| Render Worker 启动耦合 | [staging API 依赖 worker healthy（105 行）](../../deploy/docker-compose.staging.yml) |
| H5 与同类假绿证据 | [质量矩阵登记（H1/H2/H3 约 1059/1129/1393 行，H5 约 1507 行）](../../governance/quality-matrix.toml)、[dev mock 配置（24 行）](../../prototypes/playwright-web-admin-dev-config.ts)、[仅导航/固定文本用例（445 行）](../../prototypes/e2e/web-admin-shell.spec.ts) |
| 数字 `include!` 分片 | [Wave3 repository（317 行）](../../backend/crates/api/src/wave3_repository.rs)、[Wave4 repository（208 行）](../../backend/crates/api/src/wave4_repository.rs) |
| migration 跨域关系 | [全局标准对齐 migration（168 行）](../../backend/migrations/202606280002_database_design_standard_alignment.sql) |
| 图谱覆盖缺口 | `.ua/knowledge-graph.json`、`.ua/domain-graph.json`、`docs/agent-knowledge-graph.md` |

## 3. 批次、依赖和状态

| ID | 任务 | 业务优先级 | 治理类型 | 前置 | 状态 |
|---|---|---|---|---|---|
| AR-01 | 确认鉴权存储故障时的高风险写操作策略 | 阻断决策 | P2 | 无 | 已完成（`8e9f568`） |
| AR-02 | 分离传输无关的操作人/货主上下文 | 高 | P1+P2 | AR-01 可并行决策 | 已完成（`b3bee45`） |
| AR-03 | 修复 M4 截断数据上的客户端查询 | 阻断缺陷 | P0+P1 | 无 | 已完成（代码与真实 M4 E2E 通过） |
| AR-04 | 建立 M4 发运应用服务边界 | 高 | P1+P2 | AR-02；保持既有幂等语义或等待 AR-06 明确 | 已完成（`2bbb4bd`） |
| AR-05 | 让一条 M4 关键链路真正复用 H6 状态规则 | 高 | P2 | AR-04、状态确认 | 已完成（`cf352ee`） |
| AR-06 | 收敛 L11 幂等语义和重复实现 | 高 | P1+P2 | 保持 ADR-0034；确认存储权威 | 已完成（PostgreSQL-only、直接访问 baseline=0、同键并发证据通过，`567fb98`） |
| AR-07 | 恢复生产前端 OpenAPI 客户端唯一入口 | 高 | P0+P1 | 无 | 已完成（M3 与客户平台真实 E2E 通过） |
| AR-08A | 收敛菜单、视图和故障回退边界 | 中高 | P1+P2 | 菜单故障策略确认 | 本地菜单闭环完成，H1 完整真实证据待补（`79d2688`、`3be857f`） |
| AR-08B | 消除 M4 模型与组件循环依赖 | 中高 | P1 | AR-03 可独立先做 | 已完成（代码、自检、构建与 M4 真实 E2E 通过，`2149b85`） |
| AR-09 | 解除 Render Worker 对核心 API 启动的硬阻塞 | 高 | P1+P2 | 部署行为确认 | 实现/服务级验证完成，隔离 Compose smoke 脚本已补；执行受 MinIO 镜像拉取网络阻塞（`ea93dfe`） |
| AR-10 | 修正 H5 与同类虚假真实 E2E/截图证据 | 阻断证据 | P1 | 可用测试 PostgreSQL | 本地闭环完成，外部项 deferred（`dd699b8`） |
| AR-11 | 阻止新增数字 `include!` 并改造一个代表聚合 | 中 | P1+P2 | AR-04/AR-05 后独立执行 | 技术闭环完成，范围确认待补（`2948bd2`、`eb4f770`） |
| AR-12 | 复核首版前 migration 基线与表所有权 | 中 | P2 | 数据库方案确认 | 环境漂移已确认，备份/运行证据待补（`9a97537`、`83419d4`） |
| KG-01A | 补齐确定性可追溯图谱关系 | 中高 | P1+P2 | schema/更新命令确认；ALTER 关系等待 AR-12 | validator 与 schema/更新命令已闭环，官方图谱产物待补（`53d4a36`、`867c2ff`） |
| KG-01B | 修正业务域拓扑 | 中 | P1 | 既有计划 G2 完成 | 阻塞 |
| KG-02 | 确认并修正图谱新鲜度语义 | 中高 | P2 | 新鲜度方案确认 | B 方案与规则闭环完成，官方图谱刷新待补（`9a95416`） |
| REDIS-01 | 评估 Redis 是否仍是必要基础设施 | 中高 | P2 | 无；不得先改运行时 | 决策方向完成，后续迁移 blocked（`d1109a3`） |

治理类型沿用 [AI 协作规范](../agent-collaboration.md)：P0 为现有脚本可验证，P1 为可新增
或扩展脚本验证，P2 为必须人工语义判断。

### 3.1 与既有任务的唯一关闭关系

本计划不产生重复关闭。重叠范围仍由既有计划或故事作为父状态源，本计划只提供可执行子任务；
子任务完成后必须同步父项，不能在两处各自宣布整项完成。

| 本计划 | 既有事实源 | 关系 | 唯一关闭动作 |
|---|---|---|---|
| AR-01/AR-02 | ADR-0024、`layered-design.md` | 新安全决策 / 分层整改 | 新 ADR 决定 AR-01；AR-02 只关闭分层依赖 |
| AR-03/AR-04 | 既有计划 G1-D、M4 故事与 RTM | G1-D 子任务 | 本计划验收后回写 G1-D，不单独关闭全部 M4 |
| AR-05 | US-H6-001、RTM、G1-D | H6 执行新切片 | 新故事确认后由质量矩阵记录真实覆盖 |
| AR-06 | ADR-0018、ADR-0034、编码规范 §3.3 | 技术债整改 / 存储偏差决策 | 存储分支实现和直接访问 baseline 均关闭后才完成 |
| AR-07 | 既有计划 G1-C/G0-D、ADR-0042 | M3/契约子任务 | 分别回写 M3 父项和客户平台契约证据 |
| AR-08A | H1-007、菜单设计、既有计划 G1 | 菜单一致性子任务 | 只关闭故障回退与集合一致性，不关闭全部菜单治理 |
| AR-08B | 既有计划 G1-D | M4 技术债子任务 | 只关闭循环依赖 |
| AR-09 | US-H9-009 与既有 H9 验收记录 | 部署韧性补充 | 不把 Render Worker smoke 写成 Print Agent/S4 完成 |
| AR-10 | H1/H2/H3/H4/H5 RTM、质量矩阵 | 证据纠偏 | 只升级有真实证据的故事；外部/硬件项继续 deferred/blocked |
| AR-11 | `layered-design.md` | 新技术债任务 | 新增门禁和代表聚合分别提交，二者都通过才关闭 |
| AR-12 | 现有 table catalog、数据库 review、ADR-0038 | 现有产物修复 / 决策 | 不创建第二份表目录 |
| KG-01A | 图谱运行手册、AR-12 | 新图谱质量任务 | 只关闭确定性追踪边和 validator |
| KG-01B | 既有计划 G2 | G2 后续任务 | G2 补齐 manifest 后独立关闭域拓扑 |
| KG-02 | 图谱运行手册、`AGENTS.md` | 新治理决策 | 新鲜度方案确认并同步全部入口后关闭 |
| REDIS-01 | [Redis 必要性评估任务](redis-necessity-assessment-task-2026-07-31.md)、ADR-0018、ADR-0024、缓存策略 | 基础设施决策 | 只形成存储选型结论；实现和删除另立后续任务 |

`TODO.md` 只保留本计划的单一入口；具体状态以本表、3.2 执行回写和各父事实源为准。

### 3.2 2026-08-01 执行回写

以下是当前工作树的 Review Loop 状态；原验收标准仍是关闭任务的唯一判定，不以静态账本替代真实证据。

| 任务 | 当前状态 | 当前证据或阻塞 |
|---|---|---|
| AR-01 | 已完成 | 高风险写入 fail-closed；ADR-0046、策略矩阵和 H1 运行手册已同步（`8e9f568`）。 |
| AR-02 | 已完成 | 操作上下文已从 runtime 鉴权边界分离（`b3bee45`）。 |
| AR-03 | 历史证据通过，当前重跑受环境漂移阻塞 | M4 查询参数已进入请求与 query key；历史默认窗口外真实 PostgreSQL/Playwright 证据与截图保留。2026-08-01 复跑 `pnpm --dir apps/web-admin run test:e2e:m4-real` 在 webServer 启动阶段失败：`VersionMismatch(202606030001)`；需按 AR-12 补齐测试库 migration 后重采集，不能把旧截图当作当前运行证据。 owner：测试/部署负责人；恢复条件：修复测试库 migration 漂移并重新通过真实 Playwright。 |
| AR-04 | 已完成 | 发运 application service 与 repository port 已落地，真实事务回滚证据通过（`2bbb4bd`）。 |
| AR-05 | 已完成 | 采用“共享纯规则 + M4 事务执行”（用户确认 A），正常/短拣/非法跳转链路已接入（`cf352ee`）。 |
| AR-06 | 已完成 | PostgreSQL-only、直接访问 baseline=0，补充同键并发真实 PostgreSQL 证据（`567fb98`）。 |
| AR-07 | 已完成 | 客户平台真实 E2E 使用自动回收的 `_e2e` 数据库，4/4 通过。 |
| AR-08A | 已完成 | 采用 fail-closed 菜单回退（用户确认 A）；真实 H1 Playwright 已覆盖发布菜单、低权限/无可用菜单、角色权限、会话和 API Key，5/5 通过并生成真实截图；同时修正 H1 配置误收集 M-DI/M-TE 用例的问题（本轮提交）。 |
| AR-08B | 已完成 | M4 模型/组件循环已消除，构建与真实 E2E 通过（`2149b85`）。 |
| AR-09 | 实现/服务级验证完成，隔离 Compose smoke 待运行环境 | 用户已确认“API 与打印解耦”；Compose 配置检查、Render Worker 单测、H9 PostgreSQL 失败持久化与同键重试测试通过（`de7160f`）。`ea93dfe` 补齐了 `wms-api-e2e` 隔离种子、JWT、组套创建/截单和唯一项目清理，脚本不再要求现场 staging token 或占位实例 URL。2026-08-01 首次借助 sudo 包装 Docker 时，外部包装清除了 `COMPOSE_PROJECT_NAME`，导致清理命中默认 staging；该次未完成 smoke，脚本随后改为临时 env 文件并显式 `-p`，正常调用不再依赖环境变量传递。修正后的实跑在拉取缺失的 MinIO 镜像前失败：Docker 代理 `192.168.124.5:7890` 不可达；因此 worker 健康、真实 HTTP 失败码和恢复重试仍未取得运行证据，AR-09 不关闭。owner：部署/基础设施负责人；恢复条件：提供可用的 MinIO 与 `mc` 镜像或 registry 代理后，重新运行隔离 Compose smoke。 |
| AR-10 | 本地闭环完成，外部项 deferred | H5 真实 HTTP/PostgreSQL E2E、截图和质量矩阵纠偏已通过（`dd699b8`）；承运商/PDA/Print Agent/硬件证据继续 deferred。 |
| AR-11 | 技术闭环完成，范围确认待补 | 新增数字 `include!` 门禁与单据编号语义模块拆分已通过（`2948bd2`、`eb4f770`）；门禁精确范围尚未单独记录项目主人确认。owner：项目主人；恢复条件：确认 A（仅新增生产数字/共享片段）或 B（所有生产 `include!`）后，再回写验收项。 |
| AR-12 | 环境漂移已确认，备份/运行证据待补 | 只读盘点确认 local/test 为 32 条、dev-h2 为 4 条、staging 为 5 条 migration，均落后于仓库当前 114 条；目录当前为 193 张静态表，目录生成链、空库测试、“首版前保留现链”ADR 和备份/可丢弃/证据依赖盘点已完成（`9a97537`、`83419d4`、`5a6c7fc`）；实际备份、同链迁移和运行证据仍需现场材料。owner：部署/发布负责人；恢复条件：完成迁移前备份与审批，按 ADR-0045 补齐 dev/staging 同一 migration 链并重采集运行证据。 |
| KG-01A | validator 与 schema/更新命令已闭环，官方图谱产物待补 | 用户已确认采用 A（上游 Understand-Anything 生成器）；validator 现在校验 `traceability` schema `1.0` 与固定 canonical ID scheme；运行手册已记录官方 `/understand --full --language zh --auto-update` 入口和刷新后门禁；仍需由官方 Understand 刷新 `.ua`。 owner：图谱维护负责人；恢复条件：按已确认 A 方案运行官方 Understand，单独提交 `.ua`，并通过 traceability/freshness 门禁。 |
| KG-01B | blocked | 等待既有 G2 的 module manifest 补齐，不猜测业务域拓扑。另发现 G2 范围不一致：`check_bounded_contexts.py`/ADR-0012 以 24 个 BC 为目标，而 `architecture-dependencies.md` §1 还列出 H-INT/H-FILE/H-APV/H-SCH 等新增能力，G2 文字又写“29 个模块”；不能在范围未统一前批量生成 manifest。 owner：架构/域模型负责人；恢复条件：项目主人确认 manifest 清单范围，完成 G2 module manifest，基于 manifest、依赖文档和实际调用重建 H1/H2/H3 横向关系，并通过 domain validator 与 Dashboard 抽查。 |
| KG-02 | B 方案与规则闭环完成，图谱产物待补 | B 方案（源提交 + 输入指纹）已实现并有正反测试（`9a95416`）；当前 `.ua/meta.json` 仍是旧字段，需官方刷新。 owner：图谱维护负责人；恢复条件：先提交输入变更，再按 B 方案运行官方 Understand，单独提交 `.ua`，并通过 freshness 门禁。 |
| REDIS-01 | 决策方向完成，后续迁移 blocked | 已确认不新增 Redis；鉴权替代、性能/多实例和 successor ADR 另立任务，当前不删除 Redis（`d1109a3`）。 |

图谱刷新受 `.ua/.understandignore` 存在时的 Understand-Anything 确认门禁约束；未获确认前不得手工改写 `.ua`。

## 4. 任务与验收标准

### AR-01 确认鉴权存储故障时的高风险写操作策略

**问题**

[ADR-0024](../adr/0024-auth-model.md) 已接受 Redis 故障时跳过 token 撤销和权限变更检查，
允许最长一小时的失效窗口；生产组合使用同一 fail-open 策略。该取舍保障仓内不停服，
但没有区分普通读取与角色变更、双人复核、库存质量状态、印章发布等高风险写操作。

**最小范围**

- 保留普通作业可用性目标。
- 只决定 Redis 撤销存储异常时高风险写操作采用 fail-open、fail-closed，还是 PostgreSQL
  回查；不顺带重写 JWT 或引入新鉴权框架。

**验收标准**

- [x] 项目主人和安全/业务负责人确认候选方案及适用 operation 清单。
- [x] 若故障语义改变，新增 ADR 并明确读取、普通写入、高风险写入的故障行为；新 ADR
      Accepted 后局部取代 ADR-0024 §2.3.1，ADR-0024 其余鉴权决策继续有效。若确认维持
      现状，则记录有批准人的复核结论，不制造无变化 ADR。
- [x] ADR 索引、H1 用户故事、错误码和运行手册按最终决策同步。
- [x] 明确 PostgreSQL 回查失败、Redis 抖动、恢复后的行为和告警级别。
- [x] 测试分别覆盖 Redis 正常、撤销命中、Redis 不可用和恢复四种路径。
- [x] 若继续全量 fail-open，记录最长窗口、Redis HA 现状和正式发布阻断条件，不把风险写成已消除。
- [x] 新增策略矩阵测试，覆盖 operation 风险级别 × Redis 正常/撤销命中/不可用/恢复；
      不能用现有单个 fail-open 测试代替。

**最小验证**

```bash
cargo test --manifest-path backend/Cargo.toml -p wms-api --lib \
  auth::tests::auth_runtime_degrades_open_when_redis_revocation_store_is_unavailable -- --exact
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test auth_session_postgres -- --test-threads=1
# 本任务新增后必须可运行：
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test auth_runtime_policy_matrix -- --test-threads=1
```

**停止条件**

没有人工确认不得修改故障语义。

### AR-02 分离传输无关的操作人/货主上下文

**问题**

`backend/crates/api/src/auth.rs` 同时包含 `Claims/AuthContext`、Axum extractor、Redis、JWT、
环境变量和 HTTP 错误映射。service 和 repository 直接依赖该 runtime 模块，违反
[分层设计规范](../layered-design.md) 的依赖方向。

**最小范围**

- 在现有 crate 内提取一个传输无关的 `ActorContext` 或 `OperationContext` 值对象。
- handler 完成 `AuthContext -> OperationContext` 转换；不为了一个值对象新增 crate、工厂或接口。

**验收标准**

- [x] 新上下文只包含用例真正需要的操作人、货主、权限/仓库范围，不依赖 Axum、Redis、HTTP 或环境变量。
- [x] service 和 repository 不再导入包含 Axum/Redis 的 `crate::auth`。
- [x] service 可以消费已解析的权限和仓库范围；repository 只接收 `owner_id`、已解析 warehouse IDs
      或聚焦查询条件，不解释权限码。
- [x] JWT 验签、撤销存储、extractor 和 HTTP 错误映射仍留在 runtime/adapter 边界。
- [x] 货主隔离和审计 actor 语义不变，并有纯上下文单元测试。
- [x] 扩展 `check_layer_dependency.py`，用最小 fixture 证明 service/repository 反向依赖会失败。
- [x] 保持 ADR-0024 的鉴权语义不变，在 `layered-design.md` 和相关设计文档补充
      `AuthContext -> OperationContext` 转换边界；若需要改变 ADR-0024 已锁定决策，则另走替代 ADR。

**最小验证**

```bash
python3 scripts/governance/check_layer_dependency.py --json
cargo test --manifest-path backend/Cargo.toml -p wms-api --lib auth
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test auth_session_postgres -- --test-threads=1
```

### AR-03 修复 M4 截断数据上的客户端查询

**问题**

管理端先请求默认最多 50 条出库单、波次或退货，再在浏览器内按关键字、状态和日期过滤。
页面还宣称关键字支持商品、批号和客商，但订单后端 `q` 当前只查询 WMS/ERP 单号。目标单据
位于第 51 条以后时，页面会错误显示“查不到”。

**最小范围**

- 第一切片先处理订单：复用 OpenAPI 已有的单值 `q/status/limit`，让 applied query 进入
  query key 和请求参数；状态控件先改为单选，除非先确认并扩展多状态契约。
- 关键字 placeholder 只声明后端真实支持的 WMS/ERP 单号。商品、批号、客商、日期等条件
  在契约补齐前删除或禁用。
- 波次和退货模式只展示各自服务端真实支持的条件；未接服务端的共享筛选不得继续显示。
- 当前不强制新增 cursor。结果数达到 `limit` 时明确提示“当前窗口可能未完整，请收窄条件”，
  不显示虚构总数；真实分页另按需求立项。

**验收标准**

- [x] 前端不再把服务端返回的有限窗口冒充完整数据集。
- [x] 订单、波次、退货三种模式的每个可见查询条件都有对应服务端参数；否则该条件不显示。
- [x] query key 包含已应用的服务端查询条件；修改未应用表单不会提前请求。
- [x] 后端测试创建至少 51 条单据，目标位于默认窗口外，使用 `q/status/limit` 仍能命中。
- [x] 真实 M4 Playwright 使用 HTTP/PostgreSQL 复现默认窗口外订单，并显示按关键字命中的目标单据。
- [x] 返回数量达到 `limit` 时显示固定窗口提示；空结果和请求错误不能混为一类。
- [x] OpenAPI、生成 schema 和前端构建保持同步。

**最小验证**

```bash
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test wave4_postgres -- --test-threads=1
just openapi-check
pnpm --dir apps/web-admin run build
pnpm --dir apps/web-admin run test:e2e:m4-real
```

### AR-04 建立 M4 发运应用服务边界

**问题**

当前 handler 直接持有具体 `PgWave4Repository`；发运 repository 方法同时承担状态判断、
库存扣减、客户平台投影、ERP outbox、幂等、审计和事务编排。

**最小范围**

只先迁移 `ship_outbound_order` 一条关键用例，不批量拆全部 Wave4，也不立即拆 crate。

**本轮实现边界**

`Wave4ShippingService` 负责发运用例的上下文、审计意图和幂等输入，handler 不再直接调用
具体仓储；`ShipOutboundOrderPort` 由 `PgWave4Repository` 实现并继续持有现有单 PostgreSQL
事务编排。这样先建立可替换的应用服务边界，不把 SQLx transaction 泄露到 service，也不在
AR-06 前重写既有幂等身份和冲突语义。

**验收标准**

- [x] handler 只提取请求/上下文并调用发运 application service。
- [x] service 持有用例边界和审计意图；聚焦的 repository port 在一个 PostgreSQL 事务中完成
      状态、库存、outbox、幂等和审计持久化，service 不直接依赖 SQLx transaction 类型。
- [x] service 不依赖 Axum 类型，只依赖 AR-02 的操作上下文和聚焦 repository 端口。
- [x] AR-06 尚未明确时原样保留 Wave4 现有幂等身份和冲突行为，不在本任务发明共享接口。
- [x] 发运成功时业务写入、库存扣减、outbox 和审计保持原子；任一步失败全部回滚。
- [x] 客户平台和 ERP 的外部投递仍走 outbox，不在数据库事务中发网络请求。
- [x] 原有成功、重复请求、库存不足、非法状态和跨货主测试全部保持通过。

**最小验证**

```bash
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test wave4_postgres outbound_complete_pick_review_ship_replays_and_deducts_inventory \
  -- --exact --test-threads=1
# 本任务新增后必须可运行，覆盖 service 编排与失败注入回滚：
cargo test --manifest-path backend/Cargo.toml -p wms-api --lib wave4_shipping_service
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test wave4_postgres outbound_ship_rolls_back_all_side_effects \
  -- --exact --test-threads=1
# 只作完成状态补充，不替代上述行为测试：
just wave-4-complete-check
```

**本轮证据**

- `Wave4ShippingService` 单元测试：service 构造审计并委托 port。
- `outbound_complete_pick_review_ship_replays_and_deducts_inventory`：真实 PostgreSQL
  成功、重复请求和库存扣减经 service 路径通过。
- `outbound_ship_rolls_back_all_side_effects`：在发运写入晚期制造唯一键冲突，订单状态、库存、
  outbox、幂等记录和发运审计均保持回滚。

### AR-05 让一条 M4 关键链路真正复用 H6 状态规则

**问题**

[US-H6-001](../domain/user-stories-h6-state-machine.md) 只承诺定义查询和转换校验，明确没有接管
业务状态写入。M4 实际持久化 `picked/picked_short/reviewed_short` 等状态，但 H6 注册表没有
完整表达这些状态，二者已经漂移。

**最小范围**

- 先确认 M4 实际状态集合和短拣语义。
- 首个执行切片固定采用 US-H6-001 已预留的“共享纯转换规则 + M4 事务执行”；
  只接入“拣货完成 -> 复核完成 -> 发运”，不建设可编辑状态机平台。

**验收标准**

- [x] 新增或修订用户故事，明确本切片由 H6 提供共享纯状态规则、M4 service 负责事务执行；
      若要改为 H6 唯一执行器，另建 P2 故事/ADR 和独立验收。
- [x] 业务负责人确认 M4 状态、事件、初始态、终态和短拣分支。
- [x] H6 定义与 M4 实际持久化状态一致，不再存在注册表未知状态。
- [x] M4 service 在进程内调用同一纯转换函数，不通过 HTTP 回调自身。
- [x] 非法跳转在写数据库前失败；合法跳转的业务状态、H2 审计和领域事件同事务完成。
- [x] 测试覆盖完整正常链、短拣链、非法跳转、重复请求和事务回滚。
- [x] RTM 和质量矩阵不再把“定义可读”写成“业务执行已接入”。

**最小验证**

```bash
cargo test --manifest-path backend/Cargo.toml -p wms-api state_machine
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test h6_state_machine_postgres -- --test-threads=1
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test wave4_postgres -- --test-threads=1
# 本任务新增后必须可运行，直接验证 H6 定义与 Wave4 持久化链路：
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test h6_wave4_state_contract_postgres -- --test-threads=1
```

**停止条件**

状态集合和 H6 权威边界未确认时，只能补测试暴露漂移，不能改变状态语义。

### AR-06 收敛 L11 幂等语义和重复实现

**问题**

多个模块共享 `idempotency_request` 表，却复制了不同的重放、锁、过期和成功写入实现。
现有门禁只能证明“存在幂等测试证据”，不能证明各实现语义一致。
[ADR-0034](../adr/0034-wave-3-operational-postgres-schema.md) 已接受
`unique(owner_id, idempotency_key)`；[ADR-0018](../adr/0018-resilience-engineering.md) 和
[编码规范 §3.3](../coding-standards.md) 已接受前端 UUID v4、Redis → PostgreSQL
降级与 24 小时 TTL。当前后端实现却以 PostgreSQL `idempotency_request` 为权威，没有
Redis-first 幂等路径；这是待确认的设计偏差，不能在普通重构中夹带一套双存储协议。

**最小范围**

- 保持 ADR-0034 的幂等身份，先补齐共享语义测试，再提取现有 PostgreSQL 权威实现；
  不建设通用分布式工作流框架。
- 第一切片只迁移两个行为不同的代表模块并建立可收缩 baseline；后续按聚合分批迁移，
  不用一次大改关闭全部重复实现。
- H9 Print Agent 机器协议继续遵守其独立幂等记录决策，不顺带并入。

**验收标准**

- [x] 复核并保持 `(owner_id, idempotency_key)` 唯一身份；method、path 和 request hash
      用于冲突判断，不改变唯一键。
- [x] 项目主人确认存储权威：接受 PostgreSQL-only，已新增 ADR-0044 局部取代 ADR-0018 并同步
      编码规范 §3.3；若维持 Redis → PostgreSQL，则 AR-06 分独立实现切片明确权威、双写失败、
      恢复和并发语义。该分支通过 Redis 集成测试前，AR-06 不得关闭。
- [x] PostgreSQL 路径按 24 小时 `expires_at` 清理；客户端生成 UUID v4，自动/人工重试
      复用原 key，并有最小前端自检。
- [x] 已明确同键同载荷、同键异载荷、跨 operation 复用、成功和过期请求的结果；处理中、失败和并发
      语义仍由后续 L6/L11 切片补证据。
- [x] 数据库唯一约束与 ADR-0034 一致。
- [x] 共享实现覆盖锁定、回放、冲突、过期和结果保存；任务类型与参数映射不再复制 SQL/锁算法。
- [x] 首切片迁移两个行为不同的现有模块；管理菜单、库存状态配置、系统字典、任务引擎、对账、质量联系单、报损报溢与双人策略模块追加迁移并将 baseline 从 31 收缩至 23；
      其余直接访问已登记 baseline，每次迁移后只能收缩，
      AR-06 仅在 baseline 归零时关闭。
- [x] 静态门禁在 baseline 归零前阻止新增直接表访问或同类 SQL/锁实现，归零后禁止共享模块外
      直接读写 `idempotency_request`；不能只按 helper 名称匹配。
- [x] L11 真实 PostgreSQL 测试证明回放、冲突、过期和结果一致；L6 并发证据待后续切片补齐。

**最小验证**

```bash
python3 scripts/governance/check_idempotency_test.py --json
python3 -m pytest scripts/governance/tests/test_audit_idempotency_gates.py -q
pnpm --dir apps/web-admin run test:self-checks
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test wave3_billing_contract_idempotency_postgres -- --test-threads=1
# 本任务新增后必须可运行，现有文本证据门禁不能替代：
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test shared_idempotency_postgres -- --test-threads=1
```

**停止条件**

存储权威未确认时，只能补 PostgreSQL 行为测试和 no-new baseline 门禁，不能新增 Redis 双存储；
若测试证明 ADR-0034 的唯一身份无法满足真实业务，停止实现并新增替代 ADR。替代 ADR Accepted
前不得修改唯一约束。

### AR-07 恢复生产前端 OpenAPI 客户端唯一入口

**问题**

M3 feature 存在多处裸 `fetch` 和手写响应类型；独立客户平台已有正式 OpenAPI，却没有生成
TypeScript schema，仍维护手写 DTO，adapter 的字符串 path 也未受生成 `paths` 类型约束。

**最小范围**

- Web 管理端复用现有 `@wms/api-client`。
- 客户平台从自己的 OpenAPI 生成类型，保留现有复用 `@wms/api-client/requestJson` 的轻量
  transport adapter；不新增第二套请求框架。

**验收标准**

- [x] `apps/web-admin/src` 的 M3 请求全部通过 feature -> `@wms/api-client`。
- [x] 删除与生成 schema 重复的 M3 手写 DTO，类型变化可在编译期失败。
- [x] 在现有 M3 real spec 中，盘点、养护、移库各至少覆盖一条真实请求和 PostgreSQL 持久化回读；新增
      `just web-admin-m3-real-e2e` 创建一次性数据库并自动销毁。
- [x] 客户平台增加 `gen:schema` package script；`just openapi-sync` 和 `just openapi-check`
      同时覆盖主 WMS 与客户平台契约，不用主仓检查冒充客户平台检查。
- [x] 客户平台生成产物固定为 `apps/customer-portal/src/schema.ts`，只由
      `shared/openapi/customer-portal-openapi.yaml` 生成，不手工编辑。
- [x] 客户平台删除重复手写 DTO，或把 `types.ts` 收缩为生成 `components/paths` 的类型别名；
      adapter 的请求体、响应体和 path 实际引用生成 schema，禁止只生成不用。
- [x] 客户平台页面不直接拼 HTTP path；请求集中在现有 API adapter，字符串 path 受生成
      `paths` 类型约束。页面是否保留局部 query key 不作为本任务扩展范围。
- [x] 新增 `frontend.no-bare-fetch` T1 规则，禁止生产 `apps/**/src` 在明确 transport allowlist
      外使用裸 `fetch(`；`apps/**/src` 不设例外，allowlist 只允许共享生成客户端内部。
- [x] 治理检查登记 rule_id/source/path/Tier，包含正反 fixture；不以批量白名单让当前违规假绿。
- [x] 主管理端和客户平台的 typecheck、构建通过；主管理端 M3 真实 E2E 通过。
- [x] 客户平台真实 E2E 使用 `PORTAL_DATABASE_URL`，数据库名以 `_e2e` 结尾，测试自动清理数据。

**最小验证**

```bash
just openapi-check
pnpm --dir packages/api-client run typecheck
pnpm --dir apps/web-admin run build
pnpm --dir apps/web-admin run test:e2e:m3-real
pnpm --dir apps/customer-portal run build
pnpm --dir apps/customer-portal run test:e2e:real
# 本任务新增后必须可运行，并由 just openapi-sync 调用：
pnpm --dir apps/customer-portal run gen:schema
```

### AR-08A 收敛菜单、视图和故障回退边界

**问题**

管理端同时维护 `menuSections`、`defaultMenuTree`、`AdminView` 和 renderer 映射。现有
[菜单设计](../h1-menu-management-design.md) 已允许菜单 API 失败时使用内置默认菜单，
但完整本地树可能展示当前用户没有权限的入口。生产 Dashboard 还展示硬编码的
`12/5/8/3` 示例 KPI；虽然标注了“示例”，仍不应作为预发布业务数据。

**最小范围**

- 服务端发布菜单继续拥有结构、顺序、启用状态和权限事实，不在前端复制权限元数据。
- 实施前确认故障回退改为 fail-closed：只显示 Dashboard、错误和重试。当前没有服务端生成的
  view → permission 映射，不保留无法正确过滤的完整默认树。
- 不建设同时承载菜单、路由、权限和页面元数据的中央 DSL。

**验收标准**

- [x] 项目主人确认菜单故障回退方案，并同步菜单设计文档。
- [x] 菜单请求失败、返回空树、当前 view 被移除和低/无权限用户都有明确且不越权的页面状态。
- [x] `AdminView`、必要的本地 view ID、开发菜单种子和 renderer 处理集合由表驱动检查验证一致；
      该检查不复制服务端权限表。
- [x] 服务端成功返回菜单时，前端不使用本地结构覆盖其顺序、启用状态或权限结果。
- [x] 没有真实聚合接口前，Dashboard 显示未接入/不可用状态，不展示硬编码业务数量；
      不为删除示例数据顺带建设 Dashboard 平台。
- [x] 真实 H1 Playwright 覆盖正常发布菜单、低权限用户和无可用菜单场景；`pnpm --dir
      apps/web-admin run test:e2e:h1-real` 在隔离临时 PostgreSQL 与一次性 Redis 上 5/5
      通过；截图位于 `artifacts/screenshot-portal/real-web/h1-menu/`、
      `h1-role-permission/`、`h1-session/`。

**最小验证**

```bash
python3 scripts/governance/check_admin_navigation.py --json
pnpm --dir apps/web-admin run test:self-checks
pnpm --dir apps/web-admin run build
pnpm --dir apps/web-admin run test:e2e:h1-real
```

**停止条件**

菜单故障回退方案未确认时，只能补集合检查和错误复现测试，不改变现有 fallback。

### AR-08B 消除 M4 模型与组件循环依赖

**问题**

M4 存在 `Parts -> DetailDialog -> model -> Parts` 循环依赖，模型层反向导入 React 组件。

**最小范围**

只移动 OpenAPI 派生类型、状态标签和纯 helper；不与 AR-08A 菜单改造混提交，不引入页面框架。

**验收标准**

- [x] M4 DTO 派生类型和纯 helper 位于不依赖 React 组件的模型文件。
- [x] 页面、Parts、Dialog 只单向依赖模型，模型不导入任何组件。
- [x] 在现有 `test:self-checks` 增加最小 import/SCC 检查，修复前失败、修复后通过。
- [x] 构建图中不再有该三节点 SCC；没有为了消除循环增加转发模块或重复类型。

**最小验证**

```bash
pnpm --dir apps/web-admin run test:self-checks
pnpm --dir apps/web-admin run build
pnpm --dir apps/web-admin run test:e2e:m4-real
```

### AR-09 解除 Render Worker 对核心 API 启动的硬阻塞

**问题**

staging Compose 要求 H9 Render Worker 健康后才启动 WMS API。渲染是打印子能力，不应让入库、
库存、出库查询等核心 API 因渲染器故障无法启动。

**最小范围**

- 复用现有 Compose、worker 和令牌协议。
- 只解除启动硬依赖；打印任务在 worker 不可用时仍须 fail closed，不能伪造已入队或已渲染。
- Render Worker 与 Windows Print Agent 分开验收。

**验收标准**

- [x] 项目主人确认“核心 API 可启动、打印接口受控不可用”的降级边界（用户确认：API 与打印解耦）。
- [ ] worker 不健康时 API `/healthz`、`/readyz` 和一个鉴权后的非打印核心接口仍可用。
- [x] 服务层在 worker 故障时 fail-closed，持久化失败状态且同键可安全重试（H9 PostgreSQL 测试通过）。
- [ ] 真实打印 HTTP 接口返回稳定错误码 `H9_CATEGORY_PDF_RENDER_FAILED`，不创建结果不明的任务，不吞掉失败。
- [x] worker 恢复后新打印请求可成功；旧失败请求按既有幂等规则安全重试（PostgreSQL `failed_render_retries_same_instance_output_and_idempotency_key` 通过）。
- [ ] 新增一个真实 Compose smoke：停止 worker、启动 API、验证非打印路径和打印受控失败，
      再恢复 worker 验证新请求及同键安全重试。
- [x] smoke 使用唯一 `COMPOSE_PROJECT_NAME`、隔离 override、临时凭据、随机宿主端口和一次性卷，
      无论成功失败都执行 `down -v`；禁止停止、复用或清理默认 staging stack（脚本治理测试通过）。
- [x] Compose 配置检查显式注入非空测试值（`docker compose ... config --quiet` 通过）。
- [ ] worker `/healthz`、正确令牌和错误令牌均有真实 smoke。
- [x] 部署 runbook 登记该 smoke、故障恢复步骤和证据边界（脚本/文档登记测试通过）。
- [x] 不用 Render Worker E2E 替代 Print Agent 或物理打印机 S4 证据（runbook 明确边界）。

**最小验证**

```bash
env WMS_STAGING_DB_PASSWORD=test-only \
  WMS_JWT_SECRET=test-only-not-a-production-secret \
  WMS_HFILE_ACCESS_KEY=test-only \
  WMS_HFILE_SECRET_KEY=test-only \
  WMS_H9_RENDER_TOKEN=test-only \
  docker compose -f deploy/docker-compose.staging.yml config --quiet
pnpm --dir apps/h9-render-worker test
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --lib print_orchestration::render_worker::tests
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test h9_print_suite_postgres -- --test-threads=1
# 本任务新增后必须可运行：
just h9-render-worker-compose-smoke
```

### AR-10 修正 H5 与同类虚假真实 E2E/截图证据

**问题**

H5 五个故事当前登记的浏览器证据使用 `WMS_WEB_ADMIN_DEV_MOCK=1`，只验证导航、标题、固定文本
和截图，却被质量矩阵标为 verified。同类问题还涉及当前使用 `shell-dev` 的
US-H1-007、US-H2-002 和 US-H3-004，治理规则不能只修 H5 后让其他假证据继续通过。

**最小范围**

新增一套共享的 H5 real 配置和 spec，不为五个故事各建一套服务器；spec 内按
US-H5-001～005 分测试块和矩阵映射。同时纠正全部 `shell-dev` 虚假真实证据登记；
H1/H2/H3 不在本任务顺带新增业务 E2E。

**验收标准**

- [x] 新增 package script `test:e2e:h5-real`、real config 和 `*-real.spec.ts`，连接真实 WMS API
      和一次性 PostgreSQL；该命令当前不存在，是本任务交付物。
- [x] US-H5-001～005 分别建立“验收标准 -> test block -> screenshot”映射：快递商配置、
      选择规则、面单打印、下单/取消和轨迹查询不能互相冒充覆盖。
- [x] 逐故事复核 `types`、required layers 和 `not_applicable`：US-H5-003 不得漏
      `pda_runtime/hardware_runtime/offline_sync`，US-H5-004/005 不得漏 `external_runtime`；
      US-H5-005 含签收状态写入和审计，不得标成纯只读或 audit 不适用。
- [x] spec 不使用 Playwright 业务路由拦截、dev mock 或页面内存状态冒充后端。
- [x] 测试数据创建与回收自动化，重复运行不污染共享数据库。
- [x] 每个已标记完成的 H5 页面有 page/spec/screenshot 映射和真实页面 PNG。
- [x] 质量矩阵只在对应故事全部必需维度有真实证据后保留 `[[stories]]` 和 verified；
      未完整覆盖的故事移回 `[[deferred_stories]]`，填写 `reason/owner/resume_when`。
- [x] 外部承运商 dev/staging、PDA、Print Agent 或物理打印机证据不能由本地 Playwright 抵扣，
      对应故事保持 deferred/blocked。
- [x] 盘点所有 verified 且使用 `shell-dev`/dev mock 的故事；除 H5 外，US-H1-007、
      US-H2-002、US-H3-004、US-H4-001/004 只能改用已有合格证据，或移入
      `[[deferred_stories]]` 并填写 `reason/owner/resume_when`，不得伪造新证据。
- [x] 全局扩展 `check_quality_matrix.py`：校验 `e2e_checks` 命令存在、真实 spec 使用
      `*-real.spec.ts`、截图映射存在，并拒绝 `shell-dev` 或 `WMS_WEB_ADMIN_DEV_MOCK=1`
      满足真实 evidence；检查必须解析 package script -> Playwright config，不能只匹配矩阵文本。
- [x] 质量矩阵检查增加正反 fixture，不为历史假证据建立 baseline；一个共享 config 可以复用，
      但五个 H5 故事必须分别判定。

**最小验证**

```bash
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test express_postgres -- --test-threads=1
# 本任务新增后必须可运行：
pnpm --dir apps/web-admin run test:e2e:h5-real
python3 scripts/governance/check_quality_matrix.py --json
python3 scripts/governance/check_scope_gap_discovery.py --strict --module H5 --json
python3 -m pytest scripts/governance/tests/test_quality_matrix.py -q
```

### AR-11 阻止新增数字 `include!` 并改造一个代表聚合

**问题**

Wave3、Wave4 和主数据 repository 使用共享作用域的数字 `part*.rs` 与 `include!`。它们降低了
单文件行数，却没有得到 Rust 模块可见性、语义接口或编译期依赖边界。

**最小范围**

- 禁止一次性重拆 16 万行 API crate。
- 门禁只约束生产代码中新增加的 `include!("*_part*.rs")` 数字/共享作用域分片，不禁止测试
  fixture 或已经具有语义边界的 include。
- 门禁建立后，单独把 `document_numbering_repository` 作为首个代表聚合改为语义 `mod`；
  不与 AR-04、AR-05 或其他行为修改混提交。

**验收标准**

- [ ] 项目主人确认门禁精确范围，不把所有生产 `include!` 一刀切。
- [x] 新增 `backend.no-new-numeric-include-fragment` 规则，登记 source、适用路径、T2/context、
  baseline 和正反 fixture，并接入现有治理调度。
- [x] checker 只阻止 baseline 之外新增加的数字/共享作用域分片；历史项可见且只能下降。
- [x] `document_numbering_repository` 按业务语义建立私有模块，不再依赖 part1/part2/part3
  共享父作用域。
- [x] 模块仅暴露调用方需要的最小 `pub(crate)` API，不靠父模块共享全部私有符号。
- [x] 模块拆分不改变业务行为，相关 PostgreSQL 集成测试保持通过。
- [x] 其余历史分片留在 baseline，后续每个聚合使用独立任务和提交收缩，不夹带功能变更。

**最小验证**

```bash
# 本任务新增后必须可运行：
python3 scripts/governance/check_backend_module_fragments.py --json
python3 -m pytest scripts/governance/tests/test_backend_module_fragments.py -q
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test document_numbering_postgres -- --test-threads=1
```

**停止条件**

门禁范围未确认时不得把全部 `include!` 设为阻断项。

### AR-12 复核首版前 migration 基线与表所有权

**问题**

主库已有 114 个 migration；部分 migration 同时创建多个业务域的表，部分全局对齐 migration
跨域修改约 20 张既有表。当前图谱还没有完整表达 `ALTER TABLE` 依赖。该现状与 ADR-0038
“首版前只维护当前基线”的方向存在张力，但尚未证实部署故障。现有
`generate_table_catalog.py --check --json` 已返回退出码 `1`：生成器识别 173 张表，而签入目录
仍是旧结果，必须修复现有产物而不是再建第二份清单。2026-07-31 首切片已扩展生成器识别无
`IF NOT EXISTS` 的建表、ALTER/REFERENCES/RENAME/DROP 事件，并刷新目录为 193 张静态表；空库
基线、所有权决策和稳定 fingerprint 仍待补证。

**最小范围**

先修复现有 table catalog 生成链和可重建验证，再做 migration 基线决策；不直接压缩、
删除或重写 migration 历史，不包含 ADR-0014 的 Oracle 数据迁移范围。

**只读环境盘点（2026-08-01）**

未探测或修改生产数据库；通过现有 local/test、dev-h2 和 staging PostgreSQL 只读查询
`_sqlx_migrations` 与公开 schema 元数据，结果如下：

| 环境 | 已应用 migration | 最新版本 | 静态表 | schema fingerprint |
|---|---:|---|---:|---|
| local/test（5434） | 32 | `202607120007` | 85 | `6c46f6cfc7c0740da1ac2df8b25c5738` |
| dev-h2（15432） | 4 | `202606050001` | 35 | `8dd2904e8ff5389c00c60d5f2e724b4e` |
| staging（Compose 内部 PostgreSQL） | 5 | `202606060001` | 42 | `05d6d1268e4405c58ae770a86b070c69` |
| 仓库 migration 链 | 114 | `202607280001` | — | — |

该结果证明 dev/staging 当前并未跟随仓库 migration 链；盘点不等同于备份、数据可丢弃或
发布证据确认。

**备份、可丢弃和证据依赖盘点（2026-08-01）**

| 环境/资产 | 可丢弃边界 | 迁移前备份要求 | 运行证据影响 | 现场动作 |
|---|---|---|---|---|
| local/test（5434） | ADR-0045 允许按需从零重建；丢弃前仍需确认没有待保留本地数据 | 若保留数据，按 H10 重大变更前手动 `pg_dump`；从零重建不需要回填旧数据 | 不作为 dev/staging 发布证据 | 测试负责人确认是否可丢弃，必要时执行空库基线测试 |
| dev-h2（Compose） | PostgreSQL/MinIO 使用 `postgres_dev_h2_data`、`minio_dev_h2_data`；仅在 dev 证据已封存或作废后可销毁 | 应用迁移前先做数据库备份并记录 checksum；不得以空库重建替代备份 | 现有 4 条 migration 和旧 API 运行证据需作废并重采集 | 部署负责人备份、批准卷重建或按现链补迁移，再重新采集 dev 证据 |
| staging（Compose） | `postgres_staging_data`、`redis_staging_data`、`minio_staging_data` 默认不可丢弃；销毁必须先审批 | 按 H10 做迁移前全量备份、校验和恢复点记录 | 旧 schema/镜像与新链不一致，相关 staging 证据需重验 | 发布负责人完成备份、审批、补迁移和 smoke，再归档新证据 |
| AR-09 隔离 smoke | 脚本生成的一次性卷由 `down -v` 清理，不承载正式业务数据 | 使用临时凭据，不要求正式库备份 | 只生成临时 smoke 记录，不能替代 staging 证据 | 先准备可迁移 H9 fixture、token 和 worker，再执行脚本 |

盘点依据为 [H10 数据库备份与恢复](../infra/technical-specs.md)、
`deploy/docker-compose.dev-h2.yml`、`deploy/docker-compose.staging.yml` 和 ADR-0045；
本表不代表备份已执行，也不代表 dev/staging 已完成同链迁移。

**验收标准**

- [x] 扩展现有 `generate_table_catalog.py` 和测试，识别 `CREATE TABLE`（含无
      `IF NOT EXISTS`）、`ALTER/REFERENCES/RENAME/DROP`，不创建平行目录生成器。
- [x] 由同一命令刷新 `docs/database/table-catalog.md`，包含
      “表 -> 所属模块 -> 创建 migration -> 后续 ALTER migration”。
- [x] 在一次性空 PostgreSQL 上从零执行当前 migration，验证约束、索引、种子和稳定
      schema fingerprint。
- [x] 只读盘点 local/dev/staging 已应用 migration、当前 schema 和仓库链版本；不探测或改动生产数据库。
- [x] 完成可丢弃数据、备份和运行证据依赖盘点；上述环境漂移必须先按发布流程处理。
- [x] 确认首个正式版本基线建立前，是保留现链还是生成单一当前基线；形成 ADR 或明确决策记录。
- [ ] 若保留现链，仓库、dev/staging 继续使用同一 migration 链；若批准重建，开发/测试可重建，
      staging 必须先获批准、备份并作废受影响证据后销毁重建，禁止把新 baseline 直接应用到
      已有 schema 或让 staging 长期使用另一条 migration 链。
- [x] 现有生成器输出确定性的 `CREATE/ALTER/REFERENCES` 关系，供 KG-01A 消费；AR-12
      不以图谱生成能力作为自身关闭条件。

**最小验证**

```bash
python3 scripts/governance/generate_table_catalog.py --check --json
python3 -m pytest scripts/governance/tests/test_generate_table_catalog.py -q
# 本任务新增后必须可运行：
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test schema_baseline_postgres -- --test-threads=1
```

**停止条件**

没有数据库方案确认不得删除、合并或重写 migration。

### KG-01A 补齐确定性可追溯图谱关系

**问题**

当前主图有 10,583 个节点、15,446 条边，但 443 个测试资产只有 9 条 `tested_by`，223 个文档
只有 50 条 `documents`，OpenAPI、handler、service、table 和部署之间缺少可追溯边。

**最小范围**

实施前确认：A) 改进 Understand-Anything 上游生成器；或 B) 仓库增加只处理确定性关系的
轻量后处理器。建议 A；无论选哪种，仓库只保留一个 validator，不建设第二套通用图数据库。

**验收标准**

- [x] 确认实现归属采用 A：改进 Understand-Anything 上游生成器（用户确认：KG-01A=A）。
- [x] 新增 `knowledge-graph.traceability` T2 规则、validator 和正反 fixture。
- [x] 记录 schema 版本和官方更新命令；validator 校验 `traceability` schema `1.0` 与
      `<type>:<relative-path>[:symbol]`（本轮 review loop）。
- [ ] 用官方 Understand 命令刷新 `.ua` 图谱产物，并通过 traceability/freshness 门禁。
- [x] 只从 Rust/TS/Python import、OpenAPI、SQL migration、质量矩阵、RTM、Markdown 链接、
      Compose/Kubernetes 生成边；每条边保留来源定位，无法确定的关系保持未关联。
- [x] 支持 `operation -> handler -> service/repository -> table`、
      `implementation -> tested_by -> test/evidence`、文档关系和部署承载关系。
- [x] 消费 AR-12 的确定性 `CREATE/ALTER/REFERENCES` 关系；AR-12 未完成前不得关闭 ALTER 边验收。
- [x] 用当前已存在的出库查询、H9 分类 PDF、药检单下载、H2 审计四条稳定链验收；各 AR 任务
      的增量关系由各自 Review Loop 验证，不作为 KG-01A 前置。

**最小验证**

```bash
# 本任务新增后必须可运行：
python3 scripts/governance/check_knowledge_graph_traceability.py --json
python3 -m pytest scripts/governance/tests/test_knowledge_graph_traceability.py -q
```

另需调用 `understand-anything:understand` 更新主图，并用 Dashboard 抽查上述四条稳定链。
实现归属未确认时不得开始生成器改造。

### KG-01B 修正业务域拓扑

**问题与范围**

业务域图六个域恰好组成单向环，不能表达 H1/H2/H3 等横向能力的真实多对多依赖。本任务只在
既有计划 G2 补齐 module manifest 后，结合 `architecture-dependencies.md`、用户故事和实际依赖
重建域图；缺失 manifest 时不猜测。

**验收标准**

- [ ] H1、H2、H3 表现为横向依赖，H2 审计可从 M1/M2/M3/M4/H9 等真实调用方进入。
- [ ] 域关系均可回溯到 manifest、依赖文档或实际调用；生成器排序不会制造单向环。
- [ ] `understand-anything:understand-domain` 更新域图，Dashboard 抽查通过；traceability
      validator 的 domain 模式和正反 fixture 通过。

**最小验证**

```bash
# 本任务新增后必须可运行：
python3 scripts/governance/check_knowledge_graph_traceability.py --domain h2-audit --json
```

**停止条件**

G2 未完成时保持 blocked，不把 KG-01A 完成写成业务域拓扑完成。

### KG-02 确认并修正图谱新鲜度语义

**问题与候选**

当前运行手册要求 `.ua/meta.json` 的提交等于 `HEAD`；签入图谱时，图谱提交本身会改变
`HEAD`，该规则无法自洽。候选为：A) 图谱仅作不签入的 exact-HEAD 本地产物；B) 元数据记录
`sourceCommitHash` 和分析输入指纹，图谱可单独提交。建议 B，但属于治理语义变化。

**验收标准**

- [x] 项目主人确认选择 B（用户确认：KG-02=B）。
- [ ] 按 B 方案先提交输入变化，再用官方命令生成并单独提交 `.ua`。
- [x] B 的新鲜度要求 source commit 为当前提交或其祖先，且其后没有 `.ua/` 之外的分析输入
      变化；仅“提交是祖先”不能判绿。
- [x] `.ua` schema、`AGENTS.md`、图谱运行手册、hook 和新鲜度正反测试同步；旧字段不做兼容双读。

**最小验证**

```bash
# 本任务新增后必须可运行：
python3 -m pytest scripts/governance/tests/test_knowledge_graph_freshness.py -q
```

方案未确认时保留现规则，不修改 hook 或元数据语义。

## 5. 每个任务的执行 Review Loop

每个任务独立执行，默认一次只处理一个任务 ID：

1. **目标**：复制该任务的验收标准，明确本轮只关闭哪些勾选项。
2. **输入**：重新核对源码、ADR、用户故事、OpenAPI、质量矩阵和图谱新鲜度。
3. **决策门**：任务标记“待确认”时先停止，按编号方案取得用户确认。
4. **红灯**：先写能复现问题的最小失败测试或治理 fixture；确认它在修复前确实失败。
5. **最小修复**：修根因，不顺手创建框架，不扩到相邻未选任务。
6. **定向检查**：运行任务列出的最小验证；前端使用真实 HTTP/PostgreSQL 和页面截图。
7. **全局检查**：运行 `git diff --check`、`just gov-t1`，按变更范围补 `just task-check`、
   `just openapi-check` 或 Wave 出口检查。
8. **图谱反馈**：更新图谱或运行 diff 分析，确认依赖方向和影响面符合预期。
9. **复审**：检查业务语义、分层、权限、审计、幂等、测试证据和文档一致性；只修失败项。
10. **停止**：所有验收项通过则关闭；同一阻塞连续三轮仍无法解决则停止并请求决策。

任务完成时必须留下：

- 对应测试和命令退出码；
- 真实 E2E/截图或明确“不适用”的依据；
- ADR、用户故事、OpenAPI、RTM、质量矩阵的同步结果；
- 更新后的图谱基线或 diff 影响报告；
- 一次只包含该任务主题的 review 结论；满足
  [默认本地提交规则](../agent-commit-rules.md) 时提交，否则保留隔离 diff 和不提交原因。

## 6. 全计划完成标准

只有同时满足以下条件，计划才可以标记完成：

- [ ] 后继 Wave / 范围决策已批准；没有把本计划实现工作计入当前 Wave 6。
- [ ] 表中所有任务的本轮范围均已验收；Waiver 必须有批准引用和到期时间，External pending
      必须保持 blocked 并记录 owner/resume condition，二者都不能伪装成 pass。
- [ ] 没有用 baseline、dev mock、静态字符串检查或图谱推断代替安全/GSP/真实运行证据。
- [ ] 后端依赖方向与 `bin/runtime -> handler -> service -> domain/repository` 一致。
- [ ] 前端依赖方向与 `app shell -> page -> feature -> api-client` 一致。
- [ ] M4 查询、状态转换、库存扣减、审计、outbox 和幂等有真实 PostgreSQL 行为证据。
- [ ] H5 不再存在虚假 verified：本地可覆盖切片有真实 HTTP/PostgreSQL Playwright 和页面级
      截图；外部承运商、PDA、Print Agent/硬件缺口保持 deferred/blocked。
- [ ] Render Worker 故障不阻塞核心 API 启动，且打印路径仍受控失败。
- [ ] 图谱能追踪代表性的 API、数据表、测试、文档和部署链，并公开尚未解析的关系。
- [ ] `git diff --check`、`just gov-t1` 及所有任务定向检查均为退出码 `0`。

## 7. 本文档的 Review Loop

| 轮次 | 检查重点 | 结果 |
|---|---|---|
| R1 | 问题是否有源码/ADR/测试/部署证据，是否误把图谱缺边当代码缺陷 | 通过：推断已隔离到 KG 任务 |
| R2 | 每项验收是否二值、可运行、没有未经确认的业务默认值 | 通过：待确认、blocked 和分支关闭条件已拆开 |
| R3 | 与现有治理计划、分层规范、质量矩阵和 ADR 是否冲突或重复 | 通过：已移出 Wave 6，并明确唯一父状态源 |
| R4 | 文档链接、空白字符、T1 治理和最终独立复审 | 通过：三路复审无 blocker/high，T1 59/59 |
