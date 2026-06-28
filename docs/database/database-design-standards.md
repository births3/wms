# 数据库设计与命名规范

> 本文档是 WMS PostgreSQL 表、字段、索引、约束和 migration 命名的专门规范。代码通用命名仍见 `docs/coding-standards.md`，表清单由 `docs/database/table-catalog.md` 从 migrations 生成。

## 1. 适用范围

- 适用于 `backend/migrations/*.sql` 和 repository 中的 SQL。
- 业务语义以用户故事、ADR 和领域代码为准；本规范只约束数据库表达方式。
- 开发阶段可以直接修改未发布 migration；正式发布前的破坏性变更必须按 ADR-0016 补兼容迁移、回填、灰度和回滚证据。

## 2. 命名规则

| 对象 | 规则 | 示例 |
|---|---|---|
| migration 文件 | `YYYYMMDDHHMM_<scope>.sql` | `202606030001_wave3_core_tables.sql` |
| 表 | `snake_case` 复数或集合名 | `receiving_orders`, `inventory_batches` |
| 横向能力表 | 能力前缀 + 业务名 | `auth_users`, `audit_event` |
| 系统表 | `system_` 前缀 | `system_dictionary_items` |
| 关联表 | `<left>_<right>` 复数集合 | `auth_user_roles`, `outbound_wave_orders` |
| 事件 / 流水表 | 业务名 + `_events` / `_movements` / `_changes` | `inventory_movements` |
| 字段 | `snake_case` | `owner_id`, `created_at` |
| 普通索引 | `<table>_<purpose>_idx` | `receiving_orders_owner_status_idx` |
| 唯一表达式索引 | `<table>_<purpose>_uidx` 或 `<table>_<purpose>_idx` | `system_dictionary_items_scope_uidx` |

说明：

- 新 migration 使用当前仓库已经采用的索引命名风格，不再使用旧示例里的 `idx_<table>_<columns>`。
- 简单 `CHECK`、`UNIQUE`、`FOREIGN KEY` 可以内联；只有需要跨团队排查、迁移脚本引用或错误码映射时才显式命名约束。

## 3. 表结构规则

| 表类型 | 必备字段 | 说明 |
|---|---|---|
| 业务聚合表 | `id UUID PRIMARY KEY`, `owner_id UUID NOT NULL`, `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` | 多货主隔离默认要求 |
| 可变聚合表 | 业务聚合表字段 + `updated_at`, `version` | 用于乐观锁或事务内并发保护 |
| 子表 / 明细表 | `owner_id`, 父表 ID, `created_at` | 查询仍必须显式带 `owner_id` |
| 全局字典 / 权限元数据 | 可无 `owner_id` | 必须是全局语义，不承载货主业务数据 |
| 审计表 | 按 ADR-0025 | append-only、分区、哈希链优先于通用规则 |

## 4. 约束与索引

- owner-scoped 唯一键必须包含 `owner_id`。
- 数量字段必须有非负或正数 `CHECK`，例如 `qty >= 0`、`planned_qty > 0`。
- 时间区间字段必须校验结束时间晚于开始时间。
- 高频查询索引应以 `owner_id` 开头，除非该表是全局表或审计特殊索引。
- 子表引用父表时，应用层必须保证 `child.owner_id = parent.owner_id`；生产级跨货主强约束需要优先考虑复合唯一键 + 复合外键。

## 5. 状态与字典

- 状态字段可以使用 `TEXT`，但状态机必须在 domain / service 层校验。
- 全局可控、货主可覆盖或需要运营配置的枚举，优先进入 `system_dictionary_*`。
- `document_type` 等会影响流程方向、批号策略、workflow template 的字段，必须能追溯到系统字典配置。

## 6. migration 数据写入

- migration 默认只建结构、索引、约束、函数、触发器和权限。
- 允许写入确定性的系统级种子数据，例如系统字典预置项。
- 禁止在 migration 写入测试账号、真实货主业务数据、生产默认配置或密钥。

## 7. 设计检查清单

- [ ] 表名和字段名符合本文命名规则。
- [ ] 业务表有 `owner_id`，查询路径有 owner 过滤。
- [ ] 聚合根有 `id`、`created_at`，可变聚合有 `updated_at` 和 `version`。
- [ ] owner-scoped 唯一键包含 `owner_id`。
- [ ] 数量、时间区间、状态变更审批源有数据库或领域层约束。
- [ ] 写操作能在同一事务内安排业务表、审计和幂等。
- [ ] 新字段能追溯到用户故事、ADR 或业务澄清。
