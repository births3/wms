# GSP 字段编码规范（Field Coding Standards）

> 时间：2026-05-17
> 版本：v1
> 文档层级：L2 规范（必须遵守）
> 关联：[gsp-field-traceability.md](gsp-field-traceability.md) §6 字段词典；[ADR-0001](../adr/0001-tech-stack.md) 技术栈

---

## 1. 目的

本文档规定医药 WMS 字段的**实现层规范**。所有新增字段必须遵守本规范；已有字段如违反本规范，必须列入 baseline 并制定整改计划。

> 与 [gsp-field-traceability.md](gsp-field-traceability.md) 的区别：traceability 矩阵记录 70 个 GSP 强制字段的合规追溯；本文档约束**所有字段**的实现规范。

---

## 2. 命名规范

### 2.1 数据库列名

- **小写 + 下划线**（snake_case）：`batch_no` / `expire_date` / `temp_record_at`
- **缩略词约定**：`id`（不写 ID）/ `no`（不写 number）/ `dt`（datetime）/ `ts`（timestamp，秒级以下）
- **业务量纲后缀**：`_qty`（数量）/ `_amount`（金额）/ `_temp`（温度）/ `_at`（时间戳）/ `_date`（日期）
- **布尔字段**：`is_*` / `has_*` / `requires_*`（如 `is_recalled` / `has_dual_person` / `requires_dedicated_ledger`）
- **外键**：`<entity>_id`（如 `user_id` / `product_id`）；多角色字段加角色前缀（`receiver_user_id` / `verifier_user_id`）

### 2.2 Rust struct 字段

- 与 SQL 列名一致：`batch_no: String` / `expire_date: NaiveDate`
- 不在 Rust 端转 camelCase（避免序列化层歧义；前端转换在 OpenAPI 层）

### 2.3 OpenAPI / 前端

- 跨端契约层（OpenAPI schema）：与 SQL 一致 snake_case
- 前端 React 组件内部可用 camelCase（由 `openapi-typescript` 自动映射）

### 2.4 GSP 法规对应

中文术语（如"批号"）在文档/UI 显示，英文 code（如 `batch_no`）在数据库/接口。映射表见 [glossary.md](../glossary.md) 及 [gsp-field-traceability.md](gsp-field-traceability.md) §6 字段词典。

---

## 3. 数据类型规范

### 3.1 基本类型映射

| 业务语义 | PostgreSQL | Rust | 备注 |
|----|----|----|----|
| 短文本（编码、状态）| `VARCHAR(N)` | `String` | N ≤ 32 |
| 长文本（描述、备注）| `TEXT` | `String` | 无长度限制 |
| 整数 ID | `BIGINT` | `i64` | 不用 `INT`（防止溢出）|
| 有界计数 / 配置阈值 | `INT` | `i32` | 仅限非 ID 字段，必须有范围校验 |
| 数量 | `NUMERIC(15,3)` | `Decimal` | 整数 12 位 + 小数 3 位 |
| 金额 | `NUMERIC(15,2)` | `Decimal` | 不用 `FLOAT` / `DOUBLE`（精度问题）|
| 温度 | `NUMERIC(5,2)` | `Decimal` | 范围 `-99.99 ~ 999.99` |
| 时间戳 | `TIMESTAMPTZ` | `DateTime<Utc>` | 必须带时区 |
| 日期（无时间）| `DATE` | `NaiveDate` | 如生产日期、有效期 |
| 布尔 | `BOOLEAN` | `bool` | 不用 0/1 整数代替 |
| 枚举 | `VARCHAR(20)` + CHECK | `enum` | Rust enum 通过 sqlx 序列化 |
| 复杂结构 | `JSONB` | `serde_json::Value` 或具体 struct | 资质、温度记录、随货同行单 |
| IP 地址 | `INET` | `IpAddr` | 不用 VARCHAR |

### 3.2 禁用类型

- ❌ `FLOAT` / `DOUBLE`：财务、数量类不允许（精度损失）
- ❌ ID 类字段使用 `INT` / `SERIAL`：主键统一 `BIGINT` / `BIGSERIAL`（防止 21 亿行溢出）；非 ID 的有界计数 / 配置阈值可用 `INT`，但必须声明范围校验
- ❌ `TIMESTAMP`（不带时区）：必须 `TIMESTAMPTZ`
- ❌ `CHAR(N)` 定长：用 `VARCHAR(N)` 或 `TEXT`
- ❌ 数组类型 `ARRAY[]`：除非有明确不可变集合语义（如 `经营范围 TEXT[]`）

### 3.3 长度规范

| 字段类型 | 最大长度 | 理由 |
|----|----|----|
| 业务编码（商品/客户）| `VARCHAR(32)` | 兼容现有 ERP / 国家药管平台 |
| 名称（商品名/厂家）| `VARCHAR(128)` | GB 18031 中文最长 |
| 批号 | `VARCHAR(20)` | GMP 实务最长 16 位，留余量 |
| 有效期 / 生产日期 | `DATE` | 无小时分钟（合规要求精度到日）|
| 追溯码 | `VARCHAR(32)` | 兼容 8/12/17/20 位多种标准 |
| 库位 | `VARCHAR(32)` | 含层、列、位的复合编码 |
| 用户姓名 | `VARCHAR(64)` | 兼容外籍员工长姓名 |
| 描述/备注 | `TEXT` | 不限制 |

---

## 4. 校验规则

### 4.1 数据库层（CHECK 约束）

```sql
-- 批号格式
ALTER TABLE inventory ADD CONSTRAINT chk_batch_no
    CHECK (batch_no ~ '^[A-Za-z0-9]{1,20}$');

-- 有效期 > 生产日期
ALTER TABLE inventory ADD CONSTRAINT chk_expire_after_produce
    CHECK (expire_date > produce_date);

-- 数量非负
ALTER TABLE inventory_transaction ADD CONSTRAINT chk_qty_signed
    CHECK (variation_qty != 0);  -- 变动数量正负皆可，但不能为 0

-- 温度范围
ALTER TABLE temp_record ADD CONSTRAINT chk_temp_range
    CHECK (temp_celsius BETWEEN -50 AND 50);
```

### 4.2 应用层（Rust validator / sqlx）

```rust
#[derive(Debug, Deserialize, Validate)]
struct ProductCreate {
    #[validate(regex = r"^[A-Z0-9]{6,32}$")]
    code: String,

    #[validate(length(min = 1, max = 128))]
    name: String,

    #[validate(custom = "validate_approval_no")]
    approval_no: String,  // 批准文号正则
}
```

### 4.3 校验位置

每个字段必须在以下至少 2 层校验：

1. **OpenAPI schema**（前端调用前）
2. **Rust handler**（API 入口）
3. **Domain service**（业务规则）
4. **数据库 CHECK 约束**（最后防线）

GSP 关键字段（70 字段中 audit_required=true）必须 4 层全覆盖。

---

## 5. 加密与脱敏

### 5.1 加密分级

| 等级 | 含义 | 加密方式 | 适用字段 |
|----|----|----|----|
| `none` | 公开数据 | 无 | 商品编码 / 批号 / 库位 |
| `masked` | 显示脱敏 | 应用层屏蔽（如 `186****1234`）| 手机号 / 身份证 / IP 地址 |
| `encrypted` | 存储加密 | AES-256-GCM 列级加密 | 法人身份证 / 银行账号 / 患者资料 |

### 5.2 当前 70 字段加密分布

```
none:       68  (主流业务字段)
masked:      2  (IP 地址 / 后续：手机号、身份证)
encrypted:   0  (Wave 1+ 加患者数据时启用)
```

### 5.3 加密实现（Wave 1+）

- 列级加密：`pgcrypto` 扩展 + 应用层透明加解密
- 密钥管理：env 注入 `MASTER_KEY` → 派生数据密钥（DEK）
- 备份解密：备份脚本只能拿到密文 + DEK 加密版，不能直接解密
- 审计：解密访问记入 `audit_trail`（actor=system / 操作时间 / IP）

---

## 6. 审计要求

### 6.1 audit_required=true 的字段

70 字段中 100% 标记 `audit_required=true`（GSP 强制）。任何变更必须：

1. 写入 `audit_trail` 表（旧值 / 新值 / 操作人 / 时间 / IP / 审批源）
2. `audit_trail` 是 append-only（不允许 UPDATE / DELETE）
3. 保留 ≥ 5 年（普通）/ ≥ 30 年（放射性）

### 6.2 审计触发位置

- 数据库层：触发器（trigger）兜底（Wave 3+）
- 应用层：Rust domain service 显式调用 `audit_log!` 宏
- 接口层：所有 PUT/PATCH/DELETE 必须有审计

### 6.3 audit_required=false 的字段

允许直接更新，无需审计：

- 看板缓存字段（如 `last_view_at`）
- 描述字段（备注）
- 推送状态（如 `last_notified_at`）

---

## 7. 默认值与不可变性

### 7.1 不可变字段（写入后禁止 UPDATE）

| 字段 | 理由 |
|----|----|
| `id` (主键)| 全局唯一标识 |
| `created_at` | 审计追溯起点 |
| `created_by` | 责任人锁定 |
| `batch_no` / `produce_date` / `expire_date` | GSP：批次锁定后不允许修改 |
| 分区键（`effective_date`）| 改键会触发跨分区 UPDATE |

实现：数据库层加 trigger `BEFORE UPDATE` 抛错。

### 7.2 默认值规则

- `created_at` / `updated_at`：`DEFAULT now()`
- `is_*` 布尔：明确 `DEFAULT false` 或 `DEFAULT true`
- 状态字段：必须有初始状态默认值（如 `status DEFAULT '草稿'`）
- ❌ 禁止：业务字段无默认且 NOT NULL（强迫调用方填，但又没合理空值）

---

## 8. 索引规范

### 8.1 必建索引

| 场景 | 索引 |
|----|----|
| 主键 | `PRIMARY KEY` 自动 |
| 外键 | `FOREIGN KEY` 字段必须建索引 |
| 时间分区表 | 分区键（如 `created_at`）|
| 业务唯一约束 | `UNIQUE INDEX (batch_no, product_id, owner_id)` |
| 报表查询路径 | `(owner_id, warehouse_id, created_at DESC)` |

### 8.2 索引命名

`idx_<table>_<col1>[_<col2>...]` / `uk_<table>_<col1>` / `pk_<table>`

---

## 9. 治理脚本

| 脚本 | 校验项 |
|----|----|
| [check_gsp_field_traceability.py](../../scripts/governance/check_gsp_field_traceability.py) | 70 GSP 字段在故事字段表实现 + 5 项技术属性完整性 |
| `check_field_coding_standards.py` | （Wave 1 待实现）字段命名 / 类型 / 加密 / 审计是否符合本规范 |

---

## 10. 与其他文档的关系

```
gsp-field-coding-standards.md（本文档，规则源头）
  │
  ├─→ gsp-field-traceability.md §6      （70 字段技术属性数据 — 单一事实之源）
  ├─→ ADR-0001 技术栈                   （PostgreSQL + Rust + sqlx）
  ├─→ glossary.md                       （术语映射）
  ├─→ docs/coding-standards.md           （代码命名规范继承）
  └─→ scripts/governance/check_*         （治理脚本承接）
```

---

## 11. 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-05-17 | v1 | 初版：命名 / 类型 / 校验 / 加密 / 审计 / 默认值 / 索引 7 大规范，覆盖 70 GSP 字段 + 所有新增字段 |
