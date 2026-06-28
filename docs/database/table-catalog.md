# 数据库表目录

> 本文件由 `python3 scripts/governance/generate_table_catalog.py` 从 `backend/migrations/*.sql` 生成；不要手工修改表清单。业务解释以用户故事、ADR 和迁移脚本为准。本文件随表数量自然超过普通文档行数阈值，行数门禁按生成物处理。

## 统计

- 迁移文件：7
- 数据表：52
- 索引：49

## 表清单

| 表 | 模块 | 迁移 | 货主字段 | 字段数 | 索引数 |
|---|---|---|---|---:|---:|
| `audit_event` | Wave 1 审计 | `backend/migrations/202606020001_audit_event.sql` | 有 | 16 | 4 |
| `audit_event_2026_06` | Wave 1 审计 | `backend/migrations/202606020001_audit_event.sql` | 继承 audit_event | 0 | 0 |
| `audit_chain_seal` | Wave 1 审计 | `backend/migrations/202606020001_audit_event.sql` | 无 | 4 | 0 |
| `idempotency_request` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 12 | 1 |
| `receiving_orders` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 12 | 1 |
| `receiving_order_lines` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 11 | 1 |
| `receiving_order_receipts` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 10 | 1 |
| `receiving_inspections` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 12 | 1 |
| `receiving_inspection_signatures` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 9 | 1 |
| `receiving_putaways` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 11 | 1 |
| `inventory_batches` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 15 | 3 |
| `inventory_movements` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 9 | 1 |
| `inventory_status_changes` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 10 | 1 |
| `cold_chain_devices` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 10 | 0 |
| `temperature_readings` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 11 | 1 |
| `temperature_excursion_events` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 12 | 1 |
| `billing_accounts` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 8 | 0 |
| `billing_contracts` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 10 | 1 |
| `billing_rules` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 10 | 2 |
| `outbound_orders` | Wave 4 出库 / 追溯 | `backend/migrations/202606040001_wave4_outbound_tables.sql` | 有 | 12 | 1 |
| `outbound_order_lines` | Wave 4 出库 / 追溯 | `backend/migrations/202606040001_wave4_outbound_tables.sql` | 有 | 12 | 1 |
| `outbound_waves` | Wave 4 出库 / 追溯 | `backend/migrations/202606040001_wave4_outbound_tables.sql` | 有 | 7 | 0 |
| `outbound_wave_orders` | Wave 4 出库 / 追溯 | `backend/migrations/202606040001_wave4_outbound_tables.sql` | 有 | 5 | 0 |
| `outbound_shipments` | Wave 4 出库 / 追溯 | `backend/migrations/202606040001_wave4_outbound_tables.sql` | 有 | 8 | 1 |
| `traceability_outbound_reports` | Wave 4 出库 / 追溯 | `backend/migrations/202606040001_wave4_outbound_tables.sql` | 有 | 9 | 1 |
| `traceability_outbound_report_events` | Wave 4 出库 / 追溯 | `backend/migrations/202606040001_wave4_outbound_tables.sql` | 有 | 12 | 2 |
| `packing_stations` | Wave 5 增值 / TMS / 计费 | `backend/migrations/202606050001_wave5_value_added_tables.sql` | 有 | 11 | 0 |
| `packing_jobs` | Wave 5 增值 / TMS / 计费 | `backend/migrations/202606050001_wave5_value_added_tables.sql` | 有 | 17 | 1 |
| `retail_replenishment_suggestions` | Wave 5 增值 / TMS / 计费 | `backend/migrations/202606050001_wave5_value_added_tables.sql` | 有 | 13 | 0 |
| `crossdock_plans` | Wave 5 增值 / TMS / 计费 | `backend/migrations/202606050001_wave5_value_added_tables.sql` | 有 | 9 | 1 |
| `billing_charge_calculations` | Wave 5 增值 / TMS / 计费 | `backend/migrations/202606050001_wave5_value_added_tables.sql` | 有 | 11 | 0 |
| `billing_statements` | Wave 5 增值 / TMS / 计费 | `backend/migrations/202606050001_wave5_value_added_tables.sql` | 有 | 10 | 0 |
| `billing_statement_charges` | Wave 5 增值 / TMS / 计费 | `backend/migrations/202606050001_wave5_value_added_tables.sql` | 有 | 5 | 0 |
| `tms_dispatches` | Wave 5 增值 / TMS / 计费 | `backend/migrations/202606050001_wave5_value_added_tables.sql` | 有 | 16 | 1 |
| `transit_temperature_readings` | Wave 5 增值 / TMS / 计费 | `backend/migrations/202606050001_wave5_value_added_tables.sql` | 有 | 11 | 0 |
| `container_recoveries` | Wave 5 增值 / TMS / 计费 | `backend/migrations/202606050001_wave5_value_added_tables.sql` | 有 | 12 | 0 |
| `auth_owners` | H1 鉴权 / 货主访问 | `backend/migrations/202606060001_h1_auth_tables.sql` | 无 | 4 | 1 |
| `auth_users` | H1 鉴权 / 货主访问 | `backend/migrations/202606060001_h1_auth_tables.sql` | 无 | 10 | 1 |
| `auth_user_owner_bindings` | H1 鉴权 / 货主访问 | `backend/migrations/202606060001_h1_auth_tables.sql` | 有 | 5 | 1 |
| `auth_roles` | H1 鉴权 / 货主访问 | `backend/migrations/202606060001_h1_auth_tables.sql` | 有 | 5 | 1 |
| `auth_permissions` | H1 鉴权 / 货主访问 | `backend/migrations/202606060001_h1_auth_tables.sql` | 无 | 4 | 1 |
| `auth_user_roles` | H1 鉴权 / 货主访问 | `backend/migrations/202606060001_h1_auth_tables.sql` | 有 | 4 | 1 |
| `auth_role_permissions` | H1 鉴权 / 货主访问 | `backend/migrations/202606060001_h1_auth_tables.sql` | 无 | 3 | 1 |
| `system_dictionary_categories` | M1 系统字典 | `backend/migrations/202606280001_system_dictionary.sql` | 无 | 11 | 0 |
| `system_dictionary_items` | M1 系统字典 | `backend/migrations/202606280001_system_dictionary.sql` | 有 | 14 | 2 |
| `products` | M1 主数据 / 数据库规范对齐 | `backend/migrations/202606280002_database_design_standard_alignment.sql` | 有 | 14 | 1 |
| `suppliers` | M1 主数据 / 数据库规范对齐 | `backend/migrations/202606280002_database_design_standard_alignment.sql` | 有 | 11 | 1 |
| `customers` | M1 主数据 / 数据库规范对齐 | `backend/migrations/202606280002_database_design_standard_alignment.sql` | 有 | 11 | 2 |
| `customer_addresses` | M1 主数据 / 数据库规范对齐 | `backend/migrations/202606280002_database_design_standard_alignment.sql` | 有 | 13 | 1 |
| `warehouses` | M1 主数据 / 数据库规范对齐 | `backend/migrations/202606280002_database_design_standard_alignment.sql` | 有 | 10 | 2 |
| `warehouse_zones` | M1 主数据 / 数据库规范对齐 | `backend/migrations/202606280002_database_design_standard_alignment.sql` | 有 | 11 | 2 |
| `warehouse_locations` | M1 主数据 / 数据库规范对齐 | `backend/migrations/202606280002_database_design_standard_alignment.sql` | 有 | 17 | 1 |

## 字段明细

### `audit_event`

- 模块：Wave 1 审计
- 迁移：`backend/migrations/202606020001_audit_event.sql`
- 货主字段：有
- 索引：`audit_event_actor_idx`, `audit_event_diff_changed_keys_idx`, `audit_event_module_idx`, `audit_event_owner_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id BIGSERIAL` |
| `occurred_at` | `occurred_at TIMESTAMPTZ NOT NULL` |
| `actor_id` | `actor_id UUID NOT NULL` |
| `actor_name` | `actor_name TEXT NOT NULL` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `jti` | `jti TEXT NOT NULL` |
| `action` | `action TEXT NOT NULL` |
| `module` | `module TEXT NOT NULL` |
| `resource_type` | `resource_type TEXT` |
| `resource_id` | `resource_id TEXT` |
| `diff` | `diff JSONB` |
| `request_id` | `request_id UUID` |
| `ip` | `ip INET` |
| `user_agent` | `user_agent TEXT` |
| `prev_hash` | `prev_hash TEXT` |
| `self_hash` | `self_hash TEXT NOT NULL` |

### `audit_event_2026_06`

- 模块：Wave 1 审计
- 迁移：`backend/migrations/202606020001_audit_event.sql`
- 货主字段：继承 audit_event
- 索引：无

分区表，字段继承 `audit_event`。

### `audit_chain_seal`

- 模块：Wave 1 审计
- 迁移：`backend/migrations/202606020001_audit_event.sql`
- 货主字段：无
- 索引：无

| 字段 | SQL 定义 |
|---|---|
| `seal_date` | `seal_date DATE PRIMARY KEY` |
| `last_id` | `last_id BIGINT NOT NULL` |
| `last_self_hash` | `last_self_hash TEXT NOT NULL` |
| `sealed_at` | `sealed_at TIMESTAMPTZ NOT NULL` |

### `idempotency_request`

- 模块：Wave 3 入库 / 库存 / 冷链 / 计费
- 迁移：`backend/migrations/202606030001_wave3_core_tables.sql`
- 货主字段：有
- 索引：`idempotency_request_expires_at_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `idempotency_key` | `idempotency_key TEXT NOT NULL` |
| `request_hash` | `request_hash TEXT NOT NULL` |
| `method` | `method TEXT NOT NULL` |
| `path` | `path TEXT NOT NULL` |
| `status_code` | `status_code INT NOT NULL` |
| `response_body` | `response_body JSONB NOT NULL` |
| `resource_type` | `resource_type TEXT NOT NULL` |
| `resource_id` | `resource_id TEXT NOT NULL` |
| `expires_at` | `expires_at TIMESTAMPTZ NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `receiving_orders`

- 模块：Wave 3 入库 / 库存 / 冷链 / 计费
- 迁移：`backend/migrations/202606030001_wave3_core_tables.sql`
- 货主字段：有
- 索引：`receiving_orders_owner_status_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `receipt_no` | `receipt_no TEXT NOT NULL` |
| `document_type` | `document_type TEXT NOT NULL` |
| `supplier_id` | `supplier_id UUID` |
| `warehouse_id` | `warehouse_id UUID NOT NULL` |
| `external_ref` | `external_ref TEXT` |
| `status` | `status TEXT NOT NULL` |
| `expected_arrival_at` | `expected_arrival_at TIMESTAMPTZ` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `receiving_order_lines`

- 模块：Wave 3 入库 / 库存 / 冷链 / 计费
- 迁移：`backend/migrations/202606030001_wave3_core_tables.sql`
- 货主字段：有
- 索引：`receiving_order_lines_owner_product_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `receiving_order_id` | `receiving_order_id UUID NOT NULL REFERENCES receiving_orders(id) ON DELETE CASCADE` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `line_no` | `line_no INT NOT NULL CHECK (line_no > 0)` |
| `product_id` | `product_id UUID` |
| `product_code` | `product_code TEXT NOT NULL` |
| `expected_qty` | `expected_qty BIGINT NOT NULL CHECK (expected_qty > 0)` |
| `batch_no` | `batch_no TEXT` |
| `production_date` | `production_date DATE` |
| `expiry_date` | `expiry_date DATE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `receiving_order_receipts`

- 模块：Wave 3 入库 / 库存 / 冷链 / 计费
- 迁移：`backend/migrations/202606030001_wave3_core_tables.sql`
- 货主字段：有
- 索引：`receiving_order_receipts_owner_occurred_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `receiving_order_id` | `receiving_order_id UUID NOT NULL REFERENCES receiving_orders(id) ON DELETE CASCADE` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `actual_qty` | `actual_qty BIGINT NOT NULL CHECK (actual_qty >= 0)` |
| `shortage_qty` | `shortage_qty BIGINT NOT NULL CHECK (shortage_qty >= 0)` |
| `rejected_qty` | `rejected_qty BIGINT NOT NULL CHECK (rejected_qty >= 0)` |
| `arrival_temperature_celsius` | `arrival_temperature_celsius DOUBLE PRECISION` |
| `exception_note` | `exception_note TEXT` |
| `occurred_at` | `occurred_at TIMESTAMPTZ NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `receiving_inspections`

- 模块：Wave 3 入库 / 库存 / 冷链 / 计费
- 迁移：`backend/migrations/202606030001_wave3_core_tables.sql`
- 货主字段：有
- 索引：`receiving_inspections_owner_batch_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `receiving_order_id` | `receiving_order_id UUID NOT NULL REFERENCES receiving_orders(id) ON DELETE CASCADE` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `batch_no` | `batch_no TEXT NOT NULL` |
| `accepted_qty` | `accepted_qty BIGINT NOT NULL CHECK (accepted_qty >= 0)` |
| `rejected_qty` | `rejected_qty BIGINT NOT NULL CHECK (rejected_qty >= 0)` |
| `production_date` | `production_date DATE NOT NULL` |
| `expiry_date` | `expiry_date DATE NOT NULL` |
| `quality_status` | `quality_status TEXT NOT NULL` |
| `trace_codes` | `trace_codes TEXT[] NOT NULL DEFAULT '{}'` |
| `occurred_at` | `occurred_at TIMESTAMPTZ NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `receiving_inspection_signatures`

- 模块：Wave 3 入库 / 库存 / 冷链 / 计费
- 迁移：`backend/migrations/202606030001_wave3_core_tables.sql`
- 货主字段：有
- 索引：`receiving_inspection_signatures_owner_signed_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `receiving_order_id` | `receiving_order_id UUID NOT NULL REFERENCES receiving_orders(id) ON DELETE CASCADE` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `dual_required` | `dual_required BOOLEAN NOT NULL` |
| `first_signer_id` | `first_signer_id UUID NOT NULL` |
| `second_signer_id` | `second_signer_id UUID` |
| `strategy_rule_id` | `strategy_rule_id UUID` |
| `signed_at` | `signed_at TIMESTAMPTZ NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `receiving_putaways`

- 模块：Wave 3 入库 / 库存 / 冷链 / 计费
- 迁移：`backend/migrations/202606030001_wave3_core_tables.sql`
- 货主字段：有
- 索引：`receiving_putaways_owner_batch_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `receiving_order_id` | `receiving_order_id UUID NOT NULL REFERENCES receiving_orders(id) ON DELETE CASCADE` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `batch_no` | `batch_no TEXT NOT NULL` |
| `product_code` | `product_code TEXT NOT NULL` |
| `qty` | `qty BIGINT NOT NULL CHECK (qty > 0)` |
| `location_id` | `location_id UUID NOT NULL` |
| `location_code` | `location_code TEXT NOT NULL` |
| `quality_status` | `quality_status TEXT NOT NULL` |
| `occurred_at` | `occurred_at TIMESTAMPTZ NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `inventory_batches`

- 模块：Wave 3 入库 / 库存 / 冷链 / 计费
- 迁移：`backend/migrations/202606030001_wave3_core_tables.sql`
- 货主字段：有
- 索引：`inventory_batches_owner_expiry_idx`, `inventory_batches_owner_location_status_idx`, `inventory_batches_owner_product_batch_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `product_code` | `product_code TEXT NOT NULL` |
| `batch_no` | `batch_no TEXT NOT NULL` |
| `production_date` | `production_date DATE NOT NULL` |
| `expiry_date` | `expiry_date DATE NOT NULL` |
| `qty_on_hand` | `qty_on_hand BIGINT NOT NULL CHECK (qty_on_hand >= 0)` |
| `qty_locked` | `qty_locked BIGINT NOT NULL DEFAULT 0 CHECK (qty_locked >= 0)` |
| `quality_status` | `quality_status TEXT NOT NULL` |
| `location_id` | `location_id UUID NOT NULL` |
| `location_code` | `location_code TEXT NOT NULL` |
| `recall_flag` | `recall_flag BOOLEAN NOT NULL DEFAULT FALSE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `inventory_movements`

- 模块：Wave 3 入库 / 库存 / 冷链 / 计费
- 迁移：`backend/migrations/202606030001_wave3_core_tables.sql`
- 货主字段：有
- 索引：`inventory_movements_owner_batch_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `batch_id` | `batch_id UUID NOT NULL REFERENCES inventory_batches(id)` |
| `movement_type` | `movement_type TEXT NOT NULL` |
| `qty_delta` | `qty_delta BIGINT NOT NULL` |
| `source_document_type` | `source_document_type TEXT NOT NULL` |
| `source_document_id` | `source_document_id UUID NOT NULL` |
| `occurred_at` | `occurred_at TIMESTAMPTZ NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `inventory_status_changes`

- 模块：Wave 3 入库 / 库存 / 冷链 / 计费
- 迁移：`backend/migrations/202606030001_wave3_core_tables.sql`
- 货主字段：有
- 索引：`inventory_status_changes_owner_batch_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `batch_id` | `batch_id UUID NOT NULL REFERENCES inventory_batches(id)` |
| `from_status` | `from_status TEXT NOT NULL` |
| `to_status` | `to_status TEXT NOT NULL` |
| `reason` | `reason TEXT NOT NULL` |
| `approval_source` | `approval_source TEXT NOT NULL` |
| `approval_id` | `approval_id TEXT NOT NULL` |
| `occurred_at` | `occurred_at TIMESTAMPTZ NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `cold_chain_devices`

- 模块：Wave 3 入库 / 库存 / 冷链 / 计费
- 迁移：`backend/migrations/202606030001_wave3_core_tables.sql`
- 货主字段：有
- 索引：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `device_code` | `device_code TEXT NOT NULL` |
| `device_type` | `device_type TEXT NOT NULL` |
| `installed_at_location_code` | `installed_at_location_code TEXT` |
| `calibration_due_at` | `calibration_due_at TIMESTAMPTZ` |
| `status` | `status TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `temperature_readings`

- 模块：Wave 3 入库 / 库存 / 冷链 / 计费
- 迁移：`backend/migrations/202606030001_wave3_core_tables.sql`
- 货主字段：有
- 索引：`temperature_readings_owner_device_captured_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `device_code` | `device_code TEXT NOT NULL` |
| `temperature_celsius` | `temperature_celsius DOUBLE PRECISION NOT NULL` |
| `humidity_percent` | `humidity_percent DOUBLE PRECISION` |
| `captured_at` | `captured_at TIMESTAMPTZ NOT NULL` |
| `external_report_url` | `external_report_url TEXT` |
| `out_of_range` | `out_of_range BOOLEAN NOT NULL` |
| `source_system` | `source_system TEXT` |
| `external_reading_id` | `external_reading_id TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `temperature_excursion_events`

- 模块：Wave 3 入库 / 库存 / 冷链 / 计费
- 迁移：`backend/migrations/202606030001_wave3_core_tables.sql`
- 货主字段：有
- 索引：`temperature_excursion_events_owner_status_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `external_event_id` | `external_event_id TEXT NOT NULL` |
| `device_code` | `device_code TEXT NOT NULL` |
| `location_code` | `location_code TEXT` |
| `started_at` | `started_at TIMESTAMPTZ NOT NULL` |
| `ended_at` | `ended_at TIMESTAMPTZ` |
| `min_temperature_celsius` | `min_temperature_celsius DOUBLE PRECISION` |
| `max_temperature_celsius` | `max_temperature_celsius DOUBLE PRECISION` |
| `affected_batch_ids` | `affected_batch_ids UUID[] NOT NULL DEFAULT '{}'` |
| `status` | `status TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `billing_accounts`

- 模块：Wave 3 入库 / 库存 / 冷链 / 计费
- 迁移：`backend/migrations/202606030001_wave3_core_tables.sql`
- 货主字段：有
- 索引：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `account_code` | `account_code TEXT NOT NULL` |
| `account_name` | `account_name TEXT NOT NULL` |
| `status` | `status TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `billing_contracts`

- 模块：Wave 3 入库 / 库存 / 冷链 / 计费
- 迁移：`backend/migrations/202606030001_wave3_core_tables.sql`
- 货主字段：有
- 索引：`billing_contracts_owner_account_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `account_id` | `account_id UUID NOT NULL REFERENCES billing_accounts(id)` |
| `contract_no` | `contract_no TEXT NOT NULL` |
| `valid_from` | `valid_from DATE NOT NULL` |
| `valid_to` | `valid_to DATE NOT NULL` |
| `status` | `status TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `billing_rules`

- 模块：Wave 3 入库 / 库存 / 冷链 / 计费
- 迁移：`backend/migrations/202606030001_wave3_core_tables.sql`
- 货主字段：有
- 索引：`billing_rules_owner_contract_idx`, `billing_rules_owner_effective_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `contract_id` | `contract_id UUID NOT NULL REFERENCES billing_contracts(id)` |
| `charge_item` | `charge_item TEXT NOT NULL` |
| `unit` | `unit TEXT NOT NULL` |
| `unit_price_cents` | `unit_price_cents BIGINT NOT NULL CHECK (unit_price_cents >= 0)` |
| `billing_cycle` | `billing_cycle TEXT NOT NULL` |
| `effective_from` | `effective_from DATE NOT NULL` |
| `effective_to` | `effective_to DATE NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `outbound_orders`

- 模块：Wave 4 出库 / 追溯
- 迁移：`backend/migrations/202606040001_wave4_outbound_tables.sql`
- 货主字段：有
- 索引：`outbound_orders_owner_status_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `wms_order_no` | `wms_order_no TEXT NOT NULL` |
| `erp_order_no` | `erp_order_no TEXT` |
| `customer_id` | `customer_id UUID NOT NULL` |
| `warehouse_id` | `warehouse_id UUID NOT NULL` |
| `required_ship_at` | `required_ship_at TIMESTAMPTZ` |
| `status` | `status TEXT NOT NULL` |
| `short_pick` | `short_pick BOOLEAN NOT NULL DEFAULT FALSE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `outbound_order_lines`

- 模块：Wave 4 出库 / 追溯
- 迁移：`backend/migrations/202606040001_wave4_outbound_tables.sql`
- 货主字段：有
- 索引：`outbound_order_lines_owner_product_batch_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `outbound_order_id` | `outbound_order_id UUID NOT NULL REFERENCES outbound_orders(id) ON DELETE CASCADE` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `line_no` | `line_no INT NOT NULL CHECK (line_no > 0)` |
| `product_code` | `product_code TEXT NOT NULL` |
| `batch_no` | `batch_no TEXT NOT NULL` |
| `planned_qty` | `planned_qty BIGINT NOT NULL CHECK (planned_qty > 0)` |
| `picked_qty` | `picked_qty BIGINT NOT NULL DEFAULT 0 CHECK (picked_qty >= 0)` |
| `reviewed_qty` | `reviewed_qty BIGINT NOT NULL DEFAULT 0 CHECK (reviewed_qty >= 0)` |
| `shipped_qty` | `shipped_qty BIGINT NOT NULL DEFAULT 0 CHECK (shipped_qty >= 0)` |
| `short_pick_qty` | `short_pick_qty BIGINT NOT NULL DEFAULT 0 CHECK (short_pick_qty >= 0)` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `outbound_waves`

- 模块：Wave 4 出库 / 追溯
- 迁移：`backend/migrations/202606040001_wave4_outbound_tables.sql`
- 货主字段：有
- 索引：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `wave_no` | `wave_no TEXT NOT NULL` |
| `status` | `status TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `outbound_wave_orders`

- 模块：Wave 4 出库 / 追溯
- 迁移：`backend/migrations/202606040001_wave4_outbound_tables.sql`
- 货主字段：有
- 索引：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `wave_id` | `wave_id UUID NOT NULL REFERENCES outbound_waves(id) ON DELETE CASCADE` |
| `outbound_order_id` | `outbound_order_id UUID NOT NULL REFERENCES outbound_orders(id) ON DELETE CASCADE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `outbound_shipments`

- 模块：Wave 4 出库 / 追溯
- 迁移：`backend/migrations/202606040001_wave4_outbound_tables.sql`
- 货主字段：有
- 索引：`outbound_shipments_owner_shipped_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `outbound_order_id` | `outbound_order_id UUID NOT NULL REFERENCES outbound_orders(id) ON DELETE CASCADE` |
| `carrier_type` | `carrier_type TEXT NOT NULL` |
| `handover_to` | `handover_to TEXT NOT NULL` |
| `package_count` | `package_count INT NOT NULL CHECK (package_count > 0)` |
| `shipped_at` | `shipped_at TIMESTAMPTZ NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `traceability_outbound_reports`

- 模块：Wave 4 出库 / 追溯
- 迁移：`backend/migrations/202606040001_wave4_outbound_tables.sql`
- 货主字段：有
- 索引：`traceability_outbound_reports_owner_status_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `platform` | `platform TEXT NOT NULL` |
| `status` | `status TEXT NOT NULL` |
| `queued_count` | `queued_count INT NOT NULL CHECK (queued_count > 0)` |
| `generated_at` | `generated_at TIMESTAMPTZ NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `traceability_outbound_report_events`

- 模块：Wave 4 出库 / 追溯
- 迁移：`backend/migrations/202606040001_wave4_outbound_tables.sql`
- 货主字段：有
- 索引：`traceability_outbound_report_events_owner_status_idx`, `traceability_outbound_report_events_trace_code_idx`

| 字段 | SQL 定义 |
|---|---|
| `event_id` | `event_id UUID NOT NULL` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `report_id` | `report_id UUID NOT NULL REFERENCES traceability_outbound_reports(id) ON DELETE CASCADE` |
| `trace_code` | `trace_code TEXT NOT NULL` |
| `status_change_type` | `status_change_type TEXT NOT NULL` |
| `occurred_at` | `occurred_at TIMESTAMPTZ NOT NULL` |
| `report_status` | `report_status TEXT NOT NULL DEFAULT 'queued'` |
| `retry_count` | `retry_count INT NOT NULL DEFAULT 0 CHECK (retry_count >= 0)` |
| `last_error_code` | `last_error_code TEXT` |
| `platform_receipt_id` | `platform_receipt_id TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `packing_stations`

- 模块：Wave 5 增值 / TMS / 计费
- 迁移：`backend/migrations/202606050001_wave5_value_added_tables.sql`
- 货主字段：有
- 索引：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `station_code` | `station_code TEXT NOT NULL` |
| `station_name` | `station_name TEXT NOT NULL` |
| `printer_code` | `printer_code TEXT` |
| `scale_code` | `scale_code TEXT` |
| `temperature_zone` | `temperature_zone TEXT NOT NULL` |
| `status` | `status TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `packing_jobs`

- 模块：Wave 5 增值 / TMS / 计费
- 迁移：`backend/migrations/202606050001_wave5_value_added_tables.sql`
- 货主字段：有
- 索引：`packing_jobs_owner_status_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `outbound_order_id` | `outbound_order_id UUID NOT NULL REFERENCES outbound_orders(id) ON DELETE CASCADE` |
| `station_id` | `station_id UUID REFERENCES packing_stations(id)` |
| `job_no` | `job_no TEXT NOT NULL` |
| `pack_mode` | `pack_mode TEXT NOT NULL` |
| `recommended_box_type` | `recommended_box_type TEXT NOT NULL` |
| `actual_box_type` | `actual_box_type TEXT NOT NULL` |
| `adjustment_reason` | `adjustment_reason TEXT` |
| `outbound_lpn` | `outbound_lpn TEXT NOT NULL` |
| `trace_codes` | `trace_codes TEXT[] NOT NULL` |
| `status` | `status TEXT NOT NULL` |
| `weight_grams` | `weight_grams BIGINT` |
| `waybill_no` | `waybill_no TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `retail_replenishment_suggestions`

- 模块：Wave 5 增值 / TMS / 计费
- 迁移：`backend/migrations/202606050001_wave5_value_added_tables.sql`
- 货主字段：有
- 索引：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `store_id` | `store_id UUID NOT NULL` |
| `product_code` | `product_code TEXT NOT NULL` |
| `period_key` | `period_key TEXT NOT NULL` |
| `min_qty` | `min_qty BIGINT NOT NULL CHECK (min_qty >= 0)` |
| `max_qty` | `max_qty BIGINT NOT NULL CHECK (max_qty >= min_qty)` |
| `current_qty` | `current_qty BIGINT NOT NULL CHECK (current_qty >= 0)` |
| `in_transit_qty` | `in_transit_qty BIGINT NOT NULL CHECK (in_transit_qty >= 0)` |
| `daily_sales_avg` | `daily_sales_avg BIGINT NOT NULL CHECK (daily_sales_avg >= 0)` |
| `suggested_qty` | `suggested_qty BIGINT NOT NULL CHECK (suggested_qty >= 0)` |
| `status` | `status TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `crossdock_plans`

- 模块：Wave 5 增值 / TMS / 计费
- 迁移：`backend/migrations/202606050001_wave5_value_added_tables.sql`
- 货主字段：有
- 索引：`crossdock_plans_owner_store_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `asn_id` | `asn_id UUID NOT NULL` |
| `outbound_order_id` | `outbound_order_id UUID NOT NULL REFERENCES outbound_orders(id) ON DELETE CASCADE` |
| `store_id` | `store_id UUID NOT NULL` |
| `product_code` | `product_code TEXT NOT NULL` |
| `qty` | `qty BIGINT NOT NULL CHECK (qty > 0)` |
| `status` | `status TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `billing_charge_calculations`

- 模块：Wave 5 增值 / TMS / 计费
- 迁移：`backend/migrations/202606050001_wave5_value_added_tables.sql`
- 货主字段：有
- 索引：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `contract_id` | `contract_id UUID NOT NULL REFERENCES billing_contracts(id) ON DELETE CASCADE` |
| `period_start` | `period_start TEXT NOT NULL` |
| `period_end` | `period_end TEXT NOT NULL` |
| `charge_item` | `charge_item TEXT NOT NULL` |
| `quantity` | `quantity BIGINT NOT NULL CHECK (quantity >= 0)` |
| `amount_cents` | `amount_cents BIGINT NOT NULL CHECK (amount_cents >= 0)` |
| `source_refs` | `source_refs TEXT[] NOT NULL` |
| `status` | `status TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `billing_statements`

- 模块：Wave 5 增值 / TMS / 计费
- 迁移：`backend/migrations/202606050001_wave5_value_added_tables.sql`
- 货主字段：有
- 索引：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `contract_id` | `contract_id UUID NOT NULL REFERENCES billing_contracts(id) ON DELETE CASCADE` |
| `period_start` | `period_start TEXT NOT NULL` |
| `period_end` | `period_end TEXT NOT NULL` |
| `status` | `status TEXT NOT NULL` |
| `total_amount_cents` | `total_amount_cents BIGINT NOT NULL CHECK (total_amount_cents >= 0)` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `billing_statement_charges`

- 模块：Wave 5 增值 / TMS / 计费
- 迁移：`backend/migrations/202606050001_wave5_value_added_tables.sql`
- 货主字段：有
- 索引：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `statement_id` | `statement_id UUID NOT NULL REFERENCES billing_statements(id) ON DELETE CASCADE` |
| `charge_id` | `charge_id UUID NOT NULL REFERENCES billing_charge_calculations(id) ON DELETE RESTRICT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `tms_dispatches`

- 模块：Wave 5 增值 / TMS / 计费
- 迁移：`backend/migrations/202606050001_wave5_value_added_tables.sql`
- 货主字段：有
- 索引：`tms_dispatches_owner_order_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `dispatch_no` | `dispatch_no TEXT NOT NULL` |
| `outbound_order_id` | `outbound_order_id UUID NOT NULL REFERENCES outbound_orders(id) ON DELETE CASCADE` |
| `delivery_provider_type` | `delivery_provider_type TEXT NOT NULL` |
| `vehicle_no` | `vehicle_no TEXT` |
| `plate_no` | `plate_no TEXT` |
| `driver_user_id` | `driver_user_id UUID` |
| `carrier_code` | `carrier_code TEXT` |
| `waybill_no` | `waybill_no TEXT` |
| `status` | `status TEXT NOT NULL` |
| `dispatch_version` | `dispatch_version INT NOT NULL CHECK (dispatch_version > 0)` |
| `scheduled_load_at` | `scheduled_load_at TIMESTAMPTZ` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `transit_temperature_readings`

- 模块：Wave 5 增值 / TMS / 计费
- 迁移：`backend/migrations/202606050001_wave5_value_added_tables.sql`
- 货主字段：有
- 索引：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `dispatch_id` | `dispatch_id UUID NOT NULL REFERENCES tms_dispatches(id) ON DELETE CASCADE` |
| `device_code` | `device_code TEXT NOT NULL` |
| `plate_no` | `plate_no TEXT NOT NULL` |
| `measured_at` | `measured_at TIMESTAMPTZ NOT NULL` |
| `temperature_celsius` | `temperature_celsius DOUBLE PRECISION NOT NULL` |
| `humidity_percent` | `humidity_percent DOUBLE PRECISION` |
| `is_exceeded` | `is_exceeded BOOLEAN NOT NULL DEFAULT FALSE` |
| `external_trace_url` | `external_trace_url TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `container_recoveries`

- 模块：Wave 5 增值 / TMS / 计费
- 迁移：`backend/migrations/202606050001_wave5_value_added_tables.sql`
- 货主字段：有
- 索引：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `container_lpn` | `container_lpn TEXT NOT NULL` |
| `dispatch_id` | `dispatch_id UUID REFERENCES tms_dispatches(id) ON DELETE SET NULL` |
| `customer_id` | `customer_id UUID NOT NULL` |
| `delivery_provider_type` | `delivery_provider_type TEXT NOT NULL` |
| `status` | `status TEXT NOT NULL` |
| `shipped_at` | `shipped_at TIMESTAMPTZ NOT NULL` |
| `recovered_at` | `recovered_at TIMESTAMPTZ` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `auth_owners`

- 模块：H1 鉴权 / 货主访问
- 迁移：`backend/migrations/202606060001_h1_auth_tables.sql`
- 货主字段：无
- 索引：`UNIQUE auth_owners_owner_code_lower_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_code` | `owner_code TEXT NOT NULL` |
| `owner_name` | `owner_name TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `auth_users`

- 模块：H1 鉴权 / 货主访问
- 迁移：`backend/migrations/202606060001_h1_auth_tables.sql`
- 货主字段：无
- 索引：`UNIQUE auth_users_username_lower_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `username` | `username TEXT NOT NULL` |
| `display_name` | `display_name TEXT NOT NULL` |
| `password_hash` | `password_hash TEXT NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'active'` |
| `failed_login_count` | `failed_login_count INT NOT NULL DEFAULT 0 CHECK (failed_login_count >= 0)` |
| `locked_until` | `locked_until TIMESTAMPTZ` |
| `permissions_changed_at` | `permissions_changed_at TIMESTAMPTZ` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `auth_user_owner_bindings`

- 模块：H1 鉴权 / 货主访问
- 迁移：`backend/migrations/202606060001_h1_auth_tables.sql`
- 货主字段：有
- 索引：`auth_user_owner_bindings_owner_idx`

| 字段 | SQL 定义 |
|---|---|
| `user_id` | `user_id UUID NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `is_active` | `is_active BOOLEAN NOT NULL DEFAULT TRUE` |
| `is_primary` | `is_primary BOOLEAN NOT NULL DEFAULT FALSE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `auth_roles`

- 模块：H1 鉴权 / 货主访问
- 迁移：`backend/migrations/202606060001_h1_auth_tables.sql`
- 货主字段：有
- 索引：`UNIQUE auth_roles_owner_role_code_lower_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `role_code` | `role_code TEXT NOT NULL` |
| `role_name` | `role_name TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `auth_permissions`

- 模块：H1 鉴权 / 货主访问
- 迁移：`backend/migrations/202606060001_h1_auth_tables.sql`
- 货主字段：无
- 索引：`UNIQUE auth_permissions_code_lower_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `permission_code` | `permission_code TEXT NOT NULL` |
| `permission_name` | `permission_name TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `auth_user_roles`

- 模块：H1 鉴权 / 货主访问
- 迁移：`backend/migrations/202606060001_h1_auth_tables.sql`
- 货主字段：有
- 索引：`auth_user_roles_role_idx`

| 字段 | SQL 定义 |
|---|---|
| `user_id` | `user_id UUID NOT NULL` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `role_id` | `role_id UUID NOT NULL REFERENCES auth_roles(id) ON DELETE CASCADE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `auth_role_permissions`

- 模块：H1 鉴权 / 货主访问
- 迁移：`backend/migrations/202606060001_h1_auth_tables.sql`
- 货主字段：无
- 索引：`auth_role_permissions_permission_idx`

| 字段 | SQL 定义 |
|---|---|
| `role_id` | `role_id UUID NOT NULL REFERENCES auth_roles(id) ON DELETE CASCADE` |
| `permission_id` | `permission_id UUID NOT NULL REFERENCES auth_permissions(id) ON DELETE CASCADE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `system_dictionary_categories`

- 模块：M1 系统字典
- 迁移：`backend/migrations/202606280001_system_dictionary.sql`
- 货主字段：无
- 索引：无

| 字段 | SQL 定义 |
|---|---|
| `dict_code` | `dict_code TEXT PRIMARY KEY` |
| `dict_name` | `dict_name TEXT NOT NULL` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `control_level` | `control_level TEXT NOT NULL` |
| `param_schema` | `param_schema JSONB NOT NULL DEFAULT '{}'::jsonb` |
| `scope_mode` | `scope_mode TEXT NOT NULL` |
| `override_policy` | `override_policy JSONB NOT NULL DEFAULT '{}'::jsonb` |
| `sort_order` | `sort_order INT NOT NULL DEFAULT 0` |
| `remark` | `remark TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `system_dictionary_items`

- 模块：M1 系统字典
- 迁移：`backend/migrations/202606280001_system_dictionary.sql`
- 货主字段：有
- 索引：`UNIQUE system_dictionary_items_scope_uidx`, `system_dictionary_items_owner_lookup_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `dict_code` | `dict_code TEXT NOT NULL REFERENCES system_dictionary_categories(dict_code)` |
| `item_code` | `item_code TEXT NOT NULL` |
| `item_name` | `item_name TEXT NOT NULL` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `owner_id` | `owner_id UUID` |
| `params` | `params JSONB NOT NULL DEFAULT '{}'::jsonb` |
| `effective_from` | `effective_from TIMESTAMPTZ` |
| `effective_to` | `effective_to TIMESTAMPTZ` |
| `source` | `source TEXT NOT NULL` |
| `disabled_reason` | `disabled_reason TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `products`

- 模块：M1 主数据 / 数据库规范对齐
- 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 货主字段：有
- 索引：`products_owner_status_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `product_code` | `product_code TEXT NOT NULL` |
| `product_name` | `product_name TEXT NOT NULL` |
| `specification` | `specification TEXT NOT NULL` |
| `dosage_form` | `dosage_form TEXT` |
| `storage_condition` | `storage_condition TEXT NOT NULL` |
| `special_drug_category` | `special_drug_category TEXT NOT NULL DEFAULT 'normal'` |
| `approval_no` | `approval_no TEXT` |
| `manufacturer` | `manufacturer TEXT` |
| `status` | `status TEXT NOT NULL DEFAULT 'active'` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `suppliers`

- 模块：M1 主数据 / 数据库规范对齐
- 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 货主字段：有
- 索引：`suppliers_owner_status_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `supplier_code` | `supplier_code TEXT NOT NULL` |
| `supplier_name` | `supplier_name TEXT NOT NULL` |
| `uscc` | `uscc TEXT NOT NULL` |
| `contact_name` | `contact_name TEXT` |
| `contact_phone` | `contact_phone TEXT` |
| `status` | `status TEXT NOT NULL DEFAULT 'active'` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `customers`

- 模块：M1 主数据 / 数据库规范对齐
- 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 货主字段：有
- 索引：`UNIQUE customers_owner_id_uidx`, `customers_owner_status_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `customer_code` | `customer_code TEXT NOT NULL` |
| `customer_name` | `customer_name TEXT NOT NULL` |
| `customer_type` | `customer_type TEXT NOT NULL DEFAULT 'customer'` |
| `contact_name` | `contact_name TEXT` |
| `contact_phone` | `contact_phone TEXT` |
| `status` | `status TEXT NOT NULL DEFAULT 'active'` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `customer_addresses`

- 模块：M1 主数据 / 数据库规范对齐
- 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 货主字段：有
- 索引：`customer_addresses_owner_customer_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `customer_id` | `customer_id UUID NOT NULL` |
| `province` | `province TEXT NOT NULL` |
| `city` | `city TEXT NOT NULL` |
| `district` | `district TEXT NOT NULL` |
| `detail_address` | `detail_address TEXT NOT NULL` |
| `contact_name` | `contact_name TEXT NOT NULL` |
| `contact_phone` | `contact_phone TEXT NOT NULL` |
| `is_default` | `is_default BOOLEAN NOT NULL DEFAULT FALSE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `warehouses`

- 模块：M1 主数据 / 数据库规范对齐
- 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 货主字段：有
- 索引：`UNIQUE warehouses_owner_id_uidx`, `warehouses_owner_status_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `warehouse_code` | `warehouse_code TEXT NOT NULL` |
| `warehouse_name` | `warehouse_name TEXT NOT NULL` |
| `warehouse_type` | `warehouse_type TEXT NOT NULL` |
| `address` | `address TEXT` |
| `status` | `status TEXT NOT NULL DEFAULT 'active'` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `warehouse_zones`

- 模块：M1 主数据 / 数据库规范对齐
- 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 货主字段：有
- 索引：`UNIQUE warehouse_zones_owner_id_uidx`, `warehouse_zones_owner_warehouse_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `warehouse_id` | `warehouse_id UUID NOT NULL` |
| `zone_code` | `zone_code TEXT NOT NULL` |
| `zone_name` | `zone_name TEXT NOT NULL` |
| `temperature_zone` | `temperature_zone TEXT NOT NULL` |
| `quality_color` | `quality_color TEXT NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'active'` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `warehouse_locations`

- 模块：M1 主数据 / 数据库规范对齐
- 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 货主字段：有
- 索引：`warehouse_locations_owner_zone_status_idx`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `warehouse_id` | `warehouse_id UUID NOT NULL` |
| `zone_id` | `zone_id UUID NOT NULL` |
| `location_code` | `location_code TEXT NOT NULL` |
| `row_no` | `row_no INT NOT NULL CHECK (row_no > 0)` |
| `column_no` | `column_no INT NOT NULL CHECK (column_no > 0)` |
| `layer_no` | `layer_no INT NOT NULL CHECK (layer_no > 0)` |
| `max_volume_cm3` | `max_volume_cm3 BIGINT NOT NULL CHECK (max_volume_cm3 >= 0)` |
| `used_volume_cm3` | `used_volume_cm3 BIGINT NOT NULL DEFAULT 0 CHECK (used_volume_cm3 >= 0)` |
| `max_sku_count` | `max_sku_count INT NOT NULL DEFAULT 1 CHECK (max_sku_count > 0)` |
| `location_type` | `location_type TEXT NOT NULL` |
| `bound_owner_id` | `bound_owner_id UUID` |
| `status` | `status TEXT NOT NULL DEFAULT 'available'` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |
