# ADR-0008：借鉴 Odoo 的 9 个设计

- 状态：Accepted
- 决策日期：2026-05-18
- 决策人：项目主人
- 相关：ADR-0001（技术栈）/ ADR-0007（v0.3 路线）

---

## 背景

2026-05-18 完成了 odoo（github.com/odoo/odoo master）的源码深入分析（`/tmp/odoo` 保留备查）。Odoo 是生产 20+ 年、全球部署的成熟 ERP/WMS 系统，Supply Chain/Inventory 14 个 addons + 核心 stock 模块 27 MB / 58K 行代码，含大量经过实战检验的设计模式。

本 ADR 把可借鉴的 9 个 Odoo 设计登记为 wms 的实施指引，避免重复造轮子，明确每项的实施 Wave 和工作量。

> **关键原则**：借鉴**架构思路**，不直接照搬代码（不同语言：Odoo Python ORM vs wms Rust + sqlx）。每个借鉴项都给出"在 wms 怎么做"的具体方案。

---

## 9 个借鉴清单

### 速查表

| # | Odoo 设计 | wms 落点 | 价值 | 风险 | 工作量 | 实施 Wave |
|---|---|---|---|---|---|---|
| 1 | mail.thread mixin | H2 `Auditable` Rust trait + macro | 🔴 极高 | 🟡 中 | 1.5 周 | **Wave 1**（H2 实施时）|
| 2 | stock.move 三段式 | M3 `inventory_move` 统一事件流 | 🔴 极高 | 🔴 高 | 1 个月 | Wave 2-3（M3 实施时）|
| 3 | ir.sequence 双实现 | M-CG `code_sequence` standard/no_gap | 🟡 高（合规）| 🟢 低 | 4 天 | Wave 1-2（M-CG 实施时）|
| 4 | __manifest__.py | wms `module-manifest.toml` 模块依赖图 | 🟢 中（治理）| 🟡 中 | 1-2 天 | **Wave 0**（治理增强）|
| 5 | GS1 nomenclature | M-TC `trace_code_nomenclature` + `trace_code_rule` | 🔴 极高 | 🟡 中 | 2 周 | Wave 4（M-TC 实施时）|
| 6 | ir.rule 行级访问控制 | H1 `record_rule` 行级权限引擎 | 🔴 极高（GSP）| 🟡 中 | 1.5 周 | **Wave 1**（H1 实施时）|
| 7 | ir.model.access.csv 声明式权限 | H1 `permission_matrix.csv` | 🟡 高 | 🟢 低 | 3 天 | **Wave 1**（H1 实施时）|
| 8 | TransientModel 向导 | wms `wizard` Rust 模式 + 自动清理 | 🟢 中 | 🟢 低 | 1 周 | Wave 2（基础 CRUD 时）|
| 9 | state button 命名约定 | 全局 `confirm_/validate_/done_/cancel_` 方法名 | 🟢 中 | 🟢 极低 | 半天 | **Wave 1** 立即采纳 |

---

## 详细方案

### 1. mail.thread mixin → `Auditable` Rust trait

**Odoo 原型**（`addons/mail/models/mail_thread.py` 5300 行 + mail_message 1505 行 + followers 531 行）：

```python
class StockPicking(models.Model):
    _name = 'stock.picking'
    _inherit = ['mail.thread', 'mail.activity.mixin']  # 一行声明
    state = fields.Selection([...], tracking=True)     # 字段级追踪
```

**wms 落地**：

```rust
// 1. 定义 trait
#[async_trait]
pub trait Auditable {
    async fn record_change(&self, ctx: &AuditContext, change: ChangeEvent) -> Result<()>;
    fn tracked_fields(&self) -> &'static [&'static str];
    fn entity_type() -> &'static str;
    fn entity_id(&self) -> i64;
}

// 2. derive macro 自动实现
#[derive(Auditable)]
#[audit(entity_type = "inbound_order")]
pub struct InboundOrder {
    pub id: i64,
    #[track] pub status: InboundStatus,
    #[track] pub batch_no: String,
}

// 3. AuditedRepo 包装 update/delete
let order = AuditedRepo::update(&pool, &ctx, order).await?;
// 自动：UPDATE + 写 audit_trail（旧值/新值）+ 同事务提交
```

**事务安全**：用 sqlx Transaction `ON COMMIT TRIGGER` 模式或 outbox pattern，模仿 odoo 的 `env.cr.precommit`。

**成本**：1.5 周（derive macro 1 周 + repo 集成半周）。
**实施 Wave**：Wave 1 W1.B 实施 H2 时同步落地。

---

### 2. stock.move 三段式 → M3 `inventory_move` 统一事件流

**Odoo 原型**（`addons/stock/models/stock_move.py` 2774 行）：

```python
location_id      = fields.Many2one('stock.location', ...)  # 源
location_dest_id = fields.Many2one('stock.location', ...)  # 目的
product_uom_qty  = fields.Float(...)
state            = fields.Selection([...])                 # draft→...→done
move_orig_ids    = fields.Many2many('stock.move')           # DAG 上游
move_dest_ids    = fields.Many2many('stock.move')           # DAG 下游
```

**wms 落地**：新增 `inventory_move` 表统一所有库存变动事件。

```sql
CREATE TABLE inventory_move (
    id              BIGSERIAL PRIMARY KEY,
    location_src    VARCHAR(64) NOT NULL,    -- 含虚拟库位 VIRTUAL/SUPPLIER 等
    location_dest   VARCHAR(64) NOT NULL,
    product_id      BIGINT NOT NULL,
    batch_no        VARCHAR(20) NOT NULL,
    qty             NUMERIC(15,3) NOT NULL,
    state           VARCHAR(20) NOT NULL,
    move_orig_id    BIGINT REFERENCES inventory_move(id),  -- DAG
    related_doc_type VARCHAR(20),  -- asn/outbound/adjustment 等
    related_doc_id   BIGINT,
    approval_source  VARCHAR(20) NOT NULL,
    operator_id      BIGINT NOT NULL,
    ts               TIMESTAMPTZ DEFAULT now()
);
```

**与现有 M3 表的关系**：
- 现有 `inventory`（按批次粒度的当前库存）= Odoo 的 `stock.quant`
- 新增 `inventory_move`（变动事件流）= Odoo 的 `stock.move`
- 现有 `inventory_transaction`（流水）= Odoo 的 `stock.move_line`

**虚拟库位**：`VIRTUAL/SUPPLIER` / `VIRTUAL/CUSTOMER` / `VIRTUAL/INVENTORY_LOSS` / `VIRTUAL/PRODUCTION` 让入库/出库/调整/报损都是同一种 inventory_move。

**收益**：
- M6 报表查 inventory_move 一张表能拿全部业务流（不必跨表 JOIN）
- GSP 8.113"出库批号追溯到入库批号"DAG 自动满足
- 端到端追溯 SQL 简单（递归 CTE）

**成本**：1 个月（数据模型 1 周 + 业务模块对接 2 周 + M6 报表统一 3 天）。
**实施 Wave**：Wave 2-3 启动 M3 业务规则时大改。

---

### 3. ir.sequence 双实现 → M-CG `code_sequence`

**Odoo 原型**（`odoo/addons/base/models/ir_sequence.py` 375 行）：

- `standard` 实现：PostgreSQL 原生 `CREATE SEQUENCE`，性能高，**允许 gap**（事务回滚不还号）
- `no_gap` 实现：`SELECT FOR UPDATE NOWAIT` 行锁 + 应用层自增，**无 gap**（合规场景）
- `prefix/suffix` 模板：`'INV/%(year)s/%(month)s/%(day)s/'` + padding=5
- `use_date_range`：按时段（年/月）子序列

**wms 落地**：

```sql
CREATE TABLE code_sequence (
    code            VARCHAR(64) UNIQUE NOT NULL,    -- 'asn' / 'outbound' / 'destroy'
    prefix          VARCHAR(64),
    suffix          VARCHAR(64),
    padding         INTEGER DEFAULT 5,
    implementation  VARCHAR(20) DEFAULT 'standard', -- standard|no_gap
    pg_seq_name     VARCHAR(64),                    -- standard 模式的 PG SEQUENCE 名
    number_next     BIGINT DEFAULT 1,               -- no_gap 模式自增计数
    use_date_range  BOOLEAN DEFAULT false,
    owner_id        BIGINT                          -- 多货主隔离
);
```

**关键改进**：
- M-CG 当前故事**没明示双实现**——必须明示
- 销毁单号 / 发票号 / 监管上报流水号必须用 `no_gap`（合规）
- 普通 ASN / 出库单可用 `standard`（性能）

**成本**：4 天（schema + Rust 实现 + M-CG 故事增强）。
**实施 Wave**：Wave 1-2 启动 M-CG 时落地。

---

### 4. `__manifest__.py` → wms `module-manifest.toml` 模块依赖图工具化

**Odoo 原型**（`odoo/modules/module.py` 中的 `Manifest` 类）：

```python
{
    'name': 'Inventory',
    'depends': ['product', 'barcodes_gs1_nomenclature', 'digest'],
    'data': [...],         # 数据加载顺序
    'auto_install': False, # 依赖装齐自动装
    'post_init_hook': '_init',
    'installable': True,
}
```

`Manifest.all_addon_manifests()` 扫描所有 addons → `module_graph.py` 拓扑排序 → 启动时按依赖图加载。

**wms 落地**：每个模块加 `module-manifest.toml`（数据，非代码）：

```toml
# domain/m-tc/module-manifest.toml
[module]
code = "M-TC"
name = "追溯码模块"
version = "v3.1"

[depends]
business = ["M1.master-data", "M-CG"]
horizontal = ["H1", "H2"]
external = ["barcodes_gs1_rules"]

[data]
gs1_nomenclature = "data/gs1_default.toml"

[lifecycle]
post_install = "post_init_load_gs1.sql"
```

新增治理脚本 `check_module_dependencies.py`：
- 扫描所有 module-manifest.toml
- 自动构建依赖图
- 与 architecture-dependencies.md 对比一致性
- diff 触发：模块改动 → 校验 manifest 中的 depends 不被破坏

**成本**：1-2 天（29 个模块每个 ~30 行 manifest + 治理脚本）。
**实施 Wave**：**Wave 0 内可做**（治理增强，立即受益）。

---

### 5. GS1 nomenclature → M-TC `trace_code_nomenclature` + `trace_code_rule`

**Odoo 原型**（`addons/barcodes/models/barcode_nomenclature.py` 215 行 + `barcode_rule.py` 47 行 + 前端 `barcode_parser.js` 306 行）：

```python
class BarcodeNomenclature(models.Model):
    name      = ...                # 'GS1-128' / 'EAN-13'
    rule_ids  = ...                # 一对多

class BarcodeRule(models.Model):
    pattern   = ...                # '01{N20}17{N6}'  GS1 占位符
    type      = ...                # 'product'/'expiry_date'/'lot'/'serial'
    encoding  = ...                # 'any'/'ean13'/'ean8'/'upca'
    alias     = ...
```

**核心创新**：所有 GS1 AI 规则**数据驱动**，不硬编码。运行时可加新命名法（如中国药监 20 位监管码）。

**wms 落地**：

```sql
CREATE TABLE trace_code_nomenclature (
    id          BIGSERIAL PRIMARY KEY,
    name        VARCHAR(64) NOT NULL,    -- 'GS1-128'/'中国药监 20 位码'/'UDI'
    is_default  BOOLEAN DEFAULT false,
    owner_id    BIGINT
);

CREATE TABLE trace_code_rule (
    id              BIGSERIAL PRIMARY KEY,
    nomenclature_id BIGINT REFERENCES trace_code_nomenclature(id),
    sequence        INTEGER,
    pattern         VARCHAR(256),         -- '01{N14}17{N6}'
    field_type      VARCHAR(32),          -- 'gtin'/'expiry'/'batch'/'serial'/'qty'
    encoding        VARCHAR(20),
    alias           VARCHAR(128)
);
```

**预置规则集**（按 module-manifest 自动加载）：
```toml
[[nomenclature]]
name = "GS1-128"
[[rules]]
pattern = "01{N14}"
field_type = "gtin"
[[rules]]
pattern = "17{N6}"
field_type = "expiry"
[[rules]]
pattern = "10{N0,20}"
field_type = "batch"

[[nomenclature]]
name = "中国药监 20 位监管码"
[[rules]]
pattern = "{N20}"
field_type = "regulatory_code"
```

**Rust + TS 共享解析**：定义统一的 pattern matcher（前端 PDA 扫码也用同一算法）。

**成本**：2 周（数据模型 3 天 + Rust matcher 5 天 + TS 解析器 3 天 + M-TC 故事更新 1 天）。
**实施 Wave**：Wave 4（M-TC 实施时）。

---

### 6. ir.rule 行级访问控制 → H1 `record_rule` 引擎

**Odoo 原型**（`odoo/addons/base/models/ir_rule.py`）：

```python
class IrRule(models.Model):
    name        = ...
    model_id    = ...                    # 适用模型
    domain_force = ...                   # 域表达式
    groups      = ...                    # 角色绑定
    perm_read   = perm_write = perm_create = perm_unlink = ...
```

ORM 自动注入 WHERE 子句到所有查询：
```sql
SELECT * FROM stock_move WHERE ...
  AND <ir.rule.domain_force 自动注入>
```

**关键案例**：
- "养护员只能看自己负责的批号"
- "货主 A 用户绝不能查到货主 B 的库存"（多货主隔离基础）
- "夜班角色只能 read 不能 write"

**wms 当前缺口**：H1 故事提到"tenant_id 自动注入"，但**没有更细粒度的行级规则**。GSP 5.71 / 5.72 要求"按角色限制操作权限"，行级规则是合规关键。

**wms 落地**：

```sql
CREATE TABLE record_rule (
    id              BIGSERIAL PRIMARY KEY,
    name            VARCHAR(128) NOT NULL,
    model_table     VARCHAR(64) NOT NULL,   -- 'inventory'/'inbound_order'
    domain_sql      TEXT NOT NULL,          -- 'owner_id = current_setting(''app.owner_id'')'
    role_ids        BIGINT[] NOT NULL,
    perm_read       BOOLEAN DEFAULT true,
    perm_write      BOOLEAN DEFAULT true,
    perm_create     BOOLEAN DEFAULT true,
    perm_unlink     BOOLEAN DEFAULT true,
    enabled         BOOLEAN DEFAULT true
);
```

**Rust 实现**：在 sqlx 查询中间件层根据当前 `RequestContext` 自动追加 WHERE 子句（参考 axum middleware）。

**预置规则**（H1 启动时自动加载）：
- 多货主隔离（每个查询追加 owner_id 过滤）
- 角色行级（养护员仅看自己批号）
- GSP 双人作业（关键操作必须双人验证）

**成本**：1.5 周（schema + middleware + 5-10 条预置规则）。
**实施 Wave**：**Wave 1 W1.A 实施 H1 时同步落地**。

---

### 7. ir.model.access.csv 声明式权限 → H1 `permission_matrix.csv`

**Odoo 原型**（`odoo/addons/base/security/ir.model.access.csv`）：

```csv
"id","name","model_id:id","group_id:id","perm_read","perm_write","perm_create","perm_unlink"
"access_ir_attachment_group_user","ir_attachment group_user","model_ir_attachment","group_user",1,1,1,1
"access_ir_cron_group_cron","ir_cron group_cron","model_ir_cron","group_system",1,1,1,1
```

**优势**：
- 权限矩阵不是写在代码里，而是数据驱动
- 业务方修改权限不需要发版
- 治理脚本可校验"每个模型 × 每个角色"覆盖完整

**wms 当前缺口**：H1-006 用户与权限管理故事提到"角色配置"但没明示**模型级 RBAC 矩阵的 CSV 数据驱动模式**。

**wms 落地**：

```csv
# governance/permission-matrix.csv
id,name,model,role,read,write,create,unlink
M3_INV_KEEPER,"库存查询 - 保管员",inventory,keeper,1,0,0,0
M3_INV_MAINT,"库存修改 - 养护员",inventory,maintainer,1,1,0,0
M3_INV_MGR,"库存全权 - 仓库主管",inventory,warehouse_manager,1,1,1,1
M-SA_DEST_QA,"销毁审批 - 质量负责人",destroy_order,qa_manager,1,1,0,0
```

启动时加载到内存 + 每次请求查询。

**成本**：3 天（schema + 加载器 + 治理脚本 `check_permission_matrix.py` 检查每个模型必有至少 1 条规则）。
**实施 Wave**：**Wave 1 W1.A 与 ir.rule 同步落地**。

---

### 8. TransientModel 向导 → wms `wizard` Rust 模式

**Odoo 原型**：`TransientModel` 是临时表，自动定期清理（默认每天清理 ≥1 小时前的数据）。

**用途案例**：
- 盘点向导（多步操作：选范围 → 录入差异 → 确认审批）
- 双人验收向导（保存第一签字人 → 等第二签字人）
- 销毁向导（含多步审批）
- 批量上架向导（批量操作前确认）

**wms 当前缺口**：当前没有"向导"概念，临时数据散在业务表里（如临时审批状态可能与正式状态混淆）。

**wms 落地**：

```rust
// 通用向导框架
pub trait Wizard: Serialize + Deserialize<'static> {
    type Result;
    fn name() -> &'static str;
    fn ttl_minutes() -> u32 { 60 }  // 默认 1 小时
    async fn execute(&self, ctx: &Context) -> Result<Self::Result>;
}

// 具体向导
#[derive(Serialize, Deserialize, Wizard)]
#[wizard(name = "stocktake", ttl = 240)]  // 4 小时
pub struct StocktakeWizard {
    pub scope: StocktakeScope,
    pub recorded_diffs: Vec<StockDiff>,
    pub approval_state: ApprovalState,
}
```

存储：Postgres 表 `wizard_session` + `pg_cron` 定期清理。

**收益**：
- 多步操作不污染主业务表
- 可中断恢复（用户中途登出可继续）
- TTL 自动清理避免数据库膨胀

**成本**：1 周（trait + 持久层 + 清理 cron）。
**实施 Wave**：Wave 2（基础 CRUD 时）。

---

### 9. state button 命名约定 → wms 全局方法名约定

**Odoo 原型**（`stock.picking` / `stock.move`）：
```python
def button_validate(self):     # 状态：confirmed → done
def button_confirm(self):      # 状态：draft → confirmed  
def button_cancel(self):       # → cancel
def action_done(self):
def action_assign(self):       # 库存预占
```

**约定价值**：
- 阅读代码立即知道是"状态转换"还是"业务计算"
- 治理脚本可扫描"所有状态转换方法"自动生成审计点
- 前端 button 与后端方法名约定对齐

**wms 落地**（Rust 命名约定）：

```rust
impl InboundOrder {
    pub async fn confirm(&mut self, ctx: &Ctx) -> Result<()>;     // draft → confirmed
    pub async fn validate(&mut self, ctx: &Ctx) -> Result<()>;    // confirmed → done
    pub async fn cancel(&mut self, ctx: &Ctx) -> Result<()>;      // any → cancelled
    pub async fn assign(&mut self, ctx: &Ctx) -> Result<()>;      // 库存预占
    pub async fn revert(&mut self, ctx: &Ctx) -> Result<()>;      // 撤回到上一状态
}
```

**前缀约定**：
- `confirm_` / `validate_` / `done_` / `cancel_` / `assign_` / `revert_` = 状态转换
- `compute_` = 派生字段计算
- `query_` / `search_` = 只读
- `import_` / `export_` = 数据交换

**治理脚本扩展**：`check_state_transition_audit.py` 扫描这些前缀方法名，确保都接入审计追踪。

**成本**：半天（写入 docs/coding-standards.md）。
**实施 Wave**：**Wave 1 立即采纳**（写入编码规范，Rust 代码全程遵循）。

---

## 决策影响

### 立即影响（Wave 0 末 / Wave 1 启动时）

- **Wave 0 内做**：#4 模块 manifest（1-2 天）、#9 命名约定（半天写入编码规范）
- **Wave 1 W1.A H1 实施时**：#6 行级规则 + #7 权限矩阵 CSV（共 2 周）
- **Wave 1 W1.B H2 实施时**：#1 Auditable trait + macro（1.5 周）
- **Wave 1-2 M-CG**：#3 双实现序列（4 天）

### 中期影响（Wave 2）

- **Wave 2 W2.A**：#8 wizard 框架（1 周）

### 远期影响

- **Wave 2-3 W3.B**：#2 inventory_move 三段式（1 个月，重构 M3 库存模型）
- **Wave 4 W4.D**：#5 GS1 nomenclature 数据驱动（2 周）

### 总工作量

约 **2.5 个月增量**（分散在 Wave 0-4），换取 odoo 验证 10+ 年的成熟设计模式。

---

## 风险与替代方案

| 风险 | 应对 |
|---|---|
| Rust derive macro 复杂度高（#1）| 先实现 trait，macro 在 Wave 1 末再做（先手写也可工作）|
| inventory_move 重构破坏现有 M3 schema（#2）| Wave 2 启动 M3 实施时一并设计，不在 Wave 0 改 schema |
| ir.rule SQL 注入风险（#6）| domain_sql 必须经参数化校验 + 白名单过滤；不允许动态拼接 |
| 中国药监码 20 位与 GS1 共存（#5）| 两个独立 nomenclature；扫码时按 owner 默认选择 |

替代方案：放弃借鉴某项，自行设计或购买商业 WMS。但 odoo 的成熟度和社区验证使借鉴是最优解。

## 后果

### 正面

- **避免重复造轮子**：9 项借鉴对应的设计都是 odoo 验证 10+ 年的成熟模式，wms 不必从零摸索
- **GSP 合规增强**：#1（Auditable）+ #6（行级规则）+ #7（声明式权限）+ #3（no_gap 序列）直接强化 GSP 5.67 / 5.71 / 5.72 合规度
- **代码可读性**：#9 命名约定让所有状态转换方法统一前缀，治理脚本可自动扫描审计点
- **架构一致性**：#2 inventory_move 三段式让所有库存变动事件统一抽象，M6 报表/审计/追溯查询简化
- **可配置性**：#5 GS1 nomenclature + #7 权限矩阵 CSV 让运营期变更无需发版

### 负面

- **学习成本**：团队需要理解 odoo 的设计哲学才能正确借鉴（不是照搬代码）
- **重构成本**：#2 inventory_move 三段式需要在 Wave 2-3 大改 M3 schema，影响业务模块对接
- **迁移风险**：#1 Auditable trait 需要 derive macro 体系，Rust 团队首次实现可能踩坑
- **总工作量增加**：约 2.5 个月增量分散到 Wave 0-4

### 风险

- **过度抽象**：借鉴本意是参考，但实施时可能过度复刻 odoo 的 ORM 思路，与 Rust 习惯冲突
- **应对**：每项借鉴在实施 Wave 启动时**先做最小可行版本**（trait + 基础实现），观察 1 个月再决定是否扩展为完整 macro 体系
- **GSP 合规边界**：#6 行级规则的 SQL 注入风险（已列入风险表）

### 撤销机制

- 每项借鉴是独立决策；可单独撤销而不影响其他
- 撤销路径：单独 ADR（如 ADR-0010 撤销 #2）+ 数据迁移方案

## 实施约束

1. 所有 9 项的具体实施代码必须在对应 Wave 启动时**写新故事或更新现有故事**承载，不直接进入代码
2. 每项实施前应先**通读 odoo 对应源码**（路径已在本 ADR 标注），不能仅凭"听说"借鉴
3. **不得跨 Wave 提前实施**（违反 ADR-0007 节奏铁律）
4. **/tmp/odoo 必须保留**到 Wave 4 完成，作为实施期参考底本

---

## 不在范围

- 不照搬 Odoo 代码（不同语言）
- 不引入 Odoo 模块加载器（按 Cargo workspace 管理依赖）
- 不引入 OWL 前端框架（继续用 React + shadcn/ui）
- 不引入 QWeb 模板（继续用 H9 打印模板引擎设计）

---

## 关联

- ADR-0001 技术栈（Rust + Axum + sqlx + React）
- ADR-0007 v0.3 路线（5 波次划分）
- 调研产物：`/tmp/odoo` 保留备查
- 影响故事：H1 / H2 / M-CG / M-TC / M3 / M-SA / 全部业务模块（命名约定）

## 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-05-18 | v1 | 初版：基于 Odoo master 源码深入分析 9 个借鉴设计 + 实施 Wave 划分 |
