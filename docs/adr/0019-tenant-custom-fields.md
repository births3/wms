# ADR-0019：货主自定义属性（Tenant Custom Fields，简化版借鉴 Odoo Studio）

- 状态：Accepted
- 决策日期：2026-05-18
- 决策人：项目主人
- 关联：ADR-0008 §10（借鉴 Odoo 第 10 项）/ ADR-0013 配置 secrets / ADR-0014 数据迁移 / M1-008 业务配置中心 / pattern-extraction 报告 §5.1

---

## 背景

3PL 多货主场景下，每个货主对核心聚合根（商品 / 供应商 / 库存批次 等）都可能有自己的扩展属性需求：

- 货主 A：需要"批准文号有效期"
- 货主 B：需要"GMP 证书号"
- 货主 C：需要"GSP 等级"

如果每个货主的需求都走 sqlx schema migration（ADR-0014），后果：

- 字段无止境膨胀（10 个货主 × 5 个属性 = 50 个新列）
- 每次新货主上线都要改代码 + 部署
- 不同货主的字段相互"看到"（即使 RLS 隔离数据，schema 是共享的）

Odoo 通过 **Studio + ir.model.fields** 实现"用户可加字段不改代码"。其核心思想可简化借鉴，避开 Odoo 完整方案与 Rust 静态类型的根本冲突。

---

## 候选方案

### 方案 A（推荐）：JSONB 列 + 配置中心 schema

每个支持自定义的聚合根加一个 `extra_attrs JSONB` 列。M1-008 配置中心定义"哪个货主对哪个聚合根有哪些属性 + 类型约束"。Rust 端用 `serde_json::Value` + 运行时 schema 校验。PG GIN 索引保证查询性能。

### 方案 B：独立扩展表（每个属性一行）

```
custom_field_value(tenant_id, aggregate_id, field_key, value)
```

**否决**：JOIN 多 / 类型不统一（value 必须是 TEXT）/ Rust 端处理繁琐 / 高频查询性能差。

### 方案 C：完整 Odoo 元数据 ORM 仿制

完整复制 ir.model + ir.model.fields + ir.ui.view 机制。

**否决**：
- 工作量 8-10 周
- Rust 静态类型 + SQLx 编译期 SQL 检查 与"运行时动态字段"哲学根本冲突
- 维护成本极高（需自建 ORM 元数据系统）

---

## 决策

**采用方案 A：JSONB 列 + 配置中心 schema 定义**。

### 1. 数据模型

```sql
-- 支持的聚合根都加这一列
ALTER TABLE inventory_batch
  ADD COLUMN extra_attrs JSONB NOT NULL DEFAULT '{}'::jsonb;

-- GIN 索引保证 JSONB 查询性能
CREATE INDEX idx_inventory_batch_extra_attrs_gin
  ON inventory_batch USING GIN (extra_attrs);

-- 多租户 RLS 策略 + extra_attrs 一致按 tenant_id 隔离
```

### 2. 适用聚合根（白名单）

不是所有聚合根都允许 custom fields：

| 聚合根 | 是否支持 | 理由 |
|---|---|---|
| Product（商品） | ✅ | 货主商品扩展属性高频需求 |
| Supplier（供应商） | ✅ | 资质类扩展 |
| Customer（客户） | ✅ | 客户分类标签 |
| InventoryBatch（库存批次） | ✅ | 批次特殊属性 |
| InboundOrder（ASN 收货单） | ✅ | 业务定制 |
| OutboundOrder（出库单） | ✅ | 业务定制 |
| User（用户） | ❌ | 安全敏感，禁止扩展 |
| AuditEvent（审计） | ❌ | append-only 不变 |
| Permission（权限） | ❌ | 安全敏感 |
| 其他系统对象 | ❌ | 维稳 |

### 3. 配置中心 schema 定义（M1-008 新增类目 `tenant_custom_fields`）

```yaml
# 存于 M1-008 业务配置中心
tenant_custom_fields:
  - tenant_id: "owner-a"
    aggregate: "product"
    field_key: "x_approval_doc_expiry"   # 强制 x_ 前缀（借鉴 Odoo Studio）
    field_label: "批准文号有效期"
    field_type: "date"                   # string/int/float/bool/date/datetime/enum
    required: false
    default_value: null
    constraints:
      pattern: null                      # 仅 string 适用，regex 校验
      min: null                          # int/float/date 适用
      max: null
      enum_values: null                  # 仅 enum 适用
    description: "批准文号到期日期"
    created_by: "owner-a-admin"
    created_at: "2026-06-01T00:00:00Z"

  - tenant_id: "owner-b"
    aggregate: "supplier"
    field_key: "x_gmp_cert_no"
    field_label: "GMP 证书号"
    field_type: "string"
    required: true
    constraints:
      pattern: "^GMP[0-9]{8}$"
    description: "GMP 认证编号"
```

### 4. Rust 端校验

```rust
// backend/crates/domain/src/custom_fields/mod.rs

#[derive(Debug)]
pub struct CustomFieldValidator {
    schema_cache: Arc<RwLock<HashMap<(TenantId, AggregateName), Vec<FieldDef>>>>,
}

impl CustomFieldValidator {
    /// 校验 extra_attrs 是否符合该 tenant + aggregate 的 schema
    pub fn validate(
        &self,
        tenant_id: &TenantId,
        aggregate: AggregateName,
        attrs: &serde_json::Value,
    ) -> Result<(), CustomFieldError> {
        // 1. 从 schema_cache 加载该 tenant + aggregate 的字段定义
        let schema = self.load_schema(tenant_id, aggregate)?;

        // 2. 白名单模式：拒绝未声明的字段
        for key in attrs.as_object().unwrap().keys() {
            if !schema.iter().any(|f| f.field_key == *key) {
                return Err(CustomFieldError::UnknownField(key.clone()));
            }
            if !key.starts_with("x_") {
                return Err(CustomFieldError::InvalidPrefix(key.clone()));
            }
        }

        // 3. 逐字段校验类型 / required / constraints
        for field_def in &schema {
            match attrs.get(&field_def.field_key) {
                None if field_def.required => {
                    return Err(CustomFieldError::Required(field_def.field_key.clone()));
                }
                Some(value) => {
                    field_def.validate_value(value)?;
                }
                None => {} // optional 字段缺失允许
            }
        }
        Ok(())
    }
}
```

### 5. 命名约束

- 所有 `field_key` 必须以 **`x_` 前缀**（借鉴 Odoo Studio 命名空间隔离）
- 同一 `tenant_id + aggregate` 内 `field_key` 全局唯一
- 命名风格 `snake_case`（与 GSP 字段词典一致）

### 6. UI / 管理流程

PC 端管理界面（Wave 5 实施）：

```
M1-008 配置中心 → 自定义字段管理
  ├─ 货主管理员：添加/修改/删除本货主自定义字段
  ├─ 系统管理员：审批所有变更
  ├─ 字段添加后：立即生效（不重启服务）
  └─ 字段删除：进入"已弃用"状态，30 天后真删（保护已写数据）
```

变更生效机制：

- 配置变更触发 `tenant_custom_fields.updated` 事件（H2-005 事件总线）
- `CustomFieldValidator` 订阅事件，刷新 `schema_cache`
- 不需要重启服务

### 7. 查询能力

PG JSONB 原生查询：

```sql
-- 查询货主 A 批准文号 2027 年内到期的所有批次
SELECT * FROM inventory_batch
WHERE tenant_id = 'owner-a'
  AND (extra_attrs->>'x_approval_doc_expiry')::date < '2027-12-31'
  AND (extra_attrs->>'x_approval_doc_expiry')::date >= CURRENT_DATE;

-- GIN 索引支持的查询模式
-- @> 包含 / ? key 存在 / ?| 任一存在 / ?& 全部存在
```

### 8. 治理

新增治理脚本 `scripts/governance/check_custom_fields_schema.py`：

- 校验 `field_key` 全部 `x_` 前缀
- 校验 `field_type` 在合法枚举内
- 校验 `tenant_id` 在 M1-007 货主白名单
- 校验 `aggregate` 在白名单（§2 表）
- 校验 `field_key` 在同 tenant + aggregate 内唯一
- 校验 GSP 字段词典中的 canonical 字段不被 custom field 覆盖（防止用 custom field 替代法定字段）

---

## 后果

### 正面

- **货主自助**：货主可自定义属性而不改代码
- **零阻塞**：Wave 5 实施，不阻塞 MVP（Wave 1-4）
- **类型安全可控**：Rust 端运行时校验 + 配置中心 schema 双保险
- **性能可接受**：GIN 索引支持高效 JSONB 查询
- **借鉴成熟实践**：Odoo 多年验证的命名空间隔离（`x_` 前缀）+ schema 元数据驱动思想

### 负面

- **跨货主查询能力弱**：不同货主 schema 不同，难统一聚合查询
- **应对**：跨货主报表只用标准列，custom field 仅用于本货主业务
- **类型校验在运行时**（不是编译期），bug 较晚才暴露
- **应对**：单元测试覆盖 + 治理脚本静态检查 schema
- **JSONB 字段无法直接 JOIN**：跨表关联用 custom field 性能差
- **应对**：custom field 仅用于"叶子属性"（非关联），关联用标准列

### 风险

- **滥用**：开发者图省事把所有字段都丢 JSONB → schema 治理失败
- **应对**：白名单聚合根 + 治理脚本 + PR review 强制
- **GSP 字段误用**：custom field 替代 GSP 法定字段 → 合规风险
- **应对**：治理脚本 `check_custom_fields_schema.py` 强制 GSP canonical 字段不可被 custom field 覆盖
- **schema 变更未同步生效**：配置改了但缓存没刷新 → 校验失败
- **应对**：H2-005 事件总线触发 cache 刷新 + 兜底定时刷新（每 5 分钟）

---

## 实施约束

1. 所有 custom field 的 `field_key` 必须 `x_` 前缀（强约束，治理脚本拦截）
2. 必须经 M1-008 配置中心声明，不允许代码直接 INSERT JSONB
3. 不能用作 GSP 法定字段（GSP 字段必须是 schema 列，单一事实之源）
4. 不能用于聚合根的"真不变量"（business invariant 必须强类型，参 coding-standards §1.1.2）
5. **本 ADR 在 Wave 5 实施**（增值模块阶段），Wave 1-4 不实施
6. 删除字段需 30 天宽限期（保护已写数据，期间字段标记"已弃用"）

---

## 与其他 ADR 的关系

| ADR | 关系 |
|-----|------|
| ADR-0008 §10 | 本 ADR 是 "借鉴 Odoo 9 项" 升级到第 10 项的实施 ADR |
| ADR-0013 配置 secrets | custom field schema 存于 M1-008 业务配置中心（L3 运行时配置） |
| ADR-0014 数据迁移 | legacy → wms 迁移时 custom field 按各 tenant schema 转换；30 年保留期数据迁移特殊药品的 custom field 同步迁移 |
| ADR-0018 弹性工程 | schema cache 失效降级到配置中心直查（D2 数据降级） |
| pattern-extraction §5.1 | 14 个已落地模式中的"配置中心 (M1-008)"是本 ADR 的依赖 |
| coding-standards §1.1.2 | 真不变量必须强类型 → custom field 仅用于非不变量属性 |

---

## 实施时点

**Wave 5 W5.x**（增值模块全面铺开阶段）。

理由：
- Wave 1 H 层 / Wave 2-3 核心业务 / Wave 4 完整闭环 都不需要 custom field
- 单货主小型 3PL 场景用标准列已足够
- 多货主大型 SaaS 场景才需要本能力（Wave 5 时点匹配）

Wave 5 启动时新建 W5.x 任务行登记本 ADR 实施。

---

## 参考

- Odoo Studio: https://www.odoo.com/documentation/master/applications/studio.html
- Odoo `ir.model.fields`: https://github.com/odoo/odoo/blob/master/odoo/addons/base/models/ir_model.py
- PostgreSQL JSONB indexing: https://www.postgresql.org/docs/current/datatype-json.html#JSON-INDEXING

## 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-05-18 | v1 | 初版：JSONB + 配置中心 schema + 6 聚合根白名单 + `x_` 前缀命名 + 运行时 Rust 校验 + 治理脚本；Wave 5 实施 |
