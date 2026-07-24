# 用户故事：H8 ERP 防腐层

> 模块：H8 erp-acl
> 性质：横向技术能力中的 ERP 专用防腐层；遵循 H-INT 通用集成契约
> 波次：连接配置先形成可实施规格；真实 ERP dev/staging 联调属于 S4 发布证据
> 依赖：M1 系统配置中心、H1 权限/API Key、H2 审计追踪、H3 OpenAPI、M-PM 参数对照、ADR-0013 配置与 secrets、ADR-0018 弹性工程、ADR-0030 H-INT 集成契约

---

## 背景与边界

H8 负责隔离 ERP 的协议、连接方式和数据格式，业务模块只调用 WMS 业务 API，不直接连接 ERP 数据库。现有接口表 Worker 和 REST 回调仅证明本地双通道切片；连接配置、消息交换、运行治理、接口表探查与真实 ERP 联调必须分别按本文件故事实现和验收。

US-H8-001 只冻结 ERP 连接的配置与安全接入规则；US-H8-002 负责 ERP 报文与 WMS 语义转换；US-H8-003 负责 WMS 侧消息日志、监控、死信和人工重放；US-H8-004 负责经已配置连接只读探查 MSSQL 接口表。四个故事都不建设通用 H-INT 连接器平台，也不把运行健康状态混入连接配置状态。

| 故事 | 看什么 |
|---|---|
| US-H8-001 | 连接配置、测通、启停 |
| US-H8-002 | 报文语义 / Worker 管线 |
| US-H8-003 | WMS 内消息日志与重放（PostgreSQL） |
| US-H8-004 | ERP 接口库接口表只读探查（MSSQL `if_*`） |

### 能力分层与通用性边界

H8 的通用性限定在 ERP 领域内：支持不同 ERP 厂商、货主、仓库、消息方向、消息类型和
REST/接口表通道。跨 ERP、快递、冷链、TMS、监管平台等外部系统的共性由 H-INT 契约
统一，H8 不承载这些系统的业务语义。

```text
业务模块 -> WMS 业务 API / H8 端口 -> H8 ERP 防腐层 -> REST / 接口表适配器 -> ERP
```

- 业务模块和 domain 只使用 WMS 命令、事件与字段，不接收 ERP DTO。
- H8 负责 ERP 路由、配置、幂等、主备切换，以及 ERP 报文与 WMS 语义的双向转换。
- 通道适配器只处理连接、协议和外部报文，不决定 WMS 业务流程。
- H8 作为 H-INT 首个参考实现；共享运行时仍按 ADR-0030 的生产证据条件延后。

---

## US-H8-001：ERP 连接配置与安全接入

**作为** 系统管理员

**我要** 在管理端「集成中心 / H8」下为每个货主管理一个或多个 ERP 连接，并限定仓库、方向、消息类型、通道和凭证引用

**以便** ERP 报文按唯一、受控、可审计的规则进入或离开 WMS

### 验收标准

1. **配置入口与存储边界**：在基础能力下提供 H8 集成中心菜单树，叶子页「H8 ERP 连接」（`view_id=h8-erp-connectors`）为正式入口；连接数据存入 H8 专用表，不塞入通用键值表。权限使用 H8 专用键 `h8.erp_connector.read` / `h8.erp_connector.write`，**不复用** `m1.config.*`。不把 ERP 连接嵌在 M1 Feature Flag 页内。
2. **多连接与仓库范围**：一个货主允许存在多个有效连接；每个连接只属于一个货主。仓库白名单为空表示适用于该货主全部仓库，非空时只允许匹配清单内仓库。
3. **受控字段**：连接至少维护货主、连接编码、连接名称、仓库白名单、方向、消息类型、通道模式、REST 地址、接口库连接信息、H1 API Key 标识、Bearer secret alias、接口库专用账号及密码 secret alias、配置状态、配置版本、首次启用时间、最近一次测试结果和时间。页面和 API 不接收、不返回明文密钥。
4. **通道模式**：允许 `rest`、`interface_table`、`rest_primary_table_fallback`。双通道模式固定为通道 A REST 主用、通道 B 接口表备用；同一业务消息禁止同时双写或双投递。
5. **路由唯一性**：运行时按“货主 + 仓库 + 方向 + 消息类型”匹配连接。同一键只能命中一个 `active` 连接；全部仓库范围与任一显式仓库范围视为重叠，显式仓库清单有交集也视为重叠，存在重叠时禁止启用。
6. **配置状态**：状态只允许 `testing`、`active`、`disabled`。新建连接进入 `testing`；连接测试成功不自动启用；只有当前配置版本测试通过后才能转为 `active`。传输失败、熔断和延迟属于运行健康，不改变配置状态。
7. **连接测试**：启用前必须验证 secret alias 可解析、REST 的 TLS/认证/健康检查或接口库的连通性/表结构/最小权限，并验证路由不重叠。测试不得写入真实业务单据；真实 ERP dev/staging 请求、回执、重试和审计证据按 S4 验收。
8. **认证边界**：ERP 调 WMS 的通道 A 入站请求复用 H1 `X-WMS-API-Key`；WMS 调 ERP 的通道 A 出站请求使用独立 Bearer secret alias；通道 B 使用独立、最小权限的接口库账号及密码 secret alias，禁止复用 WMS 业务数据库账号。
9. **API Key 权限范围**：入站按消息类型授予最小 scope；现有 ASN/商品主数据分别使用 `inbound:push`、`master-data:write`，出库订单和退货消息实现时新增并使用 `outbound:push`、`return:push`，不得授予与连接消息范围无关的 scope。
10. **降级与恢复**：通道 A 按 ADR-0018 重试和熔断；达到降级条件后，同一消息携带原 Idempotency-Key 转入通道 B。通道 A 半开探测恢复后自动回主通道；切换过程不得产生业务双投递。
11. **编辑失效规则**：`active` 连接的端点、仓库/方向/消息路由或 secret alias 发生变更时，保存后立即回到 `testing`，必须重新测试并人工启用。`disabled` 连接修改这些字段时保持 `disabled`，但当前版本的测试结果失效，重新启用前必须复测。
12. **停用与续传**：停用连接后不再认领新消息；已绑定该连接的在途消息进入暂停状态，保留连接绑定、通道阶段和 Idempotency-Key。原连接重新启用后从原阶段续传，不改投其他连接。
13. **删除限制**：仅从未启用且不存在消息或其他业务引用的连接允许物理删除；曾启用或已有业务引用的连接只能停用。删除连接配置不删除 H2 审计，审计继续保留连接标识和脱敏摘要；首次启用时间一经写入不得清空。
14. **权限与审计**：`h8.erp_connector.read` 授予系统管理员和仓库主管查看配置及测试结果；`h8.erp_connector.write` 只授予系统管理员执行新增、编辑、测试、启用、停用和删除。菜单入口 `permission_key` 使用读权限。所有新增、编辑、测试、启用、停用、删除和自动主备切换均写入 H2 append-only 审计，记录货主、连接、配置版本、旧值/新值摘要、操作者或系统主体、时间和结果，不记录明文凭据。
15. **幂等与并发**：所有写动作必须支持 Idempotency-Key 或等价业务幂等键；编辑和状态动作按配置版本做乐观并发控制，重复启用、停用或测试不得产生重复配置转换和重复审计业务事件。
16. **页面证据**：独立菜单页触发“新增菜单页”截图门禁；完成实现时必须用真实浏览器 E2E 覆盖新建、测试、启用、重叠拒绝、停用和只读权限，并在质量矩阵登记 `e2e_checks` / `e2e_screenshots`，至少归档一张测试通过后启用成功（或列表+新建确认）的真实数据截图。

### 字段与约束

| 字段 | 约束 |
|---|---|
| `owner_id` | 必填；由 H1 当前货主上下文校验，禁止跨货主读写 |
| `connector_code` | 必填；货主内唯一，启用后不可修改 |
| `connector_name` | 必填；用于配置中心展示和审计定位 |
| `warehouse_ids` | 可空数组；空表示该货主全部仓库，非空值必须属于当前货主 |
| `directions` | 必填非空集合；只允许 `inbound`、`outbound` |
| `message_types` | 必填非空集合；值来自受控 H8 消息类型，不接受自由文本 |
| `channel_mode` | 必填；只允许 `rest`、`interface_table`、`rest_primary_table_fallback` |
| `api_base_url` | 使用 REST 时必填；只允许受信任 HTTPS 地址，测试环境例外须由部署策略明确 |
| `interface_db_host/port/name/username` | 使用接口表时必填；页面按敏感配置展示，不与 WMS 业务库账号共用 |
| `api_key_id` | 通道 A 入站时必填；引用 H1 API Key，不保存 Key 明文 |
| `bearer_secret_alias` | 通道 A 出站时必填；引用 secrets 管理器 |
| `interface_db_password_alias` | 通道 B 时必填；引用 secrets 管理器 |
| `status` | `testing` / `active` / `disabled` |
| `config_version` | 每次影响运行的编辑递增，用于测试结果绑定和乐观并发 |
| `first_activated_at` | 首次启用时写入且不可清空，用于物理删除判定 |
| `last_tested_version/at/succeeded/error_summary` | 记录当前配置版本的最近测试结果；错误摘要必须脱敏 |

### 状态与动作规则

| 当前状态 | 动作 | 目标状态 | 前置条件 |
|---|---|---|---|
| 不存在 | 新建 | `testing` | 字段和货主范围校验通过 |
| `testing` | 测试 | `testing` | 写入当前版本的脱敏测试结果 |
| `testing` | 启用 | `active` | 当前版本测试成功且路由无重叠 |
| `active` | 停用 | `disabled` | 暂停在途消息并保留绑定 |
| `disabled` | 启用 | `active` | 当前版本已有成功测试且路由无重叠，否则先复测 |
| `active` | 修改端点/路由/secret alias | `testing` | 配置版本递增并使旧测试结果失效 |
| `disabled` | 修改端点/路由/secret alias | `disabled` | 配置版本递增并使旧测试结果失效 |

### 测试维度覆盖

| 维度 | 场景 |
|---|---|
| L1 单元 | 仓库范围重叠、路由键匹配、状态转换、字段条件必填、secret 脱敏 |
| L2 API 契约 | 列表/新建/修改/测试/启用/停用/删除动作与统一错误结构；OpenAPI 和 api-client 同步 |
| L3 业务流程 | 新建 → 测试 → 启用 → REST 失败降级接口表 → REST 恢复回主通道 |
| L4 错误路径 | 路由重叠、凭据不可解析、TLS 失败、接口表缺失、权限过大、配置版本冲突、非法删除 |
| L5 数据一致 | 配置、状态、首次启用时间、在途绑定和 H2 审计在同一业务事务边界保持一致 |
| L8 权限 | 系统管理员具备 `h8.erp_connector.read/write`；仓库主管只有 `h8.erp_connector.read`；跨货主和越权写入拒绝 |
| L9 安全 | 明文凭据不入库、不回显、不进入日志或审计；接口库账号最小权限 |
| L11 幂等 | 重复新建、测试、启用、停用和主备切换不产生重复业务效果 |
| S4 外部证据 | 真实 ERP dev/staging 覆盖请求、回执、重试、熔断/降级、恢复和审计引用 |

---

## US-H8-002：ERP 消息交换与 WMS 语义转换

**作为** 业务模块开发者

**我要** 通过 H8 端口接收或发送受控 ERP 消息，并把 ERP 报文转换为 WMS 命令或事件

**以便** 新增 ERP 厂商或通道时不把外部 DTO、连接协议和字段差异泄漏到业务模块

### 验收标准

1. **受控消息目录**：首批入站覆盖 ASN、出库订单、退货申请、商品主数据和商品主数据变更；首批出站覆盖入库完成、库存状态、报损报溢、档案补录、对账差异、发货确认和库存快照。首个真实对接为遵循 H8 契约的自研 ERP，12 类消息均支持 REST 主用、独立接口表备用；消息类型必须来自 H8 受控目录，不接受自由文本。
2. **三级边界**：业务模块只提交或接收 WMS canonical 命令/事件；H8 负责路由、幂等和 ERP↔WMS 语义转换；REST/接口表适配器只处理协议、连接和 ERP DTO。ERP DTO 不得进入 M1/M2/M3/M4 domain。
3. **入站链路**：入站消息依次完成 H1 认证与最小 scope、货主/仓库范围、有效连接唯一路由、幂等、`schema_version` 校验、M-PM 字段规整和 WMS 业务 API 调用；业务事务提交成功后才能返回业务成功结果，技术接收不得冒充业务成功。
4. **出站链路**：业务模块在自身事务内写入 outbox；H8 认领消息后解析唯一有效连接和通道，把 canonical 事件转换为 ERP 报文并发送；技术接收后进入 `awaiting_receipt`，业务成功回执后进入 `acked`。业务模块不得直接调用 ERP URL 或写 ERP 接口表。
5. **字段规整**：外部编码、单位、状态和自由文本在进入业务 API 前必须通过 M-PM；找不到有效映射时记录可定位的失败结果，不得把原始外部值写入受控业务字段。
6. **配置与契约版本绑定**：每次处理记录实际使用的连接、配置版本、通道、消息类型和 `schema_version`；消息开始处理后不得因连接配置变化静默切换语义。不支持的 `schema_version` 作为不可重试错误明确拒绝，禁止按最新版猜测解析。
7. **幂等语义**：入站和出站至少按货主、消息类型、外部业务标识和 Idempotency-Key 判定重复；重复请求返回原结果，不重复创建业务单据、outbox 或 H2 审计业务事件。
8. **投递与回执保证**：H8 使用“至少一次投递 + WMS 业务幂等”，不得宣称跨 WMS 与 ERP 的分布式 exactly-once。消息发送方生成 Idempotency-Key（入站 ERP、出站 WMS）；通道切换、超时重试和人工重放保持原键。技术接收后长期无业务回执时按原键重试，明确拒绝或重试耗尽进入 `dead`。
9. **错误分类**：认证、权限、报文结构、字段映射和业务校验失败为不可重试错误；网络、超时、限流和临时不可用按 ADR-0018 重试。错误响应和日志必须脱敏，并保留 correlation 标识。
10. **货主与仓库隔离**：所有消息必须携带货主上下文；仓库只能落在连接白名单和调用主体授权范围交集内，跨货主、未知仓库和歧义路由一律拒绝。
11. **审计追踪**：消息接收、转换结果、业务 API 结果、发送、回执和最终失败写入 H2 append-only 审计引用；审计只记录摘要、标识和结果，不记录明文凭据或完整敏感报文。
12. **档案补录闭环**：H8 负责投递档案补录请求并把 ERP 商品主数据变更转换为 WMS 事件；M1/M-QL/M2 负责业务校验和解除当前 ASN 的“档案补录中”状态，H8 不直接修改 ASN 或商品主数据。
13. **契约与 S4 证据**：每个消息类型至少具备 REST/接口表双通道的 L2 契约测试、L3 主流程、L4 错误路径和 L11 幂等测试；按入库、出库、主数据/档案补录、库存/财务四个闭环依次取得一个货主、一个仓库的自研 ERP dev/staging 请求、两级回执、重试和审计关联证据后启用。全部闭环完成前不得宣称故事完成。

### 消息信封最小字段

| 字段 | 约束 |
|---|---|
| `owner_id` / `warehouse_id` | 货主必填；仓库按消息类型和连接范围校验 |
| `direction` / `message_type` | 来自 H8 受控目录，用于唯一路由和最小 scope |
| `schema_version` | 必填受控版本；不支持的版本明确拒绝，不做猜测兼容 |
| `external_ref` | ERP 业务标识；与消息类型共同参与去重 |
| `idempotency_key` | 跨重试、降级和人工重放保持不变 |
| `connector_id` / `config_version` | 记录实际使用的连接和配置版本 |
| `correlation_id` | 串联 H8、业务 API、H2 审计和 ERP 回执 |
| `occurred_at` | 外部事件或 WMS canonical 事件发生时间 |
| `payload_digest` | 原始报文摘要；不得替代受控的加密报文存储策略 |
| `wms_resource_id` | 业务 API 成功后记录对应 WMS 资源标识 |

### 自研 ERP 首批交付顺序

| 闭环 | 消息 | 启用门槛 |
|---|---|---|
| 入库 | ASN 入站、入库完成出站 | 双通道 L2-L4/L11 + 一个货主/仓库 S4 |
| 出库 | 出库订单入站、发货确认出站 | 双通道 L2-L4/L11 + 一个货主/仓库 S4 |
| 主数据 / 补录 | 商品主数据、商品主数据变更入站、档案补录出站 | M-PM 映射失败路径 + 跨 M1/M-QL/M2 闭环 + S4 |
| 库存 / 财务 | 退货申请入站、库存状态、报损报溢、对账差异、库存快照出站 | 双通道 L2-L4/L11 + 一个货主/仓库 S4 |

### 测试维度覆盖

| 维度 | 场景 |
|---|---|
| L1 单元 | 消息目录、唯一路由、错误分类、配置版本绑定和 canonical 转换 |
| L2 API 契约 | 首批入站/出站消息结构、统一错误、回执和 OpenAPI 一致性 |
| L3 业务流程 | ERP 入站到业务 API；业务 outbox 到 ERP 回执 |
| L4 错误路径 | 鉴权、歧义路由、映射缺失、无效报文、超时和回执丢失 |
| L5 数据一致 | 业务事务与 outbox 原子；消息结果、资源标识和审计引用一致 |
| L8 权限 | 最小 scope、货主/仓库隔离和跨货主拒绝 |
| L9 安全 | 凭据、报文、日志和错误摘要脱敏 |
| L11 幂等 | 重复入站、重复出站、重试、降级和回执重复不产生双业务效果 |
| S4 外部证据 | 客户正式 ERP dev/staging 双向请求、回执、重试和审计关联 |

---

## US-H8-003：ERP 消息日志、监控与故障重放

**作为** 运维

**我要** 查询 ERP 消息处理记录、失败原因和通道状态，并受控重放失败或死信消息

**以便** 在不直接修改业务表、不复制消息和不泄漏敏感报文的前提下恢复集成故障

### 验收标准

1. **独立入口**：在「基础能力 / H8 集成中心」增加叶子页「H8 ERP 消息」（`view_id=h8-erp-messages`）；该页为列表型页面，使用公共 `QueryPanel` + `DataGrid`，不把消息日志塞进连接配置页。
2. **日志存储边界**：H8 使用独立消息主记录和 append-only 尝试记录，不以 H2 审计代替运行日志，也不把完整报文写入 H2；消息当前状态可按处理规则更新，H2 审计仍只能 INSERT。
3. **状态机**：消息状态只允许 `pending`、`processing`、`succeeded`、`awaiting_receipt`、`failed`、`dead`、`acked`。无需异步业务回执的处理成功进入 `succeeded`；需要业务回执的消息技术接收后进入 `awaiting_receipt`，业务成功进入 `acked`，明确拒绝或重试耗尽进入 `dead`。非法跳转、终态回退和无租约认领必须拒绝。
4. **并发认领**：Worker 认领消息必须使用数据库并发控制和带期限租约；同一消息同一时刻只能由一个 Worker 处理，租约超时可恢复，已成功或已回执消息不得再次自动认领。
5. **失败与重试**：每次尝试记录通道、开始/结束时间、脱敏错误、重试序号和结果；重试次数、间隔、熔断和死信条件复用 ADR-0018，档案补录继续执行 5 次/5 分钟/24 小时边界。
6. **死信进入条件**：不可重试错误或重试耗尽进入 `dead`；进入死信必须保留原连接、配置版本、业务 Idempotency-Key、最后错误和业务资源引用，并产生 H2 审计事件。
7. **人工重放**：只有 `failed` 或 `dead` 消息允许重放；系统管理员使用现有 `h8.erp_connector.write` 权限执行，必须填写原因并二次确认。重放复用原 Idempotency-Key，新增尝试记录，不复制新的业务消息。
8. **查询与详情**：核心查询为方向、消息类型和状态；更多查询包含连接编码、仓库、通道、外部业务标识、Idempotency-Key、correlation 标识和时间范围。详情展示处理时间线、尝试、业务资源和审计引用，报文只显示脱敏摘要。
9. **监控指标与 Worker 健康**：按货主、连接、通道和消息类型统计处理量、成功率、失败/死信数量、重试次数和 P95 延迟；计数使用消息状态变化同步维护的日统计快照，P95 使用当前过滤范围最近 10,000 次已完成尝试，禁止在请求内拉回全部尝试或临时全表聚合。同页展示 Worker 实例、版本、方向、最后心跳、当前认领数和派生健康状态，不把进程启动/停止能力放入 WMS。
10. **分区与保留**：消息按 `created_at`、尝试按 `started_at` 使用 PostgreSQL 声明式 `RANGE` 月分区；迁移创建当前月和下月，H8 维护任务每小时补齐。跨月消息 ID、业务幂等键、尝试 ID 和尝试序号由全局登记表保持唯一。系统支持按受控保留策略归档和清理；未配置保留策略时禁止自动删除。清理不得删除 H2 审计、业务单据或尚未终结的消息。
11. **权限与审计**：系统管理员可查询和重放，仓库主管使用现有 `h8.erp_connector.read` 只读查看授权仓库范围；查询详情、重放、归档和清理全部记录 H2 审计引用，跨货主和越权仓库返回拒绝。
12. **大数据量边界**：查询必须命中货主、仓库和时间分区索引，默认要求时间范围；正式基线为单货主单自然月 10,000,000 条消息且每条至少 1 条已完成尝试。在生产等价 dev/staging 的 PostgreSQL 16、关闭 dev-mock、真实鉴权和 16 并发下，预热 5 分钟后连续测量 15 分钟：30 天窗口、每页 200 条的列表 P95 ≤ 500ms，30 天统计 P95 ≤ 1s，HTTP 错误为 0。证据必须包含资源规格、分区/行数、`ANALYZE`、分区裁剪计划和原始压测日志；localhost、Mock、缩量或仅 SQL 单测不得关闭本条。不允许无界导出或把完整报文返回列表页。
13. **页面证据**：新增菜单页必须登记页面级查询配置、真实 Playwright 命令和截图映射；E2E 至少覆盖失败查询、死信详情、重放成功、重复重放无双业务效果、只读无重放按钮和跨货主拒绝。
14. **S4 故障恢复证据**：故事完成必须使用客户正式 ERP dev/staging 演练超时/失败、死信、人工重放、ERP 回执和 H2 审计关联；localhost、Mock 和容器只能证明开发切片。
15. **暂停与恢复认领**：系统管理员可按连接 + 方向设置独立持久的暂停控制，必须填写原因并可选到期时间；当前在途处理完成后不再认领新消息，人工恢复或到期后继续认领。该控制不改变连接 `active/testing/disabled` 状态，动作写 H2 审计；仓库主管只读。
16. **完整报文短期保留**：默认只保存摘要；系统管理员可按连接启用完整报文加密保留，默认 7 天、最大 30 天。详情仅授权用户按需解密并记录 H2 审计；到期删除密文，不删除消息标识、处理结果、尝试和 H2 审计。

### 页面设计契约

| 项 | 约束 |
|---|---|
| 页面类型 | 列表型 |
| 主信息载体 | 页面上方公共 `QueryPanel`；主体公共 `DataGrid` |
| 页内视图 | “消息记录”与“Worker 状态”；复用同一菜单，不新增 Worker 菜单页 |
| 核心查询 | 方向、消息类型、状态，首屏一行可见 |
| 更多查询 | 连接编码、仓库、通道、外部业务标识、Idempotency-Key、correlation 标识、时间范围，默认折叠 |
| 标准动作入口 | 页头或 DataGrid 提供刷新、字段显示和视图；首版禁止无界导出 |
| 私有动作入口 | 消息行内“详情”“重放”；Worker 状态区提供按连接 + 方向“暂停认领/恢复认领”，写动作均需原因和确认 |
| 详情展示方式 | Dialog 上下分区展示对象信息、当前状态、尝试时间线、错误摘要、业务资源和审计引用 |
| 禁止常驻区域 | 不常驻消息详情、尝试时间线、审计面板、完整报文或重放表单 |

### 消息日志最小字段

| 字段 | 约束 |
|---|---|
| `message_id` | H8 内部消息标识；人工重放不得生成新的业务消息 ID |
| `owner_id` / `warehouse_id` | 所有查询、认领、归档和清理必须显式隔离 |
| `connector_id` / `config_version` | 保留实际使用的连接和配置版本 |
| `direction` / `message_type` / `channel` | 受控值；用于分区内查询与统计 |
| `external_ref` / `wms_resource_id` | 关联 ERP 与 WMS 业务对象 |
| `idempotency_key` / `correlation_id` | 跨尝试、重放、审计和回执保持可追踪 |
| `schema_version` | 实际解析的受控消息契约版本 |
| `sync_status` | `pending/processing/succeeded/awaiting_receipt/failed/dead/acked` |
| `retry_count` / `next_retry_at` | 由 ADR-0018 策略计算；人工重放不覆盖历史尝试 |
| `last_error_summary` | 脱敏摘要，不保存 token、密码或完整敏感报文 |
| `payload_digest` | 报文摘要；始终保留，用于核对完整报文密文 |
| `encrypted_payload` / `payload_expires_at` | 按连接启用时可空；受控加密且默认 7 天、最大 30 天，到期清理密文 |
| `claimed_by` / `lease_expires_at` | Worker 并发认领和租约恢复依据 |
| `created_at` / `updated_at` / `completed_at` / `acked_at` | 支持生命周期、分区、延迟和保留策略计算 |

### 状态与动作规则

| 当前状态 | 动作 | 目标状态 | 前置条件 |
|---|---|---|---|
| `pending` | Worker 认领 | `processing` | 未被有效租约占用 |
| `processing` | 无需异步回执的处理成功 | `succeeded` | WMS 业务事务已提交 |
| `processing` | 技术接收 | `awaiting_receipt` | 消息要求异步业务回执 |
| `awaiting_receipt` | 业务成功回执 | `acked` | 回执版本、关联标识和幂等键校验通过 |
| `awaiting_receipt` | 超时重试 | `processing` | 未耗尽且按原幂等键重新投递 |
| `awaiting_receipt` | 明确拒绝或重试耗尽 | `dead` | 保留回执摘要并写 H2 审计 |
| `processing` | 可重试失败 | `failed` | 记录尝试并计算下一次重试 |
| `processing/failed` | 不可重试或耗尽 | `dead` | 记录最终错误并写 H2 审计 |
| `failed` | 到期自动重试 | `processing` | 未熔断且取得有效租约 |
| `failed/dead` | 人工重放 | `processing` | 系统管理员、原因、二次确认、原幂等键 |

### 测试维度覆盖

| 维度 | 场景 |
|---|---|
| L1 单元 | 两级回执状态转换、重试分类、暂停到期、保留保护和脱敏 |
| L2 API 契约 | 列表、详情、重放、统计与统一错误结构 |
| L3 业务流程 | 失败→重试→死信→人工重放→ERP 回执 |
| L4 错误路径 | 非法状态、重复重放、租约冲突、跨货主、无界查询和清理未终结消息 |
| L5 数据一致 | 消息状态、尝试、业务资源和 H2 审计引用一致 |
| L6 并发 | 多 Worker 竞争、租约超时恢复、暂停边界和同消息单消费者 |
| L7 性能 | 10M 消息 + 10M 完成尝试、月分区裁剪、分页稳定性、日统计快照、最近 10k 完成尝试 P95，以及列表 500ms / 统计 1s 的真实 API P95 |
| L8 权限 | 系统管理员读写、仓库主管只读、货主/仓库隔离 |
| L9 安全 | 报文摘要、错误、导出和详情脱敏 |
| L10 审计 | 重放、归档、清理和最终失败可追溯 |
| L11 幂等 | 自动重试、人工重放和重复回执不产生双业务效果 |
| S4 外部证据 | 客户正式 ERP 故障、恢复、回执与审计演练 |

---

## US-H8-004：ERP 接口表只读探查

**作为** 系统管理员（及获授权的运维）

**我要** 在管理端基于某个已配置的 H8 连接，只读查询该连接绑定的 MSSQL 接口库中的受控接口表

**以便** 在不登录 ERP 库、不依赖 DBA、不修改接口表行的前提下，确认 ERP 是否写入、行是否仍为 `pending`、Worker 是否回写 `success/failed`、出站是否被 ERP 认领

### 产品决策（已确认）

| 项 | 决策 |
|---|---|
| 权限 | 新建专用读权限 `h8.erp_interface_table.read`，不复用 `h8.erp_connector.read` |
| 入口 | 独立菜单页「H8 接口表探查」（`view_id=h8-erp-interface-tables`），不嵌在连接配置页 |
| 软件路径完成证据 | Docker MSSQL（`deploy/docker-compose.h8-erp-if.yml`）联调与 E2E 可关闭**软件路径**；不要求客户正式接口库才能关软件 AC |
| 数据库只读边界 | 探查会话必须使用与 Worker 写账号分离、仅授予 `SELECT` 的账号；Docker 软件路径必须证明 DML 被数据库拒绝 |
| 探查凭据绑定 | H8 连接扩展可空字段 `interface_probe_db_username` / `interface_probe_db_password_alias` 与独立 `interface_probe_config_version`；仅使用 004 时凭据必须成对配置，不改变现有 Worker 凭据和传输配置版本 |
| 查询契约 | 默认查询最近 7 天、最大跨度 31 天、返回准确 `total`；`sync_status` 接受逗号分隔的一个或多个精确值；限流复用 H3，外部查询超时复用 ADR-0018 |

### 与相邻故事的边界

1. **依赖 US-H8-001**：仅当连接通道模式为 `interface_table` 或 `rest_primary_table_fallback`，存在 `interface_db_host/port/name`，且探查凭据 `interface_probe_db_username` / `interface_probe_db_password_alias` 成对配置并可解析时可用；纯 `rest` 或未配置探查凭据的连接不提供数据查询（选择项灰显并说明原因）。**配置状态** `testing` / `active` / `disabled` **均可探查**（排障需要）；`disabled`/`testing` 在 UI 上可黄条提示，不得因状态拒绝只读查询。不强制「最近连接测试成功」。
2. **不等于 US-H8-003**：003 查询 WMS PostgreSQL 消息/尝试并支持重放；004 查询 ERP 接口库原始行且**只读**。详情可提供按 `idempotency_key` / `wms_resource_id` 跳转 003 的检索链接；链接目标不存在或无权限时仅提示，**不阻塞** 004。
3. **不等于通用 DB 浏览器**：禁止任意 SQL、任意 schema/表名、任意导出。
4. **无写副作用**：不在本故事内提供改 `sync_status`、手工 INSERT/DELETE 接口表行；纠错仍走 002 Worker / 003 重放 / ERP 侧作业。

### 表白名单与消息目录映射

`table_key` 等于物理表名（无自由表名）。首批与 `deploy/h8-erp-if/init` 及 US-H8-002 目录对齐；新增消息类型时必须同步扩展本表与 OpenAPI 枚举。

| `table_key`（物理表） | 方向 | 关联消息/用途摘要 | 适用过滤列 |
|---|---|---|---|
| `if_in_asn` | 入站 | ASN / 采购入库 | `external_doc_no`、`external_ref`、`warehouse_id`、`idempotency_key`、`sync_status` |
| `if_in_outbound_order` | 入站 | 出库订单 | 同上 |
| `if_in_return_order` | 入站 | 销退申请 | `external_doc_no`、`external_ref`、`warehouse_id`、`idempotency_key`、`sync_status` |
| `if_in_product_master` | 入站 | 商品主数据 | `external_doc_no`、`idempotency_key`、`sync_status`（**无** `warehouse_id`） |
| `if_in_product_change` | 入站 | 商品主数据变更 / 档案补录回写 | 同上（**无** `warehouse_id`） |
| `if_out_message` | 出站 | 通道 B 出站报文 | `source_outbox_id`、`event_type`、`idempotency_key`、`sync_status`（**无** `external_doc_no` / 通常无业务 `warehouse_id` 列） |

**`sync_status` 词表（接口表，勿与 US-H8-003 的 `succeeded` 混用）**：

| 表类型 | 允许值 |
|---|---|
| 入站 `if_in_*` | `pending`、`processing`、`success`、`failed`、`dead` |
| 出站 `if_out_message` | 上表 + `acked` |

任一 `sync_status` 对当前 `table_key` 非法（例如入站表传 `pending,acked`）必须返回 **400**，禁止静默忽略非法值。对当前表不适用的可选过滤字段（如对 `if_out_message` 传 `external_doc_no`）同样 **400**；前端按所选表隐藏不适用控件。

### 验收标准

1. **独立入口**：在「基础能力 / H8 集成中心」增加叶子页「H8 接口表探查」（`view_id=h8-erp-interface-tables`）；列表型页面，公共 `QueryPanel` + `DataGrid`。菜单 `permission_key` 使用 `h8.erp_interface_table.read`。不把探查嵌进连接配置页或消息日志页。
2. **权限、菜单与角色种子**：新增权限键 `h8.erp_interface_table.read`；权限迁移/种子注册该键；菜单 registry 登记叶子页；**系统管理员角色默认授予**；仓库主管默认不授予。仅有 `h8.erp_connector.read` **不能**访问本页 API。无写权限键（本故事无写动作）。
3. **连接选择**：必须选择当前货主上下文内、通道含接口表的连接（含 `testing`/`active`/`disabled`）；跨货主连接不可见。查询请求携带 `connector_id`，服务端校验货主与通道模式。
4. **连通与凭据**：H8 连接新增可空 `interface_probe_db_username` / `interface_probe_db_password_alias`，由 `h8.erp_connector.write` 在连接配置页维护；两者必须成对出现。004 使用这组凭据建立仅授予 `SELECT` 的查询会话（进程内连接复用/池化，禁止每请求泄漏连接），不得回退到 Worker 的 `interface_db_username` / `interface_db_password_alias`。探查读权限不得看到 alias。凭据缺失固定返回 409 `H8_PROBE_CREDENTIAL_NOT_CONFIGURED`；连接不可达或 secret 不可解析时返回脱敏错误（可含主机/库名/错误类，不含凭据）。探查凭据变更只递增独立 `interface_probe_config_version` 并写 H2 审计；不得递增传输 `config_version`、使传输测试结果失效、改变消息通道状态或迫使 `active` 回到 `testing`。
5. **表白名单**：只允许上表 `table_key`；未知键 **400**。禁止用户输入自由表名或 SQL 片段。
6. **结构化查询 API**：冻结以下路径，OpenAPI、api-client 与实现必须一致：
   - 列表 `GET /api/v1/h8/erp-interface-tables/rows`
   - 详情 `GET /api/v1/h8/erp-interface-tables/rows/{row_id}`（**query 必填** `connector_id` + `table_key`；详情身份 = `connector_id` + `table_key` + `row_id`，禁止仅靠 UUID 跨表定位）
   - 仅接受结构化过滤；服务端参数化 SELECT。契约与实现须可证明**无** `UPDATE`/`INSERT`/`DELETE` 接口表路径。
7. **核心/更多查询**：核心为连接、接口表、`sync_status`、时间范围；`sync_status` 使用公共 `QueryPanel` 多选控件，多个状态按“或”查询，API 用逗号分隔且每个值精确匹配。更多为按表适用的业务键（见映射表）。时间过滤默认落在 **`updated_at`**（便于发现长期 `pending`）；默认最近 7 天，跨度 ≤ 31 天；缺省时间窗由服务端补齐，无界查询 **400**。其他字段首版不做模糊匹配，避免全表扫描。
8. **列表字段**：至少展示主键 `id`、业务键摘要（入站 `external_doc_no` / 出站 `source_outbox_id`）、`sync_status`、`retry_count`、`last_error` 摘要、`idempotency_key`、`created_at`/`updated_at`；入站表另展示可空 `wms_resource_id`，`if_out_message` 不虚构该列。选择 `if_in_product_master` 时改用商品业务列，默认展示外部单号、货主 ID、商品编码、商品名称、规格、同步状态、重试次数和更新时间，不能只显示控制列。完整 `payload_json` 不进列表。
9. **详情**：Dialog 只展示服务端表级白名单字段和生成的 `payload_summary`；摘要按 UTF-8 截断至 4096 字节。`if_in_product_master` 详情按商品信息、药品与监管、物流与包装、同步追踪分区，覆盖商品编码/名称/规格、批准文号、剂型、生产厂家、特殊药品分类、储存条件、UDI/电子监管码、尺寸重量、契约版本和控制字段；包装层级以表格展示包装单位、基础单位换算、是否基础单位和是否默认单位。列表与详情 API 均不得返回原始 `payload_json`，包装 JSON 只允许经服务端字段白名单校验后作为 `packaging_levels` 返回，解析输入不超过 1 MiB、返回摘要不超过 4096 字节，超限只返回中文省略标记。首版不提供原文查看；确有原文排障需求时另立故事和权限。无编辑、无改 `sync_status` 或写操作控件。列表行与同键详情字段必须一致。
10. **货主与仓库隔离**：强制当前货主；表有 `owner_id` 时 SQL 必须 `owner_id = 当前货主`。表有 `warehouse_id` 时：SQL 必须落在 **调用主体授权仓库 ∩ 连接 `warehouse_ids` 白名单**（连接白名单为空表示该货主全部仓，仍受主体仓权约束）。表无 `warehouse_id` 时，只有系统管理员或具备货主全仓数据范围的主体可查询，其他主体返回 403；审计过滤摘要须标明「无仓列」。跨货主数据不得返回。
11. **分页、超时与限流**：强制分页，`page_size` ≤ 100；列表不得无界导出。单次外部查询复用 ADR-0018 超时策略，API 复用 H3 已认证端点限流，超限返回 429；不新增 H8 私有 QPS 配置。**不得**为降噪合并或省略 H2 审计事件。
12. **审计**：每次列表与详情查询写入 H2 append-only 审计，记录操作者、货主、`connector_id`、`table_key`、过滤摘要、结果行数或是否命中；不记录密码、完整 payload。
13. **账号权限（软件路径）**：Docker 和生产探查都必须使用与 Worker 写账号分离的 **SELECT-only** 专用账号。Docker 初始化需创建该账号；验收必须证明 `SELECT` 成功且 `INSERT`、`UPDATE`、`DELETE` 均被数据库拒绝，未满足时不得关闭软件路径。
14. **页面证据**：登记页面级查询配置；真实浏览器 E2E 至少覆盖：有权限选连接与表并看到列表、同时选择两个 `sync_status` 并验证并集结果、打开详情、无权限 403/隐藏入口、只读无写按钮；查询区、表头、状态与详情字段使用中文业务文案。商品主数据使用 V2 的 `DEMO-PM-001`，断言列表显示 `DEMO-P-001`、中文商品名称与规格，详情显示批准文号、厂家、储存条件、监管标识和“片/盒”包装层级，并单独留下真实截图。质量矩阵登记 `e2e_checks` / `e2e_screenshots`。
15. **软件路径完成条件**：在 `deploy/docker-compose.h8-erp-if.yml` 启动的 MSSQL 接口库上，用演示/种子行（至少断言 `external_doc_no` 如 `DEMO-ASN-001` / `DEMO-PM-001` 与 `sync_status=pending` 可见）完成 API 单测或集成测 + 真实浏览器 E2E + 权限/审计单测 + SELECT-only 账号 DML 拒绝测试 + `gov-t1` 相关门禁后，可关闭本故事**软件路径**并在验收记录标记 `SOFTWARE_PATH_PASS`。客户正式接口库只读账号联调为可选增强，**不是**本故事软件路径关闭的必要条件。
16. **非目标**：不做通用数据库浏览器；不做接口表行编辑/补单；不替代 003 监控与重放；不把 ERP DTO 放入业务 domain 聚合。

### 页面设计契约

| 项 | 约束 |
|---|---|
| 页面类型 | 列表型 |
| 主信息载体 | 页面上方公共 `QueryPanel`；主体公共 `DataGrid` |
| 核心查询 | 连接、接口表、`sync_status` 多选、时间范围（`updated_at`），首屏一行可见 |
| 更多查询 | 按所选 `table_key` 动态显示适用字段：`external_doc_no` / `external_ref` / `source_outbox_id` / `idempotency_key` / `warehouse_id` / `event_type`，默认折叠 |
| 标准动作入口 | 页头或 DataGrid 提供刷新、字段显示和视图；首版禁止无界导出 |
| 私有动作入口 | 行内「详情」；首版无「重放」「改状态」 |
| 详情展示方式 | Dialog 展示对象信息、控制列、错误摘要与服务端脱敏、限长的 payload 摘要；商品主数据按四区展示并将包装层级渲染为表格；可选跳转 003 |
| 禁止常驻区域 | 不常驻 SQL 编辑器、完整报文面板、写操作表单 |

### 查询 API 最小约束

| 入参 | 约束 |
|---|---|
| `connector_id` | 必填；当前货主且通道含接口表；配置状态不限 |
| `table_key` | 必填；受控枚举（上表白名单） |
| `row_id` | 详情必填；与 `connector_id`+`table_key` 联合定位 |
| `sync_status` | 可选；一个或多个精确状态，多个值以逗号分隔并按“或”查询；任一值不属于当前表允许值则 400 |
| `time_from` / `time_to` | 过滤列固定为 `updated_at`；默认最近 7 天；跨度 ≤ 31 天 |
| `page` / `page_size` | 分页；`page_size` ≤ 100 |
| 业务键过滤 | 仅接受当前表适用列；精确匹配；不适用列 400 |

### 探查凭据字段契约

| 字段 | 存储约束 | API 可见性 |
|---|---|---|
| `interface_probe_db_username` | `h8_erp_connectors` 可空字段；与密码 alias 成对配置 | 仅连接写配置回显用户名；004 查询响应不返回 |
| `interface_probe_db_password_alias` | `h8_erp_connectors` 可空字段；只保存 secret alias，不保存密码明文 | 连接配置只返回是否已设置；004 读权限不可见 alias |
| `interface_probe_config_version` | 非空整数，初始值 1；仅探查凭据变更时递增，用于乐观并发与审计 | 连接写配置可见；004 查询响应不返回 |

探查凭据不是查询 API 入参；服务端只能按 `connector_id` 读取上述字段。未配置探查凭据时，004 查询固定返回 **409** `H8_PROBE_CREDENTIAL_NOT_CONFIGURED`，不得回退 Worker 凭据。

| 响应 | 约束 |
|---|---|
| `items[]` | 控制列 + 可选 `payload_summary`；摘要最多 4096 UTF-8 字节，不含原始 `payload_json` |
| `total` | 返回当前过滤条件下的准确总数 |

### 测试维度覆盖

| 维度 | 场景 |
|---|---|
| L1 单元 | `table_key` 白名单、owner/仓过滤、单值/多值 `sync_status`、混入非法状态/不适用列 400、禁止 SQL 拼接 |
| L2 API 契约 | 列表/详情 OpenAPI（含 `row_id` 联合键）、统一错误、api-client 同步 |
| L3 业务流程 | Docker 种子 `DEMO-ASN-001` pending + `DEMO-ASN-002` failed → 双值多选返回并集 → 详情字段一致；`DEMO-PM-001` 列表核心商品字段与详情包装层级可见 |
| L4 错误路径 | 连接不可达、只读凭据缺失、secret 不可解析、未知表、入站+`acked`、跨货主、无仓表的仓库级主体、无界时间窗、超时、429 |
| L5 数据一致 | 同键列表与详情一致；跳转 003 失败不破坏 004 |
| L7 性能 | `page_size=100` 多表白名单 smoke；超时上限可触发 |
| L8 权限 | 持有 `h8.erp_interface_table.read` 可查；仅 connector.read 不可查；无权限、跨货主和无仓表的仓库级主体均 403 |
| 契约一致性 | OpenAPI 与生成的 api-client 类型同步；首个正式版本前不建立兼容基线 |
| L10 可观测性 | 查询耗时、超时和 429 可观测；列表/详情各产生 H2 事件（含过滤摘要） |
| 安全专项 | 密码不回显；API 与审计无原始 payload；SELECT-only 账号的 DML 被数据库拒绝 |
| 软件路径证据 | `docker compose -f deploy/docker-compose.h8-erp-if.yml` + DEMO 种子断言 + SELECT-only 否定测试 + Playwright + 单测日志 |

---

## 跨故事约束

1. **业务隔离**：M1/M2/M3/M4 等业务模块只依赖 WMS 业务 API 或 H8 端口，不读取连接表、不直连 ERP。US-H8-004 的接口表访问仅经 H8 受控只读 API，业务模块不得旁路。
2. **H-INT 契约**：H-INT 只定义跨外部系统的通用约束；H8 在 ERP 领域内复用 ADR-0018 弹性、M-PM 字段规整、ADR-0013 凭证和 H2 审计，禁止把非 ERP 业务语义或通用连接器平台塞入 H8。
3. **多货主与多仓**：所有配置、测试和运行消息都必须携带货主上下文；仓库范围只能收窄，不能跨货主扩张。
4. **审计不可篡改**：H2 审计只能追加，禁止 UPDATE/DELETE；凭据只记录 alias 和版本摘要。
5. **幂等**：入站、出站、主备切换、重试和人工状态动作使用同一业务幂等语义，通道变化不得更换业务 Idempotency-Key。
6. **完成声明**：US-H8-001/002/003 故事整体完成须达到各自 S4（客户正式 ERP dev/staging 证据）；本地 Worker、Mock 和容器证据对这三条**不可替代**正式 S4。US-H8-004 **软件路径**允许以 Docker MSSQL 接口库证据关闭，不强制客户正式接口库。
7. **日志与审计边界**：H8 消息日志（003）用于运行状态、尝试和重放；接口表探查（004）只读外部接口库行；H2 用于不可篡改审计。消息日志允许受控状态更新和按策略归档，H2 审计只允许追加。
8. **权限**：US-H8-003 首版复用 `h8.erp_connector.read/write`；US-H8-004 使用独立 `h8.erp_interface_table.read`，不因持有 connector 权限自动开通接口表探查。

## Review 记录

| 日期 | 轮次 | 结论 |
|---|---|---|
| 2026-07-19 | 首轮评审 | 发现独立故事、澄清记录、配置模型、RTM 和质量矩阵状态缺失；按用户确认的 14 项决策补齐规格。 |
| 2026-07-19 | 二轮评审 | 修正“创建即产生审计、审计引用又阻止删除”的自相矛盾；删除只受首次启用和消息/业务引用约束，H2 审计独立保留。文档级业务语义已闭合，实施与 S4 证据仍待补。 |
| 2026-07-19 | 三轮评审 | 明确 H-INT 通用契约、H8 ERP 专用防腐层和通道适配器的三级边界；修正 M5/M10/M-VR 误用 H8 及 `both` 本地联调被误读为生产双写的风险，实施与 S4 证据状态不变。 |
| 2026-07-19 | 四轮评审 | 补齐 US-H8-002 消息交换与语义转换、US-H8-003 消息日志与故障重放；覆盖技术规格与遗留分析已记录但未进入故事集的范围，两个新增故事均待实现。 |
| 2026-07-19 | 五轮评审 | 补回档案补录业务边界，统一正式 ERP S4 为故事完成必需条件；消息日志保留不硬编码 7 天，未配置受控策略时禁止自动清理。 |
| 2026-07-19 | 六轮评审 | 按新增菜单页治理补齐 US-H8-003 页面设计契约，明确核心/更多查询、DataGrid、详情/重放弹窗和禁止常驻区域；实现前不虚构页面或截图证据。 |
| 2026-07-21 | 七轮确认 | 用户确认 US-H8-004：新建权限 `h8.erp_interface_table.read`、独立菜单页 `h8-erp-interface-tables`、Docker MSSQL 可关闭软件路径；补齐只读探查故事与矩阵延期条目。 |
| 2026-07-21 | 八轮评审修复 | 按规格评审补齐：详情联合键、时间列 `updated_at`、任意配置状态可探查、表级过滤/`sync_status` 400、002 映射表、OpenAPI 路径、权限菜单种子、仓权隔离、QPS 与审计不合并、DEMO 种子断言与 L5/L7。 |
| 2026-07-21 | 九轮评审修复 | 强制探查与 Worker 账号分离并由数据库拒绝 DML；无仓列收紧为货主全仓范围；首版只返回 4096 字节脱敏摘要；复用 H3/ADR-0018；固定准确 total；修正 L9/L10 维度，并移除未实现页面的虚假矩阵接线。 |
| 2026-07-21 | 十轮评审修复 | 补齐独立探查凭据的连接绑定：新增两个可空探查字段，仅 004 查询时成对必填；明确维护权限、读权限不可见 alias、禁止回退 Worker 凭据。复审发现复用传输 `config_version` 会破坏 active 当前版本已测通不变量，改为独立 `interface_probe_config_version`；同时冻结 API 路径、固定缺配置 409，并按实际接口表结构收紧出站列。 |
| 2026-07-21 | 十一轮开发复审 | 修复 Docker 探查账号未切换到 `wms_erp_if`、系统管理员权限种子遗漏、无仓列审计摘要未标识、旧探查连接池缓存增长和 dev-mock 探查版本锁未对齐；补充非 active 连接排障提示与非法 `acked` 状态自动清理。真实 Docker/API/权限/持久化审计证据仍按 deferred 条件待执行。 |
| 2026-07-21 | 十二轮开发复审 | 修复连接选择器未区分缺少独立探查凭据：API 只返回成对配置状态，前端对不可用连接禁选并提示维护入口；QueryPanel 原生支持禁用选项。补齐 ASN/销退表 `external_ref` 查询控件及参数透传。探查连接池缓存键同时绑定探查版本与传输配置版本，避免端点变更复用旧池；同步 RTM 的已实现/待证据描述。 |
| 2026-07-21 | 十三轮开发复审 | 按“每次查询可追溯”补齐列表/详情失败与未命中也写入 H2 摘要（`hit=false`/结果 0）；接口库只读检查脚本补 DEMO-ASN-001、DEMO-PM-001 的 `pending` 断言；增加连接选择器成对凭据单测。真实 Docker/API/持久化证据仍保持 deferred。 |
| 2026-07-21 | 十四轮终审 | 对照实现与故事映射复核：补齐 ASN/销退 `external_ref` 过滤映射；详情失败显示明确错误而非无限加载；dev-mock 补齐外部引用、仓库、出站资源和 WMS 资源过滤。self-check、TypeScript、浏览器 E2E、T1 均通过；真实 Docker/API/权限/持久化审计证据继续阻塞。 |
| 2026-07-21 | 十五轮运行复审 | 复现“前端页面不显示”：dev E2E 正常，真实菜单服务读取最新发布版本，而原迁移只写 `version_no=1`。新增兼容回填迁移，将 H8 菜单子树和按钮权限补入最新版本，并加入 self-check；当前 `wms_h8_real` 已执行迁移，真实菜单接口与 9002 页面均已验证可见。其他环境需运行迁移后刷新菜单。 |
| 2026-07-21 | 十六轮运行复审 | 复现“查询无真实数据”：18180 E2E API 未挂载接口表路由且返回 404；补挂生产同款接口表路由与 H8 探查权限种子。当前 `wms_h8_real` 已配置接口表连接，MSSQL `wms_erp_if` 已准备 `DEMO-ASN-001`（`pending`），真实 API 列表/详情和 9002 页面均已返回该行。 |
| 2026-07-22 | 十七轮交互复审 | 同步状态改用公共多选控件；API 的 `sync_status` 支持逗号分隔多值和参数化 `IN`。验收模板同步增加中文业务文案与控件语义证据。 |
| 2026-07-22 | 十八轮验收复审 | 发现真实 E2E 只有 pending 行，不能证明多选并集；新增 `DEMO-ASN-002` failed 种子并断言双状态各命中一行。按 ADR-0038 重建 `wms_h8_e2e`，把探查连接写入可重复 E2E seed，并将探查字段和菜单写入逻辑收敛到当前 migration 基线。 |
| 2026-07-22 | 十九轮 ASN 入站闭环 | 新增独立 `DEMO-ASN-FLOW-001`，以真实 E2E 基础档案跑通 MSSQL 接口表→H8 Worker→M2→接口表成功回写；同一幂等键重放后单据数仍为 1，H2 生命周期审计完整。该证据只关闭 ASN 软件链路，不替代其余消息类型与客户正式 ERP S4。 |
| 2026-07-22 | 二十轮 H8-002 模板复审 | 按新版验收模板逐 AC 映射 V0–V4；纠正“纯 domain 规则已测试即等于 Worker 已接线”的误判。确认 ASN V2 切片通过，但 Worker 尚未接入 canonical/M-PM、路由与配置版本绑定、错误分类和完整逐消息证据，故事整体保持 `NEEDS_WORK`。 |
| 2026-07-22 | 二十一轮 H8-003 运行治理复审 | 按 v6 模板补齐连接/通道/消息类型统计、Worker 心跳健康、按连接+方向暂停恢复、完整报文短期加密保留；真实 PostgreSQL 证明分维度 P95、持久控制、Worker 首次登记时间、按 Key Version 解密、停用/到期清密文和预检失败 H2 审计。复审进一步补上审计持久化失败门禁、Worker lifecycle 审计失败阻断业务、解密 no-store、lifecycle OpenAPI、受控中文枚举与明确错误态；dev-mock Playwright 6 条和新增截图通过。AC10 生产 RANGE 分区与 AC14 客户正式 ERP S4 仍保持 `NEEDS_WORK`。 |
| 2026-07-23 | 二十二轮 v6 证据复审 | 修复更多查询仅前端过滤的问题，仓库、外部业务标识、幂等键、关联标识和创建时间改为服务端精确过滤；新增隔离 PostgreSQL + 关闭 dev-mock 的真实浏览器链路，覆盖高级查询、详情、重放/重复拒绝、Worker 暂停恢复、报文加密解密、只读权限和跨货主 404。修复新迁移重复创建既有索引的问题。AC11 的 JWT 仓库授权范围、AC12 稳定服务端分页/生产 P95、AC10 生产 RANGE 分区和 AC14 正式 ERP S4 仍未完成。 |
| 2026-07-23 | 二十三轮终审修复 | 创建日期按浏览器本地自然日转换为 UTC 边界，避免中国时区错查八小时；Worker 预检审计暂时不可用时不再把已认领接口表行遗留在 `processing`，而是按可重试错误释放为 `pending`。新增定向回归测试并通过。 |
| 2026-07-23 | 二十四轮稳定分页 | 消息列表改为 `created_at + id` 降序联合游标，服务端限制每页 1–200 条；同时间戳的内存与真实 PostgreSQL 测试证明跨页无重复、无漏行，管理端提供“加载更多”，关闭 dev-mock 的真实浏览器链路验证两页返回不同消息。AC12 仅剩生产约定数据量 P95 证据。 |
| 2026-07-23 | 二十五轮 Worker 错误脱敏 | 将凭据脱敏收敛到 Worker 共享错误出口；HTTP 原文、入站失败、出站认领/投递失败和心跳告警在写接口表失败摘要或输出日志前统一隐藏 Bearer、password、token、API Key 及进程已知凭据。纯逻辑测试验证明文不进入摘要；AC9 仍待各业务 API 的错误分类证据。 |
| 2026-07-23 | 二十六轮业务 API 错误分类 | 对 ASN、出库订单、商品主数据、销退申请和商品变更五类入站处理器逐一验证：503 进入可重试队列，422 直接进入不可重试路径，且两类 API 原始错误中的凭据均不进入异常摘要。AC9 软件证据关闭。 |
| 2026-07-23 | 二十七轮仓库范围复审 | `route-resolve` 对已注入 `AuthContext.warehouse_scope` 的调用默认绑定授权仓库，并对显式请求其他仓库返回 403；处理器 HTTP 测试同时覆盖默认绑定和越权拒绝。Worker 当前使用的 JWT 会话仍没有多仓范围，AC10 保持 `NEEDS_WORK`。 |
| 2026-07-23 | 二十八轮映射失败保护 | 商品主数据入站不再把未知 `storage_condition` 静默改成 `normal`；未映射值在业务 API 前以 422 不可重试错误拒绝，测试证明不会发起业务写入。完整 M-PM 规则查询和实际映射仍未接线，AC5 保持 `NEEDS_WORK`。 |
| 2026-07-23 | 二十九轮绑定不可变保护 | 生命周期端点命中既有幂等消息后，若调用方改变连接 ID、配置版本、方向、消息类型、契约版本或通道则在写审计前拒绝；HTTP 测试覆盖五类切换并证明不产生伪生命周期审计。Worker 重试仍须主动读取原绑定及历史配置，AC6 保持 `NEEDS_WORK`。 |
| 2026-07-23 | 三十轮重试原绑定读取 | 接口表行 `retry_count > 0` 时，Worker 先按货主上下文、方向、消息类型、外部标识和幂等键精确读取既有消息，并复用首次连接 ID、连接编码、配置版本和通道；已绑定消息不再调用当前路由解析，且查询覆盖超过默认 7 天窗口的人工重放。历史配置快照读取和人工重放到接口表认领链仍未完成，AC6 保持 `NEEDS_WORK`。 |
| 2026-07-23 | 三十一轮历史配置快照 | PostgreSQL 在连接创建及配置版本递增时自动保存不可变运行配置快照，并拒绝运行字段在版本号不变时静默修改；按货主、连接和版本读取的 API 已进入 OpenAPI。Worker 重试读取原消息绑定后必须加载并校验对应历史版本，连接、方向、消息类型或接口表通道不一致即拒绝。真实 PostgreSQL 与 Worker 回归通过；人工重放到接口表重新认领链仍未完成，AC6 保持 `NEEDS_WORK`。 |
| 2026-07-23 | 三十二轮人工重放桥接 | 消息列表新增按连接 ID 和人工重放标记的货主内筛选；Worker 按消息类型读取 `replay:*` 标记，以原 Idempotency-Key 将既有 MSSQL 终态行恢复为 `pending`，再调用现有消息认领 API，普通处理中租约仍不可抢占。内存、真实 PostgreSQL 与 Worker 纯逻辑回归已通过；本机 Docker socket 无访问权限，尚未取得真实 MSSQL→业务 API 运行证据，AC6 保持 `NEEDS_WORK`。 |
| 2026-07-23 | 三十三轮出站唯一路由 | 删除 Worker 的全局首条 active 连接和命令行通道覆盖；出站 outbox 认领先按绑定连接的货主、方向、消息类型和可用仓库标识收窄，再逐消息调用既有 `route-resolve`，校验返回连接仍等于 Worker 绑定，并把连接编码、配置版本和主通道写入生命周期。入库完成 outbox 同步携带仓库标识。Worker 与真实 PostgreSQL M2 outbox 回归通过；其余出站源仍需补仓库标识及双通道 L2–L4/L11，AC4 保持 `NEEDS_WORK`。 |
| 2026-07-23 | 三十四轮库存状态路由身份 | 库存状态变更在同一 PostgreSQL 事务内从批次库位解析货主内仓库，并把 `warehouse_id` 写入 outbox canonical 载荷；真实 M3 PostgreSQL 测试回读验证。历史异常批次无法解析库位时不阻断库存事务，但仓库范围 Worker 不会认领该 outbox。当前有业务生产者的入库完成、库存状态和报损报溢三类均携带仓库身份；档案补录、对账差异、发货确认和库存快照四类仍只有 outbox 基础表、没有业务生产者，不能用测试数据冒充完成，AC4 保持 `NEEDS_WORK`。 |
| 2026-07-23 | 三十五轮入站 canonical 边界 | 接口表适配器读取的五类 MSSQL 行统一转换为 `H8CanonicalInboundCommand`，连接、配置版本、幂等键、关联标识、发生时间和业务字段在 `convert` 阶段收口；业务 API handler 不再接收接口表 DTO。数量和时间在边界完成类型规整，未知储存条件和缺失销退批号以 422 类不可重试映射错误终止。Worker 全套纯逻辑回归通过，AC2 关闭；真实 M-PM 规则查询仍由 AC5 跟踪。 |
| 2026-07-23 | 三十六轮五类入站 L11 | 复用既有 M2/M4/M1 PostgreSQL 幂等机制，新增销退申请、出库订单和商品变更逐类回归，并与既有 ASN、商品主数据创建证据合并。五类均证明同一 Idempotency-Key 重放返回原资源，业务行、明细、审计及幂等记录不重复；AC7 仍等待七类出站、主备降级和业务回执重复证据。 |
| 2026-07-23 | 三十七轮已有出站生产者 L11 | 增强入库完成、库存状态、报损和报溢四条既有 PostgreSQL 回归：同键重放后业务副作用、outbox、对应 H2 审计和幂等记录均各一条，outbox 保留仓库身份。当前证明三类已有出站生产者；档案补录、对账差异、发货确认和库存快照尚无业务生产者，AC7 保持 `NEEDS_WORK`。 |
| 2026-07-23 | 三十八轮人工重放真实链路 | 修复 lifecycle 只写审计、不更新消息状态的问题；入站失败、自动重试和成功现按 domain 状态机落 `failed/processing/succeeded`。Worker 在不可重试或耗尽时先经既有 dead API 写 H8 死信与 H2，再把 MSSQL 行置 dead；H8 终态同步失败则释放接口行重试。Docker MSSQL + 当前源码 API 已证明同一消息、原 Idempotency-Key 和原连接版本完成 `dead → replay → processing → succeeded`，M2 仅一张单据。AC6 关闭；复审确认普通成功/可重试失败仍缺独立 attempt 与 `next_retry_at`，US-H8-003 AC5 更正为 `PARTIAL`；客户 ERP V4 仍未完成。 |
| 2026-07-23 | 三十九轮 Worker 尝试记录 | 生命周期仓储在 `failed/succeeded` 时与消息状态同事务追加完成尝试，记录通道、起止时间、递增序号、结果、执行者和脱敏错误；内存与真实 PostgreSQL 路径均有回归。US-H8-003 AC5 仍为 `PARTIAL`，剩余缺口收窄为按 ADR-0018 计算并持久化 `next_retry_at`。 |
| 2026-07-23 | 四十轮标准重试时间 | H8 消息失败转换按 ADR-0018 L2 的 1/2/4/8/16 秒基线与基于 Idempotency-Key 的稳定 ±20% 抖动持久化 `next_retry_at`；processing、succeeded、dead 与人工重放清空该字段。隔离 Docker MSSQL 证明未到期不认领、到期认领和人工重放立即认领。AC5 仍为 `PARTIAL`，只剩档案补录 L3 的 5 分钟/24 小时边界缺专门运行证据。 |
| 2026-07-23 | 四十一轮档案补录持久重试 | PostgreSQL 建表约束将档案补录固定为 5 次和创建后 24 小时截止，Worker 失败后固定等待 5 分钟；隔离迁移库证明立即重取为 0、第 5 次失败和到期行均进入 `dead`。同时修复 dead 仍显示未来 `next_attempt_at` 的误导字段。US-H8-003 AC5 关闭；档案补录业务生产者缺口仍由 US-H8-002 跟踪。 |
| 2026-07-23 | 四十二轮 JWT 多仓范围 | H8 消息列表、统计和详情从 `auth_user_warehouse_scopes` 加载只读 JWT 用户的多仓授权；显式查询或详情访问未授权仓库返回 403，无仓消息不向受限用户暴露。API Key 单仓范围同步约束详情、重放、认领、dead、归档、报文解密、生命周期与全仓清理，新建生命周期消息继承调用身份仓库。隔离 PostgreSQL 与 HTTP 回归通过，US-H8-003 AC11 关闭。 |
| 2026-07-23 | 四十三轮分区与容量基线 | AC10/AC12 固化为 10M 消息 + 10M 完成尝试、列表 P95 500ms、统计 P95 1s。消息/尝试切换为声明式月分区，复用当前月+下月维护模式；两张全局登记表保留跨月 ID/幂等唯一性，日统计快照和最近 10k 完成尝试消除请求内全量聚合。真实 PostgreSQL 已证明跨月路由、唯一性、裁剪、状态/重试快照和 append-only；AC10 关闭，AC12 仍等待生产等价 dev/staging 的 10M 原始压测证据。 |

## 验收记录（US-H8-004 软件切片）

- 故事：`US-H8-004`
- 验收日期：`2026-07-22`
- 质量矩阵状态：`stories`
- 证据层覆盖：`V0/V1/V2/V3=PASS`；`V4=不适用（软件路径不要求客户正式接口库）`
- 当前结论：`SOFTWARE_PATH_PASS`（Docker MSSQL 只读边界、真实 DEMO 查询、权限、H2 审计与截图证据完整）

| 验收范围 | 证据层 | 当前结果 | 证据 / 说明 |
|---|---|---|---|
| AC1–AC12（入口、权限接线、连接选择、白名单 API、查询/详情、范围、分页、脱敏、审计代码、页面交互） | V0/V1/V2 | `PASS` | 领域/API 单测、OpenAPI/api-client、权限/菜单迁移、self-check；H2 持久化失败改为查询失败，不再静默丢审计 |
| MENU-VISIBILITY（发布版本、权限、真实页面可见、刷新后复验） | V0/V3 | `PASS` | 管理员真实页面可见；仅有 `h8.erp_connector.read` 的新会话及刷新后均隐藏入口，直接 API 返回 403 |
| AC14（页面证据：列表/筛选/详情/无写控件） | V1/V3 | `PASS` | 真实 Playwright 覆盖中文字段、`pending,failed` 双值多选请求，以及 `DEMO-ASN-001` pending + `DEMO-ASN-002` failed 的并集结果、通用详情、商品详情、无写按钮、反向权限与 5 张截图 |
| UI-SEMANTICS（中文文案与控件选择） | V0/V3 | `PASS` | 查询区、表头、状态和详情字段均为中文业务文案；同步状态使用公共多选控件，API 与结果断言覆盖两个值 |
| BUSINESS-CONTENT（商品主数据业务内容） | V2/V3 | `PASS` | 同一 `DEMO-PM-001` 在 MSSQL、列表、详情和截图中可追踪；列表断言 `DEMO-P-001`、中文商品名称和规格，详情断言批准文号、厂家、储存条件、UDI 及“片/盒”包装表格；API/页面均不返回 `payload_json` |
| AC13（SELECT-only 账号与 DML 拒绝） | V2 | `PASS` | Docker 证据工具验证 `DEMO-ASN-001` pending、`DEMO-ASN-002` failed、`DEMO-PM-001` pending 可 SELECT；实际 UPDATE/DELETE/INSERT 均被 MSSQL 拒绝且无残留 |
| AC15（Docker DEMO 行列表→详情、真实权限/审计） | V2/V3 | `PASS` | `wms_h8_e2e` + `wms_erp_if` 完成列表→双状态并集→详情；PostgreSQL H2 观察到列表/详情事件、结果数/命中与脱敏过滤摘要 |

验证命令：

| 命令 | 结果 |
|---|---|
| `cargo test --manifest-path backend/Cargo.toml -p wms-domain --lib h8_erp` | `PASS`（40） |
| `cargo test --manifest-path backend/Cargo.toml -p wms-domain --test h8_erp_interface_table` | `PASS`（4） |
| `cargo test --manifest-path backend/Cargo.toml -p wms-api --lib h8_erp` | `PASS`（22） |
| `pnpm --dir apps/web-admin run test:self-checks` | `PASS` |
| `pnpm --dir apps/web-admin run test:e2e:h8-interface-dev` | `PASS`（1 条，3 张开发 E2E 截图） |
| `pnpm --dir apps/web-admin run test:e2e:h8-interface-real` | `PASS`（3 条，5 张真实数据/权限截图） |
| `sudo -n bash scripts/h8_erp_interface_sync/check_probe_readonly.sh` | `PASS`（SELECT + 3 条 DEMO 种子；UPDATE/DELETE/INSERT 拒绝；无残留） |
| `cargo test --manifest-path backend/Cargo.toml -p wms-api persistent_audit_failure_is_not_swallowed --lib` | `PASS`（H2 持久化失败不得静默吞掉） |
| `pnpm --dir apps/web-admin exec tsc --noEmit` / `pnpm --dir apps/web-admin run build` | `PASS`（仅既有依赖/产物体积警告） |
| `just openapi-check` / `python3 scripts/governance/check_quality_matrix.py --json` / `python3 scripts/governance/check_scope_gap_discovery.py --strict --module H8 --json` | `PASS` |
| `just gov-t1` | `PASS`（56/56） |

证据归档：`docs/retros/h8-interface-table-software-path-evidence.json`。US-H8-004 软件路径已关闭；客户正式接口库联调保留为可选增强，不回退故事状态。

## 验收记录

- 故事：`US-H8-001`
- 验收基线：`f890528`
- 验收层级：`S4`（`external_runtime`）
- 质量矩阵状态：`deferred_stories`
- 验收日期：`2026-07-19`
- 整体结论：`NEEDS_WORK`

### 验收命令与证据包

| ID | 命令或证据 |
|---|---|
| H8-U1 | `cargo test --manifest-path backend/Cargo.toml -p wms-domain --lib h8_erp` |
| H8-U2 | `cargo test --manifest-path backend/Cargo.toml -p wms-api --lib h8_erp_connectors` |
| H8-E2E | `just web-admin-h8-real-e2e`；`prototypes/e2e/web-admin-h8-real.spec.ts` |
| H8-LOCAL | `just h8-local-integration`；`docs/retros/h8-local-integration-evidence.json` |
| H8-S4 | `just h8-container-erp-s4-evidence`；`docs/retros/h8-container-erp-s4-evidence.json` |
| H8-FAILOVER | `docs/retros/h8-failover-runtime-evidence.json` |

| AC | 验证命令或方式 | 证据 | 结果 | 缺口 / 恢复条件 |
|---|---|---|---|---|
| AC-1 | H8-U2、H8-E2E | H8 专用菜单/权限迁移、连接页面、列表截图 | `PASS` | - |
| AC-2 | H8-U1、H8-U2、H8-E2E | 多连接、仓库范围与 route-resolve 证据 | `PASS` | - |
| AC-3 | H8-U1、H8-E2E | `secrets.rs`、Vault alias 与脱敏页面证据 | `PASS` | - |
| AC-4 | H8-U1、H8-LOCAL、H8-S4 | 三种通道模式、主备切换且不双写 | `PASS` | - |
| AC-5 | H8-U1、H8-U2、H8-E2E | 路由重叠拒绝与唯一解析截图 | `PASS` | - |
| AC-6 | H8-U1、H8-U2、H8-E2E | `testing/active/disabled` 状态转换 | `PASS` | - |
| AC-7 | H8-E2E、H8-S4 | 连接测试与容器厂商回执 | `NEEDS_WORK` | 尚无客户指定厂商正式 ERP dev/staging 回执；替换 `ERP_CALLBACK_BASE` 后归档 |
| AC-8 | H8-U2、H8-E2E | H8 专用权限、API Key、secret alias 与只读 403 | `PASS` | - |
| AC-9 | H8-U1、H8-U2 | 入站消息最小 scope 覆盖测试 | `PASS` | - |
| AC-10 | H8-LOCAL、H8-S4、H8-FAILOVER | REST 失败转接口表、半开恢复和非双投递 | `PASS` | - |
| AC-11 | H8-U1、H8-U2、H8-E2E | 编辑后回到 `testing` 截图 | `PASS` | - |
| AC-12 | H8-U1、H8-U2 | 在途消息 pause/resume 测试 | `PASS` | - |
| AC-13 | H8-U1、H8-U2 | 删除限制与引用保护测试 | `PASS` | - |
| AC-14 | H8-U2、H8-E2E | 专用读写权限、只读拒绝和 H2 审计 | `PASS` | - |
| AC-15 | H8-U1、H8-U2 | Idempotency-Key、重复动作和乐观锁测试 | `PASS` | - |
| AC-16 | H8-E2E | 真实浏览器流程和质量矩阵登记的 7 张截图 | `PASS` | - |

### 聚合验证

- 质量矩阵：`python3 scripts/governance/check_quality_matrix.py --json`
- 范围检查：`python3 scripts/governance/check_scope_gap_discovery.py --strict --module H8 --json`
- 证据索引：`governance/quality-matrix.toml` 中 `US-H8-001`

### 验收结论

- 已证明：AC1-6、AC8-16 的软件主链、真实浏览器 E2E、容器厂商回执和主备切换证据。
- 未完成：AC7 的客户指定厂商正式 ERP dev/staging 回执。
- 恢复条件：使用客户正式 `ERP_CALLBACK_BASE` 运行同一 S4 契约并归档厂商侧回执；完成前保持 `deferred_stories`，不得宣称故事整体完成。

---

## 验收记录（US-H8-002）

- 故事：`US-H8-002`
- 验收基线：本记录所在提交
- 验收层级：`S4`（`external_runtime`）
- 质量矩阵状态：`deferred_stories`
- 证据层覆盖：`V0=PASS`；`V1=NEEDS_WORK（部分覆盖）`；`V2=ASN 入站切片 PASS`；`V3=不适用（无独立用户页面）`；`V4=NEEDS_WORK`
- 验收日期：`2026-07-22`
- 整体结论：`NEEDS_WORK`

| AC | 证据层 | 验证命令或方式 | 证据 | 结果 | 缺口 / 恢复条件 |
|---|---|---|---|---|---|
| AC-1 受控消息目录 | `V0/V1` | H8-DOMAIN、H8-WORKER | `backend/crates/domain/src/h8_erp_message.rs`、`scripts/h8_erp_interface_sync/outbound_publish.py` 及目录对齐测试 | `PASS` | - |
| AC-2 三级边界 | `V0/V1` | H8-DOMAIN、H8-WORKER、H8-CANONICAL | Rust domain 定义 canonical 契约；`inbound_canonical.py` 将五类接口表 DTO 转成独立命令；lifecycle 在 `convert` 阶段调用转换器，业务 handler 仅消费 canonical 命令 | `PASS` | - |
| AC-3 入站链路 | `V1/V2` | H8-WORKER、H8-ASN-V2、H8-U2 | Worker 已在业务 API 前调用 `route-resolve` 并严格校验 `schema_version`；处理器已拒绝与 `AuthContext.warehouse_scope` 不一致的仓库；ASN V2 已跑通 | `NEEDS_WORK` | 接入 M-PM 与 JWT 用户多仓范围，并补齐其余 4 类 L2–L4/L11 |
| AC-4 出站链路 | `V1/V2` | H8-WORKER、H8-LOCAL、M2-PUTAWAY-PG、M3-STATUS-PG | Worker 不再读取全局首条连接；认领先按绑定连接货主/目录/仓库收窄，逐消息调用 `route-resolve`，校验 Worker 连接并把连接编码、配置版本和主通道写入生命周期；现有生产者中的入库完成、库存状态和报损报溢载荷均携带仓库标识 | `NEEDS_WORK` | 接通档案补录、对账差异、发货确认、库存快照四类业务生产者，并取得七类逐类双通道 L2–L4/L11 与真实运行证据 |
| AC-5 字段规整 | `V1` | H8-DOMAIN、H8-WORKER、代码路径复审 | `apply_mpm_normalize` 有 domain 单测；Worker 已在业务 API 前以 422 拒绝未知 `storage_condition`，且测试证明不发生静默 `normal` 写入 | `NEEDS_WORK` | Worker 接入实际 M-PM 规则查询与有效映射，并补真实数据回读 |
| AC-6 配置版本绑定 | `V1/V2` | H8-DOMAIN、H8-WORKER、H8-U2、H8-MSG-U3、H8-ASN-REPLAY-V2 | 首次处理写入完整绑定；生命周期端点拒绝绑定切换；PostgreSQL 自动保存不可变版本快照并拒绝无版本变更；Worker 重试读取原绑定和历史配置；Docker MSSQL + 当前源码 API 已证明同一消息和原 Idempotency-Key 从 dead 经人工确认重放、Worker 自动恢复接口行并以原连接 ID/编码/版本/通道处理成功 | `PASS` | - |
| AC-7 幂等语义 | `V1/V2` | H8-DOMAIN、H8-ASN-V2、H8-INBOUND-L11、H8-OUTBOUND-L11 | 五类入站均证明同键返回原资源且业务/明细/审计/幂等记录不重复；入库完成、库存状态、报损报溢三类已有出站生产者均证明同键重放仅产生一份业务副作用、outbox、对应 H2 审计和幂等记录 | `NEEDS_WORK` | 补档案补录、对账差异、发货确认、库存快照四类出站生产者，以及主备降级与业务回执重复的 L11 证据 |
| AC-8 投递保证 | `V1` | H8-WORKER | `scripts/h8_erp_interface_sync/test_h8_sync_worker.py` 的 failover/circuit 测试证明成功路径不双写且切换保留业务键 | `PASS` | - |
| AC-9 错误分类 | `V1` | H8-DOMAIN、H8-WORKER | Worker 已将网络/408/425/429/5xx 识别为可重试，其余错误直接死信；ASN、出库订单、商品主数据、销退申请和商品变更五类处理器均有 503/422 分类与凭据脱敏测试 | `PASS` | - |
| AC-10 货主仓隔离 | `V1` | H8-DOMAIN、H8-WORKER、H8-U2 | 当前货主 active 连接和仓库白名单参与唯一路由；`route-resolve` 已默认采用 `AuthContext.warehouse_scope` 并以 HTTP 403 拒绝显式越权仓库 | `NEEDS_WORK` | JWT 用户会话仍需从公共用户仓库授权模型解析多仓范围，并接入 Worker 调用身份 |
| AC-11 审计追踪 | `V1/V2` | H8-WORKER、H8-ASN-V2、H8-MSG-PG | ASN 成功路径已有 V2；Worker 在 schema/路由预检失败时调用同一 lifecycle API，真实 PostgreSQL 测试回读 `h8_exchange_receive` 与 `h8_exchange_final_failure` | `PASS` | - |
| AC-12 档案补录闭环 | `V1` | H8-DOMAIN、代码路径复审 | domain 禁止 H8 直接改 ASN；`sync_worker.py::handle_product_change` 尚无 M1/M-QL/M2 跨模块闭环证据 | `NEEDS_WORK` | 补商品变更事件、业务校验和当前 ASN 解锁的 L3/L4/L5 证据 |
| AC-13 契约与 S4 | `V1/V2/V4` | H8-DOMAIN、H8-WORKER、H8-ASN-V2 | 当前仅 ASN V2 与部分本地出站切片 | `NEEDS_WORK` | 每类消息补 L2/L3/L4/L11，并归档客户正式 ERP dev/staging 双向请求、回执、重试与审计关联 |

### 聚合验证

- V0：`python3 scripts/governance/check_quality_matrix.py --json`；`python3 scripts/governance/check_scope_gap_discovery.py --strict --module H8 --json`
- V1（H8-DOMAIN）：`cargo test --manifest-path backend/Cargo.toml -p wms-domain --lib h8_erp`
- V1（H8-WORKER）：在 `scripts/h8_erp_interface_sync` 运行 `python3 -m unittest test_h8_sync_worker test_exchange_lifecycle test_inbound_canonical -v`
- V1（H8-OUTBOUND-ROUTE）：在 `scripts/h8_erp_interface_sync` 运行 `python3 -m unittest test_outbound_routing -v`
- V1（H8-U2 出站路由）：`cargo test --manifest-path backend/Cargo.toml -p wms-api --lib h8_erp_connectors::tests::route_resolve_enforces_auth_context_warehouse_scope`
- V2（M2-PUTAWAY-PG）：`cargo test --manifest-path backend/Cargo.toml -p wms-api --test m2_smart_putaway_postgres smart_putaway_recommends_and_commits_owner_scoped_inventory_atomically -- --exact --test-threads=1`
- V2（M3-STATUS-PG）：`cargo test --manifest-path backend/Cargo.toml -p wms-api --test m3_ops_closeout_postgres status_change_enqueues_erp_outbox_and_process_succeeds -- --exact --test-threads=1`
- V2（H8-INBOUND-L11）：分别运行 `master_data_postgres h8_product_change_replays_without_duplicate_update_or_audit`、`m2_deferred_closeout_postgres_part2 h8_sales_return_create_replays_without_duplicate_order_or_audit`、`wave4_wave_planning_postgres h8_outbound_order_create_replays_without_duplicate_lines` 三条精确 PostgreSQL 测试；ASN 与商品创建复用各自现有 L11 回归
- V2（H8-OUTBOUND-L11）：运行 M2 上架、M3 库存状态、M-SA 报损和报溢四条精确 PostgreSQL 测试，断言同键重放不重复业务副作用、outbox、对应 H2 审计或幂等记录
- V2（H8-MSG-PG）：`cargo test --manifest-path backend/Cargo.toml -p wms-api --lib h8_erp_messages::`，覆盖真实 PostgreSQL 重放筛选和 Worker 即时接管
- V2（H8-ASN-V2）：按 `docs/runbooks/h8-erp-interface-table-sync.md` 的 ASN 入站闭环执行；证据为 `docs/retros/h8-asn-inbound-flow-evidence.json`
- V2（H8-ASN-REPLAY-V2）：同一 runbook 的 ASN 死信与人工重放切片；证据为 `docs/retros/h8-asn-manual-replay-evidence.json`
- V2（H8-LOCAL）：`just h8-local-integration`；证据为 `docs/retros/h8-local-integration-evidence.json`
- V3：不适用；US-H8-002 是后台交换能力，无独立用户页面，消息运维页属于 US-H8-003
- V4：未取得客户正式 ERP dev/staging 证据；本地容器、Mock 和 Docker MSSQL 不计 V4

### 验收结论

- 已证明：受控目录与纯 domain 规则；五类接口表 DTO 统一经过 canonical 转换后才进入业务 API handler，且五类业务 API 均有真实 PostgreSQL 同键重放证据；Worker 入站唯一路由、首次配置绑定、历史配置快照读取、重试主动复用原绑定、Docker MSSQL 人工重放桥的原键恢复、死信同步、即时接管与成功回执，绑定切换拒绝、契约版本拒绝、错误重试分类、凭据脱敏和未知储存条件写入前拒绝；`route-resolve` 对已注入单仓范围的默认绑定与越权拒绝；ASN 接口表入站 V2、幂等回放和 H2 生命周期审计；出站认领的连接货主/目录/仓库收窄、逐消息唯一路由，以及入库完成、库存状态、报损报溢三类现有生产者的仓库身份和 L11；本地出站主备与非双写切片。
- 未完成：真实 M-PM 规则查询与映射、JWT 用户多仓范围、档案补录/对账差异/发货确认/库存快照四类业务生产者与真实唯一路由证据，以及其余消息类型 L2–L4/L11、H8 成功消息业务资源引用和客户正式 ERP V4。
- 恢复条件：完成上述运行接线和逐消息证据后，再使用客户正式 ERP dev/staging 跑双向请求、回执、重试和审计关联；完成前保持 `deferred_stories`。

---

## 验收记录（US-H8-003 软件切片）

- 故事：`US-H8-003`
- 验收基线：2026-07-22 自研 ERP 与 Worker 运行治理扩展
- 验收日期：`2026-07-22`
- 质量矩阵状态：`deferred_stories`（S4 未齐）
- 证据层覆盖：软件切片 `V0/V1/V2/V3=PASS`；故事要求的 `V4=NEEDS_WORK`
- 整体结论：本轮运行治理软件切片已验证，故事整体 `NEEDS_WORK`

### US-H8-003 软件 AC 核对（不含 S4）

| AC | 结果 | 说明 |
|---|---|---|
| AC1 独立入口 | `PASS` | `h8-erp-messages` 菜单/页 |
| AC2 存储边界 | `PASS` | messages + attempts 表 |
| AC3 状态机 | `PASS` | domain + 测试 |
| AC4 并发认领 | `PASS` | claim/lease API + 测试 |
| AC5 失败重试记录 | `PASS` | claim/dead/replay 及真实 Worker 的普通成功/失败均追加 attempts；L2 失败按 1/2/4/8/16 秒基线与稳定 ±20% 抖动落 `next_retry_at`；档案补录 L3 由数据库强制 5 次/5 分钟/24 小时，隔离 PostgreSQL 已验证立即重取拒绝、第 5 次与到期 dead；隔离 Docker MSSQL 已验证 L2 到期门禁和人工重放绕过 |
| AC6 死信条件 | `PASS` | should_enter_dead + mark_dead + h8_message_dead H2 审计 |
| AC7 人工重放 | `PASS` | API + E2E + 不换业务 id |
| AC8 查询详情 | `PASS` | QueryPanel 的连接、仓库、通道、外部业务标识、幂等键、关联标识和时间范围均进入服务端精确过滤；真实 PostgreSQL 与关闭 dev-mock 的 Playwright 复用确定性业务键 |
| AC9 监控指标与 Worker 健康 | `PASS` | 货主固定在上下文；连接/通道/消息类型计数使用日统计快照，P95 由数据库计算过滤范围最近 10k 次完成尝试，不再拉回全部尝试；同页展示实例、版本、方向、认领数、创建时间、心跳和派生健康状态 |
| AC10 分区与保留 | `PASS` | 消息 `created_at` 与尝试 `started_at` 均为声明式 RANGE 月分区；当前月/下月由迁移和每小时维护任务补齐，全局登记表保持跨月 ID/幂等唯一，真实 PostgreSQL 已验证分区路由、裁剪、append-only 和清理回退 |
| AC11 权限审计 | `PASS` | read/write 权限、跨货主拒绝及 detail/replay/archive/purge/dead H2 审计已验证；只读 JWT 用户按公共授权表获得多仓列表/统计裁剪，显式跨仓和详情越权返回 403；API Key 单仓身份同时约束消息操作与生命周期写入 |
| AC12 查询裁剪 | `PARTIAL` | 默认 7 天、货主/仓库/时间索引、每页 1–200 条、`created_at + id` 稳定游标、日统计快照和最近 10k 完成尝试 P95 已由真实 PostgreSQL 验证；正式基线已定为单货主单月 10M 消息 + 10M 完成尝试，列表 P95 ≤ 500ms、统计 P95 ≤ 1s，尚缺生产等价 dev/staging 的原始压测证据 |
| AC13 页面证据 | `PASS` | dev-mock Playwright 8 条覆盖 UI 异常分支、“加载更多”及下一页失败中文提示；关闭 dev-mock 的真实 PostgreSQL Playwright 2 条覆盖稳定分页、高级筛选、详情、重放与重复拒绝、只读、跨货主拒绝、Worker、暂停恢复、保留策略和授权解密；13 张截图完成视觉复核 |
| AC14 S4 | `NEEDS_WORK` | 客户正式 ERP |
| AC15 暂停与恢复认领 | `PASS` | 独立持久控制、到期恢复、权限/H2 审计、Worker 认领前门禁和页内操作均有自动化证据 |
| AC16 完整报文短期保留 | `PASS` | 默认摘要；按连接启用 1–30 天、pgcrypto 密文、Key Version 历史密钥解析、授权解密/no-store/H2 脱敏审计、审计失败门禁、每小时到期清密文及页面证据齐全 |
| UI-SEMANTICS | `PASS` | 受控单选、中文状态/类型/通道/尝试结果和中文日期展示；真实 E2E 断言全部更多查询值进入 API 且只返回精确命中行 |

验证命令：

| ID | 命令 |
|---|---|
| H8-MSG-U1 | `cargo test --manifest-path backend/Cargo.toml -p wms-domain --lib h8_erp_message` |
| H8-MSG-U2 | `cargo test --manifest-path backend/Cargo.toml -p wms-domain --lib h8_erp_exchange` |
| H8-MSG-U3 | `cargo test --manifest-path backend/Cargo.toml -p wms-api --lib h8_erp_messages` |
| H8-U1 | `cargo test --manifest-path backend/Cargo.toml -p wms-domain --lib h8_erp` |
| H8-U2 | `cargo test --manifest-path backend/Cargo.toml -p wms-api --lib h8_erp_connectors` |
| H8-WORKER | `python3 -m unittest test_exchange_lifecycle test_h8_sync_worker test_inbound_canonical -v`（在 `scripts/h8_erp_interface_sync`） |
| H8-CANONICAL | `python3 -m unittest test_inbound_canonical -v`（在 `scripts/h8_erp_interface_sync`） |
| H8-INBOUND-L11 | 三条精确 PostgreSQL 测试：`master_data_postgres h8_product_change_replays_without_duplicate_update_or_audit`、`m2_deferred_closeout_postgres_part2 h8_sales_return_create_replays_without_duplicate_order_or_audit`、`wave4_wave_planning_postgres h8_outbound_order_create_replays_without_duplicate_lines` |
| H8-OUTBOUND-L11 | M2 上架、M3 库存状态、M-SA 报损与报溢四条精确 PostgreSQL 测试 |
| H8-MSG-PG | `cargo test --manifest-path backend/Cargo.toml -p wms-api --lib h8_erp_messages::pg_repository_tests` |
| H8-WEB-TYPE | `pnpm --dir apps/web-admin exec tsc --noEmit` |
| H8-MSG-E2E | `pnpm --dir prototypes exec playwright test --config=playwright-web-admin-h8-messages-config.ts` |
| H8-MSG-REAL-E2E | `DATABASE_URL=<隔离测试库> pnpm --dir prototypes exec playwright test --config=playwright-web-admin-h8-real-config.ts --grep 'H8 ERP 消息真实链路'` |
| H8-T1 | `just gov-t1` |
