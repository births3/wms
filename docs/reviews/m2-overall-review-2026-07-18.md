# M2 入库模块整体审查（2026-07-18）

> 文档层级：L4 评审记录
> 审查范围：US-M2-001/002/003/004/005/006/007/008/010
> 结论：**NEEDS_WORK**
> 最近复审：2026-07-18，修复后仍为 1/9 完成；详见第 7 节
> 事实源：用户故事、生产代码、OpenAPI/DTO、PostgreSQL 测试、真实 PC E2E、质量矩阵与 PDA/硬件 ADR
> 边界：本记录不修改业务需求，也不以局部通过替代完整故事验收。

## 1. 结论

M2 的 PC 标准入库链路可以运行，但模块不能声明完成。9 个故事中，`US-M2-008` 满足当前验收标准；其余 8 个故事均存在未实现验收项或缺少 S4 真实运行证据，已在 `governance/quality-matrix.toml` 恢复为 `deferred_stories`。

| 状态 | 数量 | 故事 |
|---|---:|---|
| 已验证 | 1 | US-M2-008 |
| 未完成 / 待继续 | 8 | US-M2-001/002/003/004/005/006/007/010 |
| 总计 | 9 | — |

`check_quality_matrix.py --json` 通过只说明矩阵结构和已声明证据一致；不能替代逐条验收标准的语义审查。修复完成前，`check_quality_matrix.py --complete-module M2` 必须失败。

## 2. 首次审查阻断问题（修复前基线）

本节保留首次审查时的原始问题，修复后的当前判断见第 7 节。

### P0-1 双人验收允许单个调用者代签

- 验收标准要求第一人签字后进入“待第二人签字”，再由第二人独立完成签字。
- PostgreSQL 路径只校验请求中的两个用户 ID 具备收货员角色且互不相同，没有校验第一签字人等于当前认证用户。
- PC 页面和真实 E2E 由同一管理员一次提交两个收货员 ID；现有截图只能证明两个 ID 不同，不能证明两个人分别认证和签字。
- 内存实现存在 `first_signer_id == ctx.user_id` 校验，生产 PostgreSQL 路径没有，测试替身与生产行为不一致。

证据：

- [双人验收故事](../domain/user-stories-m2-inbound-verify.md)
- [PostgreSQL 签字实现](../../backend/crates/api/src/wave3_repository_part2.rs)
- [内存签字实现](../../backend/crates/api/src/inbound.rs)
- [M2 PC 提交流程](../../apps/web-admin/src/pages/inbound/M2InboundPage.tsx)
- [M2 真实 E2E](../../prototypes/e2e/web-admin-m2-real.spec.ts)

恢复条件：第一签字人绑定当前认证主体；第二签字人通过独立认证动作签字；补待第二人状态、并发/幂等、审批和真实双人设备证据。

### P0-2 `inbound:push` API Key 权限和仓库范围过宽

- 所有 `/api/v1/inbound/*` 路径都映射到 `inbound:push`，该 scope 同时获得 `m2.read` 和 `m2.write`。
- ERP 推送 Key 因此可以访问查询、放行、收货、验收、签字、拒收、上架和作废等内部作业接口。
- 仓库范围只校验 `X-WMS-Warehouse-ID` 请求头，没有进入 `AuthContext`；同一货主下，请求头仓库与请求体/查询实际仓库不能形成强绑定。

证据：

- [API Key 鉴权中间件](../../backend/crates/api/src/api_key_auth.rs)
- [M2 入库路由](../../backend/crates/api/src/wave3_handlers/receiving_handlers.rs)
- [API Key PostgreSQL 测试](../../backend/crates/api/tests/api_key_postgres.rs)

恢复条件：把外部 Key 收敛到 ASN 推送所需的最小路由和权限；仓库范围进入认证上下文并由 repository/服务端强制使用；补跨仓库拒绝测试和真实 ERP dev/staging 证据。

## 3. 首次审查故事级记录（修复前基线）

| 故事 | 已验证切片 | 未完成验收项 | 恢复验收的最短路径 |
|---|---|---|---|
| US-M2-001 创建 ASN | 自动编号、引用校验、审计、作废入口、PC E2E | 创建后 M-VR 自动校验/状态迁移、API Key 最小权限和仓库隔离、真实 ERP 推送/回执 | 先修 API Key，再接 M-VR 自动校验，最后补 ERP S4 证据 |
| US-M2-002 收货 | 数量闭环、基础现场信息落库、PC E2E | GSP 必填服务端校验、超温/稳定性报告、收货节点双人策略、追溯码/LPN、PDA 离线 | 补 DTO/服务端规则和双人审计，再做 PDA 真机闭环 |
| US-M2-003 验收 | 批号、数量、日期、质量状态、追溯码、PostgreSQL/E2E | 外观/包装/说明书/标签未入 API，抽验/批准文号、M-QL、近效期/过期、档案补录、PDA 离线 | 先补字段契约和规则，再接 M-QL/H4/ERP，最后补 PDA 证据 |
| US-M2-004 双人签字 | M-VR 查询、角色 ID 校验、签字记录和审计 | 当前用户绑定、两次独立认证、待第二人状态、附件、真实双人/PDA/审批证据 | 先封堵代签，再拆成第一签字和第二签字两个动作 |
| US-M2-005 智能上架 | 推荐、温区/色标/容量校验、部分上架、库存原子更新、幂等审计、PC E2E | 全自动模式、LPN/整托、上架节点双人策略、M2 ERP outbox/重试、PDA 离线 | 复用现有上架事务，最小补 LPN/策略/outbox，不另造上架框架 |
| US-M2-006 异常处理 | 整单拒收、数量闭环、短少强制关闭 | 批号级部分拒收、结构化异常、稳定性报告、M-QL、H4 通知、异常统计 | 补批号级异常模型与约束，再接已有 M-QL/H4 能力 |
| US-M2-007 打印 | H9 数据聚合、PC 预览/打印记录、浏览器 E2E | PDA 蓝牙打印、PC 网络打印、离线补打、真实产物人工核对 | 使用 Wave 5 既有硬件证据流程，不新增 M2 专用打印框架 |
| US-M2-008 进度看板 | 状态聚合、筛选、异常高亮、详情、可配置自动刷新、只读边界 | 无本次阻断项 | 保持回归测试 |
| US-M2-010 上架策略 | 多方案、绑定、Top N、启停/优先级存储、同品/空库位、无库位通知开关、PC E2E | `rule_priority` 未参与运行时执行，ABC/品类/效期规则未实现，缺企业微信真实通知 | 让现有配置直接驱动现有推荐查询，避免新增第二套规则引擎 |

## 4. 治理漏检复盘

| 项目 | 记录 |
|---|---|
| 现象 | 9 个 M2 故事全部显示 `verified`，严格矩阵和范围检查均通过，但人工语义审查发现 8 个故事未满足完整验收标准。 |
| 共性 | 测试和截图只覆盖标准 PC happy path；矩阵 `types` 漏报 PDA、硬件、外部系统、库存/并发等风险，导致验收层级被手工压低。 |
| 主因 | 质量矩阵检查器验证“已声明内容是否自洽”，未验证“故事验收文本是否要求了未声明的故事类型和运行证据”。 |
| 当前事故 | 8 个故事已恢复为延期状态，并补全真实故事类型、缺口、责任人和恢复条件。 |
| 规则落点 | `docs/governance/quality-matrix-method.md` 新增“Review 驳回与类型反推”规则；M2 完成检查由延期故事硬失败。 |
| 后续自动化 | 扩展范围检查：按故事段识别 PDA/蓝牙/ERP/企业微信/库存/并发等关键词，与 `types` 交叉校验；实施前先加失败测试。 |

## 5. 验证基线

审查时执行：

| 检查 | 结果 |
|---|---|
| `python3 scripts/governance/check_quality_matrix.py --json` | 修改前通过；证明原门禁存在假阴性 |
| `python3 scripts/governance/check_quality_matrix.py --complete-module M2` | 修改前错误通过；本记录落地后应因 8 个延期故事失败 |
| `python3 scripts/governance/check_scope_gap_discovery.py --strict --module M2` | 修改前通过；仅证明故事 ID/页面范围有登记 |
| M2 真实 PC E2E | 1/1 通过，证明标准 PC happy path 可运行 |
| M2/API Key 相关 PostgreSQL 测试 | 20/20 通过，证明已覆盖切片可运行 |
| 页面规模检查 | 失败；M2 repository 触发 800 行硬门禁，两个 M2 页面触发 600 行警告 |

## 6. 推荐修复顺序

1. 封堵双人代签和 API Key 越权。
2. 补 M2-002/003 的 GSP 字段、温控和质量联系单闭环。
3. 补 M2-005 的 LPN、双人策略、全自动模式和 ERP outbox。
4. 补异常统计/通知、打印硬件和 PDA S4 证据。
5. 最后让 M2-010 的配置真实驱动推荐，并拆分超大文件。

本记录取代 [延期故事快速闭环审计（2026-07-15）](deferred-story-closure-audit-2026-07-15.md) 中对 M2 当前完成状态的旧判断；用户故事本身仍是业务事实源。

## 7. 修复后复审记录（2026-07-18）

### 7.1 复审范围与结论

复审以下修复提交：

- `8dc1175`：封堵双人代签并收紧 ASN 推送 API Key。
- `cd694d3`：双人验收改为当前用户分次签字。
- `a4b8f7f`：补收货 GSP、验收核对及不合格联络。
- `bc06195`：验收提交透传 GSP 核对字段。

结论仍为 **NEEDS_WORK**。修复推进了 US-M2-001/002/003/004/006 的局部切片，但没有新增满足完整验收标准的故事；正式口径保持 1/9 完成、8/9 延期。

| 状态 | 数量 | 故事 |
|---|---:|---|
| 已验证 | 1 | US-M2-008 |
| 部分修复、仍未完成 | 5 | US-M2-001/002/003/004/006 |
| 本轮无实质关闭进展 | 3 | US-M2-005/007/010 |

### 7.2 仍未关闭的问题

| 级别 | 问题 | 当前证据与恢复条件 |
|---|---|---|
| P0 | API Key 集成验证回归 | `inbound:push` 已收敛到 ASN 创建路由，仓库范围已进入认证上下文；但 `api_key_postgres` 仍挂载旧测试路由，7 项中 3 项返回 401 而失败。必须改为验证真实 ASN 路由，并恢复同仓成功、跨仓拒绝和审计测试。 |
| P0 | 双人签名不满足不可变要求 | 第一签名绑定当前用户、待第二人状态和独立第二次动作已经实现；但第二签名仍通过 `UPDATE receiving_inspection_signatures` 修改第一条记录，不符合 append-only。前端第二签请求还把第一、第二签名人同时传为当前用户。 |
| P1 | 冷链判定和超温处置不完整 | 冷链识别遗漏数据库允许的 `cool` 类型，且统一硬编码为 2–8℃，没有按商品储存条件处理冷冻等温区；超温只要求文本说明，未强制稳定性报告附件。 |
| P1 | 验收抽样和批准文号未闭环 | 外观、包装、标签、说明书四项已经贯通前端、API 和数据库；但前端仍提交空的抽样数量和批准文号，后端把抽样数量默认为 0，也未与商品主数据批准文号比对。 |
| P1 | 不合格联络只覆盖条件分支 | 仅在预先启用 `inbound_unqualified` 联络类型时创建 M-QL/H4 记录，否则静默跳过；M2 repository 直接写跨上下文表，尚未证明审批、真实通知、批号级部分拒收和不合格货位隔离闭环。 |
| P1 | 真实 E2E 与文件规模门禁未恢复 | M2 真实 E2E 因数据库迁移 `VersionMismatch(202607120006)` 无法启动后端；页面规模检查仍失败，M2 repository 和测试文件存在 800 行以上硬门禁，`M2InboundPage.tsx` 已达 795 行。 |

### 7.3 故事复审状态

| 故事 | 修复后状态 | 剩余关键缺口 |
|---|---|---|
| US-M2-001 | 部分修复 | API Key 集成测试回归；M-VR 自动校验/状态迁移、真实 ERP 推送/回执仍缺失 |
| US-M2-002 | 部分修复 | `cool`/冷冻温区、稳定性报告、收货节点双人策略、追溯码/LPN、PDA 真机仍缺失 |
| US-M2-003 | 部分修复 | 抽样数量、批准文号比对、近效期/过期、档案补录、完整 M-QL/H4、PDA 真机仍缺失 |
| US-M2-004 | 部分修复 | 签名 append-only、附件、审批和真实双人/PDA 证据仍缺失 |
| US-M2-005 | 未完成 | 全自动模式、LPN、双人策略、M2 ERP outbox、PDA 真机仍缺失 |
| US-M2-006 | 部分修复 | 批号级部分拒收、结构化异常、稳定性报告、可靠 M-QL/H4、统计仍缺失 |
| US-M2-007 | 未完成 | 蓝牙/网络打印、离线补打和真实打印产物证据仍缺失 |
| US-M2-008 | 已完成 | 保持回归测试 |
| US-M2-010 | 未完成 | 运行时规则优先级、ABC/品类/效期规则和真实企业微信证据仍缺失 |

### 7.4 复审验证结果

| 检查 | 结果 |
|---|---|
| `just gov-t1` | 通过，55/55 |
| `python3 scripts/governance/check_quality_matrix.py --complete-module M2` | 退出码 1；8 个延期故事，符合当前完成口径 |
| `python3 scripts/governance/check_scope_gap_discovery.py --strict --module M2` | 通过；仅证明范围已登记，不代表故事完成 |
| `cargo test --manifest-path backend/Cargo.toml -p wms-api --test api_key_postgres -- --test-threads=1` | 失败；4 通过、3 失败 |
| `cargo test --manifest-path backend/Cargo.toml -p wms-api --test m2_deferred_closeout_postgres -- --test-threads=1` | 14/14 通过 |
| `cargo test --manifest-path backend/Cargo.toml -p wms-api --test wave3_evidence_receiving_postgres -- --test-threads=1` | 1/1 通过 |
| `cargo test --manifest-path backend/Cargo.toml -p wms-api --lib receiving_` | 10/10 通过 |
| 前端自检与 `tsc --noEmit` | 通过 |
| M2 真实 E2E | 未运行完成；后端启动报 `VersionMismatch(202607120006)` |
| `python3 scripts/governance/check_page_size.py` | 退出码 1；M2 相关文件仍存在硬门禁和警告 |

本节是 7.1–7.4 复审时的状态。后续修复见第 8 节。

## 8. 针对第 7.2 节的修复轮（2026-07-18 续）

### 8.1 本轮代码修复

| 级别 | 问题 | 修复 |
|---|---|---|
| P0 | API Key 集成回归 | 测试改走真实 ASN 路径 `/api/v1/inbound/receiving-orders`；同仓成功、跨仓拒绝、作业路径拒绝；`api_key_postgres` **7/7 通过** |
| P0 | 双签 append-only | 第二签 **INSERT** 完整双签记录，禁止 UPDATE 第一条；前端第二签只声明 `second_signer_id=当前用户`；打印数据优先完整双签 |
| P1 | 冷链温区 | 识别 `cool`/`frozen`/`cold`；按储存条件取温度带（冷冻 -25~-15、cool 8~15、冷藏 2~8） |
| P1 | 抽验/批准文号 | 前端表单字段；服务端 `sampling_qty>0` 必填；批准文号与商品主数据比对 |

### 8.2 仍未关闭

| 级别 | 剩余 |
|---|---|
| P1/S4 | 超温稳定性报告**附件**强制、收货/上架双人策略、LPN/追溯码、PDA 真机 |
| P1 | 不合格 M-QL 类型未配置时的默认建单、批号级部分拒收、异常统计 |
| P1 | M2-005 全自动/ERP outbox、M2-007 打印硬件、M2-010 rule_priority 执行 |
| 门禁 | 页面/仓库 800 行规模、E2E VersionMismatch 环境问题可能仍在 |

### 8.3 验证

| 检查 | 结果 |
|---|---|
| `api_key_postgres` | 7/7 |
| `m2_deferred_closeout_postgres` | 14/14 |
| `wave3_evidence_receiving_postgres` | 1/1 |
| `inbound` 双签单测 | 通过 |

**完成口径仍为 1/9（US-M2-008）**；本轮仅消除第 7.2 节列出的 P0 与部分 P1，不宣称模块 complete。
