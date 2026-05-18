# wms 项目代码书写规范

> 本文档是 wms 项目所有代码（Rust / TypeScript / SQL / 脚本）的书写规范唯一真相源。
> AI 编码助手与人类开发者同样遵守。治理体系见 `docs/governance.md`。

---

## 项目概述

- **项目**：医药冷链 GSP 合规仓储管理系统（wms）
- **后端**：Rust（Axum + SQLx + PostgreSQL + Tokio + tracing）
- **Web 前端**：Vite + React + TypeScript + shadcn/ui + Zustand + TanStack Query + React Router
- **PDA 端**：React Native + TypeScript
- **跨端契约**：utoipa → OpenAPI → openapi-typescript
- **包管理**：Cargo workspace（后端）+ pnpm workspace（前端）
- **开发模式**：TDD（outside-in 双层循环），详见 `docs/adr/0006-tdd-and-test-layers.md`
- **治理**：5 类 + 4 Tier + Baseline + diff 触发，详见 `docs/governance.md`

---

## 零、文件命名速查表

> 所有文件命名规则的唯一汇总。各章节的详细说明引用本表。
> 检查脚本：`scripts/governance/check_file_naming.py`（T1）

### 代码文件

| 类型 | 命名规则 | 示例 | 反例 |
|------|---------|------|------|
| Rust 源文件 | snake_case.rs | `receipt_order.rs` | `ReceiptOrder.rs` |
| Rust crate 目录 | kebab-case | `wms-domain/` | `wms_domain/` |
| Rust mod 目录 | snake_case | `cold_chain/` | `cold-chain/` |
| TS 组件文件 | PascalCase.tsx | `ReceiptForm.tsx` | `receipt-form.tsx` |
| TS 非组件文件 | kebab-case.ts | `use-stock-query.ts` | `useStockQuery.ts` |
| TS 目录 | kebab-case | `features/inbound/` | `features/Inbound/` |
| SQL 迁移 | `NNNN_<description>.sql` | `0001_create_products.sql` | `create_products.sql` |
| SQL 回滚 | `NNNN_<description>.down.sql` | `0001_create_products.down.sql` | — |
| 治理脚本 | snake_case.py（公共库前缀 `_`） | `check_doc_links.py`, `_baseline.py` | `checkDocLinks.py` |
| 测试文件（Rust） | 放在 `tests/` 目录，snake_case | `test_receive_expired.rs` | — |
| 测试文件（TS） | `*.spec.ts` / `*.spec.tsx` | `receipt-form.spec.tsx` | `receipt-form.test.tsx` |

### 文档文件

| 类型 | 命名规则 | 示例 | 反例 |
|------|---------|------|------|
| ADR | `NNNN-<slug>.md`（slug kebab-case） | `0001-tech-stack.md` | `0001_tech_stack.md` |
| 领域文档 | `<context>.md`（context = 代码目录名，kebab-case） | `docs/domain/master-data.md` | `docs/domain/MasterData.md` |
| 合规文档 | `gsp-<topic>.md` | `docs/compliance/gsp-audit-trail.md` | `docs/compliance/审计.md` |
| Retro | `wave-<N>-retro.md` | `docs/retros/wave-0-retro.md` | `docs/retros/retro1.md` |
| 固定名文档 | 不可改名 | `governance.md`, `architecture-dependencies.md`, `coding-standards.md` | — |
| 根目录文档 | 大写 | `README.md`, `ROADMAP.md`, `TODO.md`, `CHANGELOG.md`, `AGENTS.md` | `readme.md` |

### 配置文件

| 类型 | 命名规则 |
|------|---------|
| justfile | `justfile`（无后缀） |
| lefthook | `lefthook.yml` |
| editorconfig | `.editorconfig` |
| gitignore | `.gitignore` |
| gitattributes | `.gitattributes` |
| gate-rules | `governance/gate-rules.toml` |
| baseline | `governance/baselines/<check_name>.json` |
| 环境变量 | `.env.example`（入库）/ `.env`（不入库） |

---

## 一、Rust 代码规范

### 1.1 命名

| 类型 | 风格 | 示例 |
|------|------|------|
| Struct / Enum / Trait | PascalCase | `ReceiptOrder`, `InventoryError`, `StockRepository` |
| 函数 / 方法 | snake_case | `receive_goods`, `calculate_fifo` |
| 模块 | snake_case | `inbound`, `cold_chain` |
| 常量 | SCREAMING_SNAKE | `MAX_BATCH_SIZE`, `DEFAULT_TEMPERATURE_THRESHOLD` |
| 类型参数 | 单大写字母或短 PascalCase | `T`, `Repo` |
| Crate 名 | kebab-case（Cargo.toml）/ snake_case（代码内） | `wms-domain` / `wms_domain` |
| 数据库表 | snake_case 复数 | `receipt_orders`, `inventory_items` |
| 数据库列 | snake_case | `created_at`, `supplier_id` |
| 迁移文件 | `NNNN_<description>.sql` | `0001_create_products.sql` |

**领域命名规则**：
- 实体用业务术语，不用技术术语（`ReceiptOrder` 不是 `InboundRecord`）
- 值对象用描述性名称（`BatchNumber`, `ExpiryDate`, `Temperature`）
- 仓储 trait 用 `<Entity>Repository`（`ProductRepository`）
- 应用服务用 `<Context>Service`（`InboundService`）
- 命令/查询用 `<Action><Entity>Command` / `<Entity>Query`（`ReceiveGoodsCommand`, `StockQuery`）

**状态转换方法命名约定**（v3.1，借鉴 Odoo state button，参见 [ADR-0008](adr/0008-borrow-from-odoo.md) §9）：

实体的状态转换方法必须使用统一前缀，便于阅读、审计扫描、治理脚本识别：

| 前缀 | 含义 | 示例 |
|----|----|----|
| `confirm_` | 草稿 → 已确认（首次激活） | `inbound.confirm()` |
| `validate_` | 已确认 → 已完成（最终签字） | `inbound.validate()` |
| `done_` | 推进到完成态（业务流终态） | `picking.done()` |
| `cancel_` | 任意态 → 已取消 | `order.cancel()` |
| `assign_` | 资源分配/预占 | `order.assign_stock()` |
| `revert_` | 撤回到上一状态 | `inbound.revert()` |

**非状态转换方法**用其他前缀区分：
- `compute_` = 派生字段/聚合计算
- `query_` / `search_` = 只读查询
- `import_` / `export_` = 数据交换
- `find_` / `get_` = 数据获取

**约定收益**：
- 治理脚本可扫描状态转换方法名，自动校验所有此类方法都接入 audit_trail
- 阅读代码时立即识别"业务计算"vs"状态变化"
- 前端按钮事件名与后端方法对齐（`button onClick={() => confirmInbound(id)}`）

### 1.1.1 Wizard / 临时模型规范

> 借鉴 Odoo TransientModel 模式，参见 [ADR-0008](adr/0008-borrow-from-odoo.md) §8

涉及多步骤交互的业务（盘点 / 双人验收 / 销毁审批 / 批量上架），不直接污染主业务表，使用 **wizard 模式**：

```rust
pub trait Wizard: Serialize + Deserialize {
    type Result;
    fn name() -> &'static str;
    fn ttl_minutes() -> u32 { 60 }   // 默认 1 小时
    async fn execute(&self, ctx: &Context) -> Result<Self::Result>;
}
```

**wizard 数据存于** `wizard_session` 表（不进入主业务表）；TTL 到期由 `pg_cron` 自动清理。

**何时用 wizard**：
- 需要多步收集数据（向导式 UI）
- 中间态不应影响主业务（如双人验收第一签字后等第二签字，未完成不应被其他流程消费）
- 可能被中断后恢复（用户登出后回来继续）

**何时不用 wizard**：
- 单步操作（直接命令即可）
- 数据需要参与业务查询（应该进主表）

### 1.1.2 聚合根识别原则（v3.1，DDD 战术）

> 关联 [ADR-0012](adr/0012-bounded-contexts.md) 限界上下文。

**聚合根（Aggregate Root）**是 BC 内的"事务一致性边界"。识别聚合根的 4 个原则：

1. **真不变量（True Invariants）**：聚合内必须事务性维护的不变量
   - 例：`InboundOrder` 内"实到 + 缺货 + 拒收 = 预报数量"
   - 例：`InventoryBatch` 内"批次状态变更必须有 approval_source"
2. **生命周期一致**：聚合内对象同生同死
   - `InboundOrderItem` 离开 `InboundOrder` 无意义 → 是聚合内子实体
3. **小聚合优先**：宁可 5 个小聚合，不要 1 个大聚合
   - `Inventory` 不该聚合 `OutboundOrder`（跨模块）
4. **跨聚合用 ID 引用**：不直接持有对象引用
   - ✅ `OutboundOrder { product_id: ProductId }`
   - ❌ `OutboundOrder { product: Product }`

**wms 主要聚合根**（v3.1，待 Wave 2-3 实施时复盘）：

| 聚合根 | BC | 子实体 | 不变量 |
|---|---|---|---|
| `InboundOrder` | M2 | `InboundOrderItem` | 数量闭合 + 时间窗 + 双人验收 |
| `OutboundOrder` | M4 | `OutboundOrderItem`, `PickTask` | 批号 ERP 指定 + 拣选≠复核 |
| `InventoryBatch` | M3 | `InventoryTransaction` | 状态变更必有 approval_source |
| `WarehouseAppointment` | H-DOCK | `AppointmentObject` | 时间窗不冲突 + 车辆温区匹配 |
| `QualityLiaison` | M-QL | `LiaisonAttachment`, `LiaisonApproval` | 审批闭环 |
| `TraceCode` | M-TC | `TraceCodeBinding` | GS1 解析 + 状态机 |
| `User` | H1 | `UserRole`, `Permission` | 权限不可越权 |

**禁止**：业务字段直接放 BC 之间（如 OutboundOrder 不该有 supplier_id，应该通过 InventoryBatch 间接引用）。

### 1.1.3 领域事件命名约定（v3.1，DDD 战术）

> 关联 [ADR-0011](adr/0011-observability.md) §事件分类与命名 + [ADR-0012](adr/0012-bounded-contexts.md)。

**领域事件名 = `<聚合根>.<动作过去式>`**：

| ✅ 好例子 | ❌ 反例 | 理由 |
|---|---|---|
| `InboundOrder.confirmed` | `confirm_inbound_order` | 名词聚合根 + 过去时（事件已发生）|
| `InventoryBatch.recalled` | `recall_batch` | — |
| `Appointment.arrived` | `dock_appointment_arrival_received` | 简洁清晰 |
| `OutboundOrder.shipped` | `ship_order` | — |

**事件类型分类**（参 ADR-0011 H2-005 事件总线）：

```
audit.<resource>.<action>          # 审计事件（H2 自动发布）
business.<module>.<event>          # 业务事件（业务模块发布）
system.<source>.<event>            # 系统事件（基础设施发布）
```

**事件载荷标准字段**：

```rust
struct DomainEvent {
    event_type: String,              // 如 "business.inbound.confirmed"
    aggregate_id: AggregateId,       // 聚合根 ID
    aggregate_type: String,          // 聚合根类型名
    tenant_id: TenantId,             // 多货主隔离
    occurred_at: DateTime<Utc>,
    actor: ActorRef,                 // 触发者（user / system / external）
    payload: serde_json::Value,      // 业务载荷（按事件类型不同）
    trace_id: TraceId,               // 跨模块追踪
}
```

**禁止**：
- 在事件名里加"成功/失败"（`order.shipped_success` ❌）→ 应该用单独的 failed 事件
- 事件载荷含敏感字段（密码 / 身份证完整号）→ 必须脱敏

### 1.2 模块组织

```
backend/crates/<crate>/src/
├── lib.rs              # crate 入口，仅 pub mod 声明 + 顶层 doc
├── <context>/          # 按限界上下文分目录
│   ├── mod.rs          # 子模块声明
│   ├── entities.rs     # 实体（聚合根 + 子实体）
│   ├── value_objects.rs # 值对象
│   ├── events.rs       # 领域事件
│   ├── errors.rs       # 该上下文的错误类型
│   ├── repository.rs   # 仓储 trait（不含实现）
│   ├── service.rs      # 领域服务（纯业务逻辑，无 IO）
│   └── tests/          # 该上下文的测试
└── error.rs            # crate 级公共错误（如有）
```

**拆文件时机**：
- 单文件 > 300 行 → 考虑拆
- 单文件 > 500 行 → 必须拆
- 一个 struct 的 impl 块 > 100 行 → 拆到独立文件

**禁止**：
- `mod.rs` 里写业务逻辑（只做 pub mod / pub use 声明）
- 跨上下文直接 `use`（必须通过 app 层编排或 domain event）

### 1.3 错误处理

```rust
// ✅ 领域层：用 thiserror 定义具体错误
#[derive(Debug, thiserror::Error)]
pub enum InboundError {
    #[error("supplier GSP certificate expired: {supplier_id}")]
    SupplierGspExpired { supplier_id: SupplierId },

    #[error("product not found: {0}")]
    ProductNotFound(ProductId),

    #[error("dual acceptance not met: need {required}, got {actual}")]
    DualAcceptanceNotMet { required: u8, actual: u8 },
}

// ✅ 应用层：用 anyhow 包装基础设施错误
pub async fn receive_goods(cmd: ReceiveCommand) -> anyhow::Result<ReceiptId> {
    // ...
}

// ✅ API 层：转换为 HTTP 响应
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // 按错误类型映射 HTTP 状态码
    }
}
```

**规则**：
- domain crate：只用 `thiserror`，禁止 `anyhow`
- app crate：可用 `anyhow`（编排多个可能失败的操作）
- api crate：负责把 domain error / anyhow error 转为 HTTP 响应
- **禁止**：生产路径使用 `.unwrap()` / `.expect()` / `panic!()`
- **禁止**：吞掉错误（`let _ = fallible_fn();`）
- **允许**：测试代码中使用 `.unwrap()`

### 1.4 API Handler 模式

```rust
/// POST /api/inbound/receive
///
/// 收货：PDA 扫码后创建收货单
#[utoipa::path(
    post,
    path = "/api/inbound/receive",
    request_body = ReceiveRequest,
    responses(
        (status = 201, body = ReceiveResponse),
        (status = 400, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
    ),
    tag = "inbound"
)]
pub async fn receive_handler(
    State(ctx): State<AppContext>,
    AuthUser(user): AuthUser,          // 鉴权提取器
    Json(req): Json<ReceiveRequest>,   // 请求体
) -> Result<(StatusCode, Json<ReceiveResponse>), AppError> {
    // 1. 输入校验（在类型层已完成大部分）
    // 2. 调用应用服务
    let id = ctx.inbound_service.receive(req.into_command(user.id)).await?;
    // 3. 返回响应
    Ok((StatusCode::CREATED, Json(ReceiveResponse { id })))
}
```

**规则**：
- 每个 handler 必须有 `#[utoipa::path]` 注解（OpenAPI 契约）
- handler 只做：提取参数 → 调用 app service → 转换响应
- **禁止**在 handler 里写业务逻辑
- 错误统一通过 `AppError` 转换（不在 handler 里 match 错误码）
- 响应格式统一：成功 `{ data: T }` / 错误 `{ error: { code, message, details } }`

### 1.5 SQL / 迁移规范

```sql
-- migrations/0001_create_products.sql

-- 商品表
CREATE TABLE products (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code        VARCHAR(50) NOT NULL UNIQUE,
    name        VARCHAR(200) NOT NULL,
    -- 储存条件：cold_storage / frozen / cool / normal
    storage_condition VARCHAR(20) NOT NULL DEFAULT 'normal',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 索引命名：idx_<table>_<columns>
CREATE INDEX idx_products_code ON products(code);

COMMENT ON TABLE products IS '商品主数据';
COMMENT ON COLUMN products.storage_condition IS '储存条件：cold_storage/frozen/cool/normal';
```

**规则**：
- 表名 snake_case 复数（`products`, `receipt_orders`）
- 列名 snake_case（`created_at`, `supplier_id`）
- 主键统一 `id UUID`（除非有业务编号需求）
- 必须有 `created_at` / `updated_at`
- 索引命名 `idx_<table>_<columns>`
- 外键命名 `fk_<table>_<ref_table>`
- 约束命名 `chk_<table>_<rule>`
- 每个迁移必须有对应的 down 脚本（`0001_create_products.down.sql`）
- 迁移文件头部注释说明目的
- **禁止**：在迁移中写 DML（INSERT/UPDATE/DELETE）——种子数据用单独脚本
- **审计表**：只能 INSERT，禁止 UPDATE/DELETE（GSP 红线）

### 1.6 日志 / Tracing 规范

```rust
use tracing::{info, warn, error, instrument};

#[instrument(
    skip(repo),                    // 不打印大对象
    fields(supplier_id = %cmd.supplier_id)  // 结构化字段
)]
pub async fn receive_goods(cmd: ReceiveCommand, repo: &dyn StockRepo) -> Result<ReceiptId> {
    info!("starting goods receipt");

    if supplier.gsp_expired() {
        warn!(supplier_id = %cmd.supplier_id, "supplier GSP expired, rejecting");
        return Err(InboundError::SupplierGspExpired { .. });
    }

    // ...
    info!(receipt_id = %id, "goods receipt completed");
    Ok(id)
}
```

**日志级别定义**：

| 级别 | 用在什么场景 | 示例 |
|------|------------|------|
| `error` | 系统无法自动恢复的故障 | 数据库连接断开、审计写入失败 |
| `warn` | 业务异常但系统可继续 | 供应商资质过期被拒、温度超标 |
| `info` | 关键业务事件（审计可用） | 收货完成、出库完成、盘点开始 |
| `debug` | 开发调试信息 | SQL 查询参数、缓存命中/未命中 |
| `trace` | 极细粒度（通常关闭） | 每次循环迭代、序列化细节 |

**规则**：
- 所有 pub 函数用 `#[instrument]`（除非是纯计算无副作用的小函数）
- 结构化字段优先于字符串拼接（`info!(order_id = %id, "created")` 不是 `info!("created order {id}")`）
- **禁止**打印敏感信息（密码、token、身份证号）
- error 级别必须附带足够上下文（谁、什么操作、什么错误）
- GSP 关键事件必须 info 级别（审计追踪依赖）

### 1.7 注释风格

```rust
/// 收货单聚合根
///
/// 生命周期：草稿 → 已收货 → 验收中 → 合格/不合格 → 已上架
/// 不变量：
/// - 供应商 GSP 证必须在有效期内
/// - 双人验收签字后状态才能流转
pub struct ReceiptOrder { /* ... */ }

// 实现细节注释用 // （不出现在 cargo doc）
// 这里用 BTreeMap 而不是 HashMap 是因为需要按批号排序输出
let batches: BTreeMap<BatchNumber, Quantity> = /* ... */;
```

**规则**：
- 所有 pub 项必须有 `///` doc comment
- doc comment 写"是什么 + 为什么"，不写"怎么做"（代码本身说明怎么做）
- 领域实体的 doc comment 必须包含：生命周期/状态机 + 不变量
- 内部实现注释用 `//`，只在"为什么这样做不明显"时写
- **禁止**注释掉的代码（直接删，git 有历史）
- **禁止**无意义注释（`// 获取用户` 在 `get_user()` 上面）

### 1.8 可见性

- 默认 `pub(crate)`（crate 内可见，外部不可见）
- 只有跨 crate 需要的才用 `pub`
- 模块内部辅助函数不加 pub
- **禁止** `pub` 滥用（"先全 pub 后面再收"是技术债起点）

---

## 二、TypeScript 代码规范

### 2.1 命名

| 类型 | 风格 | 示例 |
|------|------|------|
| 文件（组件） | PascalCase.tsx | `ReceiptForm.tsx`, `StockTable.tsx` |
| 文件（非组件） | kebab-case.ts | `use-stock-query.ts`, `format-date.ts` |
| 目录 | kebab-case | `features/inbound/`, `components/ui/` |
| 组件 | PascalCase | `ReceiptForm`, `StockTable` |
| Hook | camelCase + use 前缀 | `useStockQuery`, `useAuthStore` |
| 函数 | camelCase | `formatBatchNumber`, `calculateFifo` |
| 常量 | SCREAMING_SNAKE | `API_BASE_URL`, `MAX_PAGE_SIZE` |
| 类型/接口 | PascalCase | `Product`, `ReceiptOrder`, `StockQuery` |
| Enum | PascalCase（成员也是） | `StorageCondition.ColdStorage` |
| CSS 类（Tailwind） | 不自定义类名，用 Tailwind utility | — |

### 2.2 组件规范

```tsx
// ✅ 函数组件 + 命名导出
export function ReceiptForm({ supplierId, onSubmit }: ReceiptFormProps) {
  // hooks 在顶部
  const { data, isLoading } = useReceiptQuery(supplierId)
  const [draft, setDraft] = useState<ReceiptDraft | null>(null)

  // 早返回处理 loading / error
  if (isLoading) return <Skeleton />
  if (!data) return null

  // 渲染
  return (
    <form onSubmit={handleSubmit}>
      {/* ... */}
    </form>
  )
}

// Props 类型紧跟组件定义之前
interface ReceiptFormProps {
  supplierId: string
  onSubmit: (draft: ReceiptDraft) => void
}
```

**规则**：
- 只用函数组件（禁止 class 组件）
- 命名导出（`export function X`），禁止默认导出（`export default`）
- Props 用 `interface`（不用 `type`），命名 `<Component>Props`
- 一个文件一个组件（小的辅助组件可以同文件，但不导出）
- 组件文件 > 200 行 → 拆分
- **禁止** `any`；必要时用 `unknown` + 类型守卫
- **禁止** `// @ts-ignore` / `// @ts-expect-error`（除非附 issue 链接）

### 2.3 Hook 规范

```tsx
// ✅ 自定义 hook：封装 TanStack Query
export function useStockQuery(productId: string) {
  return useQuery({
    queryKey: ['stock', productId],
    queryFn: () => api.getStock(productId),
    staleTime: 30_000,
  })
}

// ✅ 自定义 hook：封装 mutation + 乐观更新
export function useAdjustStock() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: api.adjustStock,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['stock'] })
    },
  })
}
```

**规则**：
- 所有 API 调用通过 TanStack Query hook 封装（禁止组件内裸 fetch）
- hook 文件放在 `features/<context>/api.ts` 或 `hooks/use-<name>.ts`
- queryKey 必须结构化（`['entity', id, filters]`），禁止字符串拼接
- mutation 成功后必须 invalidate 相关 query

### 2.4 状态管理规范

| 数据类型 | 放哪里 | 示例 |
|---------|--------|------|
| 服务端数据（列表、详情） | TanStack Query | 商品列表、库存、订单 |
| 客户端全局状态 | Zustand | 当前用户、UI 主题、侧边栏 |
| 表单临时状态 | React useState / useForm | 新建商品表单 |
| URL 状态（分页、筛选） | React Router searchParams | `?page=2&status=active` |

**禁止**：
- 把服务端数据存到 Zustand（TanStack Query 已管理）
- 在 Zustand store 里写异步请求（交给 TanStack Query）
- 用 Context 做全局状态（性能差，用 Zustand）

### 2.5 API 调用规范

```tsx
// ✅ 统一走 @wms/api-client（从 OpenAPI 自动生成）
import { api } from '@wms/api-client'

// 禁止：
// fetch('/api/products')           ← 裸 fetch
// axios.get('/api/products')       ← 引入额外 HTTP 库
// 手写类型 interface Product {}    ← 类型必须从 OpenAPI 生成
```

**规则**：
- 所有 API 调用通过 `@wms/api-client` 包（从 OpenAPI 自动生成）
- **禁止**裸 fetch / axios / 手写 API 类型
- 如果 OpenAPI 还没生成某个接口 → 先更新后端 utoipa 注解 → 跑 gen-api → 再写前端
- 错误处理统一在 TanStack Query 的 `onError` / 全局 error boundary

### 2.6 样式规范（Tailwind + shadcn/ui）

```tsx
// ✅ Tailwind utility 直接写
<div className="flex items-center gap-2 p-4 rounded-lg border">

// ✅ 条件样式用 cn()（shadcn/ui 提供）
<div className={cn(
  "px-4 py-2 rounded",
  isActive && "bg-primary text-primary-foreground",
  isDisabled && "opacity-50 cursor-not-allowed"
)}>

// ❌ 禁止：
// <div style={{ display: 'flex' }}>     ← inline style
// <div className="my-custom-class">     ← 自定义 CSS 类
// import styles from './X.module.css'   ← CSS Modules
```

**规则**：
- 只用 Tailwind utility class
- 条件样式用 `cn()` 工具函数
- 复用样式 → 抽组件，不抽 CSS 类
- shadcn/ui 组件优先；不够再用 Radix 原语；最后才自己写
- **禁止**：inline style / CSS Modules / styled-components / 自定义 CSS 类

### 2.7 Import 顺序

```tsx
// 1. React / 框架
import { useState } from 'react'
import { useNavigate } from 'react-router-dom'

// 2. 第三方库
import { useQuery } from '@tanstack/react-query'

// 3. 内部包（@wms/*）
import { api } from '@wms/api-client'
import type { Product } from '@wms/domain-types'

// 4. 当前 feature 内
import { useStockQuery } from '../api'
import { StockTable } from './StockTable'

// 5. 类型（type-only import 放最后）
import type { StockTableProps } from './types'
```

由 `eslint-plugin-import` 自动排序，不需要手动维护。

---

## 三、跨端规范

### 3.1 API 设计规范

**URL 命名**：
```
GET    /api/<context>/<resource>          # 列表
GET    /api/<context>/<resource>/:id      # 详情
POST   /api/<context>/<resource>          # 创建
PUT    /api/<context>/<resource>/:id      # 全量更新
PATCH  /api/<context>/<resource>/:id      # 部分更新
DELETE /api/<context>/<resource>/:id      # 删除（软删除）

# 非 CRUD 操作用动词
POST   /api/inbound/receipts/:id/accept  # 验收
POST   /api/outbound/orders/:id/pick     # 拣选
```

**规则**：
- URL 全小写 kebab-case（`/api/cold-chain/temperature-logs`）
- 资源名用复数（`/products` 不是 `/product`）
- 嵌套不超过 2 层（`/api/inbound/receipts/:id/items` 可以；再深就拍平）
- API 版本放 URL 路径（`/api/v1/...`）；breaking change 走新主版本 `/api/v2/`，老版本保留 ≥ 6 个月（与 H3-001 §4 一致；详见 [reviews/software-design-audit-2026-05-18.md](reviews/software-design-audit-2026-05-18.md) §3.1）

**分页**：
```json
GET /api/products?page=1&page_size=20&sort=created_at:desc

{
  "data": [...],
  "pagination": {
    "page": 1,
    "page_size": 20,
    "total": 156,
    "total_pages": 8
  }
}
```

**错误响应**：
```json
{
  "error": {
    "code": "SUPPLIER_GSP_EXPIRED",
    "message": "供应商 GSP 证书已过期",
    "details": {
      "supplier_id": "xxx",
      "expired_at": "2026-01-01"
    }
  }
}
```

**规则**：
- 错误码 SCREAMING_SNAKE（`SUPPLIER_GSP_EXPIRED`）
- message 面向用户（中文）
- details 面向开发者（结构化数据）
- HTTP 状态码语义正确（400 客户端错 / 401 未认证 / 403 无权限 / 404 不存在 / 409 冲突 / 422 校验失败 / 500 服务端错）

### 3.2 日志级别定义（跨端统一）

| 级别 | 场景 | 前端是否上报 |
|------|------|-------------|
| error | 系统故障、不可恢复 | ✅ 必须上报 |
| warn | 业务异常、可恢复 | ✅ 上报 |
| info | 关键业务事件 | 可选（GSP 相关必须） |
| debug | 开发调试 | ❌ 不上报 |

### 3.3 幂等性规范

所有写操作必须幂等（GSP 红线 — 重放不能产生重复数据）：

```
POST /api/inbound/receipts
Header: Idempotency-Key: <client-generated-uuid>
```

**规则**：
- 客户端生成 `Idempotency-Key`（UUID v4）
- 后端用 Redis / PG 存 key → response 映射，TTL 24h
- 相同 key 重复请求 → 返回首次响应，不重复执行
- 前端 TanStack Query mutation 自动附加 key

### 3.4 时间与时区

- 后端存储 / 传输统一 **UTC**（`TIMESTAMPTZ`）
- 前端展示时转为用户本地时区
- API 响应中时间格式：ISO 8601（`2026-05-15T12:00:00Z`）
- **禁止**：存储本地时间、用 `TIMESTAMP WITHOUT TIME ZONE`

### 3.5 ID 规范

- 实体 ID 统一 UUID v7（时间有序，适合索引）
- 对外暴露 UUID 字符串（`"01234567-89ab-cdef-0123-456789abcdef"`）
- 内部可用 newtype 包装（`pub struct ProductId(Uuid);`）
- **禁止**：自增 int 作为对外 ID（可被枚举）
- **禁止**：在 URL 中暴露内部序号

---

## 四、禁止清单（红线）

无论任何理由，以下行为**绝对禁止**：

### Rust
- 生产路径 `.unwrap()` / `.expect()` / `panic!()`
- `#[allow(unsafe_code)]`（除非有独立 ADR 批准）
- domain crate 依赖 infra / api crate
- 直接 SQL 修改库存数据（必须经领域服务）
- 审计表 UPDATE / DELETE

### TypeScript
- `any` 类型
- `// @ts-ignore`（无 issue 链接）
- 裸 fetch / axios（必须走 @wms/api-client）
- `export default`
- inline style / CSS Modules / 自定义 CSS 类

### 跨端
- 硬编码密钥 / token / 连接字符串
- 注释掉的代码（直接删）
- `if env == "prod"` 分支逻辑
- 跳过审计层的库存变更
- 不带 Idempotency-Key 的写操作

---

## 五、TDD 节奏提醒

每次写业务代码前，必须遵循 outside-in 双层 TDD：

1. **外层红**：写一个 L3 业务流程测试 → 必须失败
2. **内层循环**：L1 红 → 绿 → 重构（多次）
3. **外层验证**：L3 重新跑 → 应通过
4. **维度补充**：按 ADR-0006 §2.3 补 L2/L4/L5/L8/L11

测试命名：`test_<被测>_<条件>_<期望>`

```rust
#[test]
fn test_receive_with_expired_supplier_should_reject() { /* ... */ }
```

```tsx
it('should reject receipt when supplier GSP is expired', () => { /* ... */ })
```

---

## 六、提交前自查清单

每次提交代码前确认：

- [ ] 走了 outside-in TDD（外层 L3 + 内层 L1）
- [ ] 写操作含 L11 幂等性测试
- [ ] 涉及库存变更含 L5 数据一致性测试
- [ ] 涉权限 handler 含 L8 权限矩阵测试
- [ ] 无 `.unwrap()` / `any` / 裸 fetch
- [ ] 无注释掉的代码
- [ ] 新增 pub API 有 doc comment
- [ ] 新增 handler 有 utoipa 注解
- [ ] 迁移有 down 脚本
- [ ] 文档同步更新（改了行为必改文档）
