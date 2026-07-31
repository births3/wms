# 数据库表目录

> 本文件由 `python3 scripts/governance/generate_table_catalog.py` 从 `backend/migrations/*.sql` 生成；不要手工修改表清单。业务解释以用户故事、ADR 和迁移脚本为准。按日期动态创建的分区保留在 migration 函数中，不计入静态表清单。本文件随表数量自然超过普通文档行数阈值，行数门禁按生成物处理。

## 统计

- 迁移文件：61
- 数据表：193
- 索引：168

## 表清单

| 表 | 模块 | 创建迁移 | 货主字段 | 字段数 | 索引数 | ALTER 迁移数 | 引用表数 |
|---|---|---|---|---:|---:|---:|---:|
| `audit_event` | Wave 1 审计 | `backend/migrations/202606020001_audit_event.sql` | 有 | 16 | 4 | 0 | 0 |
| `audit_event_2026_06` | Wave 1 审计 | `backend/migrations/202606020001_audit_event.sql` | 继承 audit_event | 0 | 0 | 0 | 0 |
| `audit_chain_seal` | Wave 1 审计 | `backend/migrations/202606020001_audit_event.sql` | 无 | 4 | 0 | 0 | 0 |
| `idempotency_request` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 12 | 1 | 0 | 0 |
| `receiving_orders` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 12 | 1 | 0 | 0 |
| `receiving_order_lines` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 11 | 1 | 1 | 1 |
| `receiving_order_receipts` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 10 | 1 | 2 | 1 |
| `receiving_inspections` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 12 | 1 | 2 | 1 |
| `receiving_inspection_signatures` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 9 | 1 | 2 | 3 |
| `receiving_putaways` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 11 | 1 | 2 | 1 |
| `inventory_batches` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 15 | 3 | 1 | 0 |
| `inventory_movements` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 16 | 4 | 2 | 1 |
| `inventory_status_changes` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 10 | 1 | 1 | 1 |
| `cold_chain_devices` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 10 | 0 | 0 | 0 |
| `temperature_readings` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 11 | 1 | 0 | 0 |
| `temperature_excursion_events` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 12 | 1 | 0 | 0 |
| `billing_accounts` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 8 | 0 | 0 | 0 |
| `billing_contracts` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 10 | 1 | 1 | 1 |
| `billing_rules` | Wave 3 入库 / 库存 / 冷链 / 计费 | `backend/migrations/202606030001_wave3_core_tables.sql` | 有 | 10 | 2 | 1 | 1 |
| `outbound_orders` | Wave 4 出库 / 追溯 | `backend/migrations/202606040001_wave4_outbound_tables.sql` | 有 | 14 | 1 | 2 | 0 |
| `outbound_order_lines` | Wave 4 出库 / 追溯 | `backend/migrations/202606040001_wave4_outbound_tables.sql` | 有 | 12 | 1 | 1 | 1 |
| `outbound_waves` | Wave 4 出库 / 追溯 | `backend/migrations/202606040001_wave4_outbound_tables.sql` | 有 | 7 | 0 | 0 | 0 |
| `outbound_wave_orders` | Wave 4 出库 / 追溯 | `backend/migrations/202606040001_wave4_outbound_tables.sql` | 有 | 5 | 0 | 1 | 2 |
| `outbound_shipments` | Wave 4 出库 / 追溯 | `backend/migrations/202606040001_wave4_outbound_tables.sql` | 有 | 18 | 1 | 2 | 3 |
| `traceability_outbound_reports` | Wave 4 出库 / 追溯 | `backend/migrations/202606040001_wave4_outbound_tables.sql` | 有 | 9 | 1 | 0 | 0 |
| `traceability_outbound_report_events` | Wave 4 出库 / 追溯 | `backend/migrations/202606040001_wave4_outbound_tables.sql` | 有 | 12 | 2 | 1 | 1 |
| `packing_stations` | Wave 5 增值 / TMS / 计费 | `backend/migrations/202606050001_wave5_value_added_tables.sql` | 有 | 11 | 0 | 0 | 0 |
| `packing_jobs` | Wave 5 增值 / TMS / 计费 | `backend/migrations/202606050001_wave5_value_added_tables.sql` | 有 | 17 | 1 | 1 | 2 |
| `retail_replenishment_suggestions` | Wave 5 增值 / TMS / 计费 | `backend/migrations/202606050001_wave5_value_added_tables.sql` | 有 | 13 | 0 | 0 | 0 |
| `crossdock_plans` | Wave 5 增值 / TMS / 计费 | `backend/migrations/202606050001_wave5_value_added_tables.sql` | 有 | 9 | 1 | 1 | 1 |
| `billing_charge_calculations` | Wave 5 增值 / TMS / 计费 | `backend/migrations/202606050001_wave5_value_added_tables.sql` | 有 | 11 | 0 | 1 | 1 |
| `billing_statements` | Wave 5 增值 / TMS / 计费 | `backend/migrations/202606050001_wave5_value_added_tables.sql` | 有 | 10 | 0 | 1 | 1 |
| `billing_statement_charges` | Wave 5 增值 / TMS / 计费 | `backend/migrations/202606050001_wave5_value_added_tables.sql` | 有 | 5 | 0 | 1 | 2 |
| `tms_dispatches` | Wave 5 增值 / TMS / 计费 | `backend/migrations/202606050001_wave5_value_added_tables.sql` | 有 | 16 | 1 | 1 | 1 |
| `transit_temperature_readings` | Wave 5 增值 / TMS / 计费 | `backend/migrations/202606050001_wave5_value_added_tables.sql` | 有 | 11 | 0 | 1 | 1 |
| `container_recoveries` | Wave 5 增值 / TMS / 计费 | `backend/migrations/202606050001_wave5_value_added_tables.sql` | 有 | 12 | 0 | 0 | 1 |
| `auth_owners` | H1 鉴权 / 货主访问 | `backend/migrations/202606060001_h1_auth_tables.sql` | 无 | 4 | 1 | 0 | 0 |
| `auth_users` | H1 鉴权 / 货主访问 | `backend/migrations/202606060001_h1_auth_tables.sql` | 无 | 10 | 1 | 1 | 0 |
| `auth_user_owner_bindings` | H1 鉴权 / 货主访问 | `backend/migrations/202606060001_h1_auth_tables.sql` | 有 | 5 | 1 | 0 | 2 |
| `auth_roles` | H1 鉴权 / 货主访问 | `backend/migrations/202606060001_h1_auth_tables.sql` | 有 | 5 | 1 | 1 | 2 |
| `auth_permissions` | H1 鉴权 / 货主访问 | `backend/migrations/202606060001_h1_auth_tables.sql` | 无 | 4 | 1 | 0 | 0 |
| `auth_user_roles` | H1 鉴权 / 货主访问 | `backend/migrations/202606060001_h1_auth_tables.sql` | 有 | 4 | 1 | 0 | 2 |
| `auth_role_permissions` | H1 鉴权 / 货主访问 | `backend/migrations/202606060001_h1_auth_tables.sql` | 无 | 3 | 1 | 0 | 2 |
| `system_dictionary_categories` | M1 系统字典 | `backend/migrations/202606280001_system_dictionary.sql` | 无 | 11 | 0 | 0 | 0 |
| `system_dictionary_items` | M1 系统字典 | `backend/migrations/202606280001_system_dictionary.sql` | 有 | 14 | 2 | 1 | 1 |
| `products` | M1 主数据 / 数据库规范对齐 | `backend/migrations/202606280002_database_design_standard_alignment.sql` | 有 | 14 | 1 | 3 | 0 |
| `suppliers` | M1 主数据 / 数据库规范对齐 | `backend/migrations/202606280002_database_design_standard_alignment.sql` | 有 | 11 | 1 | 2 | 0 |
| `customers` | M1 主数据 / 数据库规范对齐 | `backend/migrations/202606280002_database_design_standard_alignment.sql` | 有 | 11 | 2 | 3 | 0 |
| `customer_addresses` | M1 主数据 / 数据库规范对齐 | `backend/migrations/202606280002_database_design_standard_alignment.sql` | 有 | 13 | 1 | 0 | 1 |
| `warehouses` | M1 主数据 / 数据库规范对齐 | `backend/migrations/202606280002_database_design_standard_alignment.sql` | 有 | 10 | 2 | 0 | 0 |
| `warehouse_zones` | M1 主数据 / 数据库规范对齐 | `backend/migrations/202606280002_database_design_standard_alignment.sql` | 有 | 11 | 2 | 1 | 1 |
| `warehouse_locations` | M1 主数据 / 数据库规范对齐 | `backend/migrations/202606280002_database_design_standard_alignment.sql` | 有 | 17 | 1 | 1 | 3 |
| `document_number_rules` | mcg_document_numbering | `backend/migrations/202607020001_mcg_document_numbering.sql` | 有 | 15 | 2 | 0 | 1 |
| `document_number_counters` | mcg_document_numbering | `backend/migrations/202607020001_mcg_document_numbering.sql` | 无 | 7 | 0 | 0 | 1 |
| `document_number_allocations` | mcg_document_numbering | `backend/migrations/202607020001_mcg_document_numbering.sql` | 有 | 12 | 1 | 0 | 2 |
| `print_field_libraries` | h9_print_template | `backend/migrations/202607050002_h9_print_template.sql` | 无 | 8 | 0 | 0 | 0 |
| `print_field_library_versions` | h9_print_template | `backend/migrations/202607050002_h9_print_template.sql` | 无 | 11 | 1 | 0 | 1 |
| `print_field_definitions` | h9_print_template | `backend/migrations/202607050002_h9_print_template.sql` | 无 | 19 | 1 | 0 | 1 |
| `admin_menu_draft_nodes` | h1_admin_menu | `backend/migrations/202607050003_h1_admin_menu.sql` | 无 | 14 | 4 | 0 | 1 |
| `admin_menu_draft_button_permissions` | h1_admin_menu | `backend/migrations/202607050003_h1_admin_menu.sql` | 无 | 9 | 0 | 0 | 1 |
| `admin_menu_versions` | h1_admin_menu | `backend/migrations/202607050003_h1_admin_menu.sql` | 无 | 5 | 0 | 0 | 0 |
| `admin_menu_version_nodes` | h1_admin_menu | `backend/migrations/202607050003_h1_admin_menu.sql` | 无 | 15 | 1 | 0 | 1 |
| `admin_menu_version_button_permissions` | h1_admin_menu | `backend/migrations/202607050003_h1_admin_menu.sql` | 无 | 8 | 0 | 0 | 1 |
| `print_templates` | h9_print_template_runtime | `backend/migrations/202607070001_h9_print_template_runtime.sql` | 有 | 14 | 1 | 0 | 0 |
| `print_template_versions` | h9_print_template_runtime | `backend/migrations/202607070001_h9_print_template_runtime.sql` | 无 | 19 | 2 | 0 | 2 |
| `print_records` | h9_print_template_runtime | `backend/migrations/202607070001_h9_print_template_runtime.sql` | 有 | 12 | 1 | 0 | 1 |
| `audit_archive_partition_state` | h2_lifecycle | `backend/migrations/202607080001_h2_lifecycle.sql` | 无 | 8 | 1 | 0 | 0 |
| `audit_archive_run` | h2_lifecycle | `backend/migrations/202607080001_h2_lifecycle.sql` | 有 | 10 | 0 | 0 | 0 |
| `event_bus_subscription` | h2_lifecycle | `backend/migrations/202607080001_h2_lifecycle.sql` | 有 | 7 | 1 | 0 | 0 |
| `event_bus_event` | h2_lifecycle | `backend/migrations/202607080001_h2_lifecycle.sql` | 有 | 9 | 1 | 0 | 0 |
| `event_bus_delivery` | h2_lifecycle | `backend/migrations/202607080001_h2_lifecycle.sql` | 有 | 10 | 1 | 0 | 2 |
| `event_bus_dead_letter` | h2_lifecycle | `backend/migrations/202607080001_h2_lifecycle.sql` | 有 | 6 | 0 | 0 | 2 |
| `business_retention_policy` | h2_lifecycle | `backend/migrations/202607080001_h2_lifecycle.sql` | 有 | 10 | 0 | 0 | 0 |
| `business_archive_job` | h2_lifecycle | `backend/migrations/202607080001_h2_lifecycle.sql` | 有 | 11 | 1 | 0 | 1 |
| `h4_notification_configs` | h4_wechat_notify | `backend/migrations/202607080002_h4_wechat_notify.sql` | 有 | 12 | 1 | 0 | 0 |
| `h4_notification_records` | h4_wechat_notify | `backend/migrations/202607080002_h4_wechat_notify.sql` | 有 | 15 | 1 | 0 | 1 |
| `h4_approval_records` | h4_wechat_notify | `backend/migrations/202607080002_h4_wechat_notify.sql` | 有 | 17 | 1 | 0 | 0 |
| `h5_express_carriers` | h5_express | `backend/migrations/202607090002_h5_express.sql` | 有 | 14 | 1 | 0 | 1 |
| `h5_express_routing_rules` | h5_express | `backend/migrations/202607090002_h5_express.sql` | 有 | 14 | 1 | 0 | 1 |
| `h5_express_waybills` | h5_express | `backend/migrations/202607090002_h5_express.sql` | 有 | 20 | 1 | 1 | 2 |
| `h5_express_tracking_events` | h5_express | `backend/migrations/202607090002_h5_express.sql` | 有 | 10 | 1 | 0 | 2 |
| `h4_wechat_settings` | h4_wechat_settings | `backend/migrations/202607090005_h4_wechat_settings.sql` | 有 | 17 | 1 | 0 | 0 |
| `auth_role_permission_exclusions` | h1_role_management | `backend/migrations/202607120002_h1_role_management.sql` | 无 | 3 | 0 | 0 | 2 |
| `auth_sessions` | h1_auth_sessions | `backend/migrations/202607120003_h1_auth_sessions.sql` | 有 | 10 | 1 | 0 | 1 |
| `auth_api_keys` | h1_api_key_lifecycle | `backend/migrations/202607120004_h1_api_key_lifecycle.sql` | 有 | 22 | 2 | 0 | 3 |
| `inventory_allocations` | m3_inventory_allocations | `backend/migrations/202607130003_m3_inventory_allocations.sql` | 有 | 10 | 1 | 0 | 2 |
| `warehouse_docks` | dock_master | `backend/migrations/202607130008_dock_master.sql` | 无 | 10 | 1 | 0 | 1 |
| `inventory_recall_actions` | m3_recall_actions | `backend/migrations/202607130010_m3_recall_actions.sql` | 有 | 12 | 2 | 0 | 1 |
| `dock_appointments` | dock_maintenance_permissions | `backend/migrations/202607130014_dock_maintenance_permissions.sql` | 有 | 20 | 3 | 0 | 3 |
| `alert_definitions` | h1_alert_definitions | `backend/migrations/202607130015_h1_alert_definitions.sql` | 有 | 15 | 1 | 1 | 1 |
| `alert_definition_triggers` | h1_alert_definitions | `backend/migrations/202607130015_h1_alert_definitions.sql` | 无 | 6 | 1 | 0 | 1 |
| `config_center_feature_flags` | m1_config_center_feature_flags | `backend/migrations/202607140001_m1_config_center_feature_flags.sql` | 有 | 8 | 1 | 0 | 0 |
| `tms_route_plans` | m10_tms_route_plans | `backend/migrations/202607140003_m10_tms_route_plans.sql` | 有 | 12 | 1 | 0 | 0 |
| `tms_route_stops` | m10_tms_route_plans | `backend/migrations/202607140003_m10_tms_route_plans.sql` | 有 | 7 | 0 | 0 | 1 |
| `tms_route_orders` | m10_tms_route_plans | `backend/migrations/202607140003_m10_tms_route_plans.sql` | 有 | 6 | 1 | 0 | 3 |
| `outbound_pick_tasks` | m4_outbound_pick_tasks | `backend/migrations/202607140004_m4_outbound_pick_tasks.sql` | 有 | 16 | 1 | 0 | 3 |
| `inventory_status_transitions` | m3_inventory_status_transitions | `backend/migrations/202607140005_m3_inventory_status_transitions.sql` | 有 | 8 | 2 | 0 | 0 |
| `inventory_maintenance_tasks` | m3_inventory_maintenance | `backend/migrations/202607140006_m3_inventory_maintenance.sql` | 有 | 8 | 2 | 0 | 1 |
| `inventory_maintenance_records` | m3_inventory_maintenance | `backend/migrations/202607140006_m3_inventory_maintenance.sql` | 有 | 22 | 2 | 0 | 2 |
| `inventory_counts` | m3_inventory_counts | `backend/migrations/202607140008_m3_inventory_counts.sql` | 有 | 15 | 1 | 0 | 0 |
| `inventory_count_lines` | m3_inventory_counts | `backend/migrations/202607140008_m3_inventory_counts.sql` | 有 | 14 | 1 | 0 | 2 |
| `drug_inspection_platforms` | mdi_drug_inspection_platforms | `backend/migrations/202607140009_mdi_drug_inspection_platforms.sql` | 有 | 16 | 1 | 0 | 1 |
| `task_types` | mte_task_types | `backend/migrations/202607140010_mte_task_types.sql` | 有 | 12 | 2 | 1 | 1 |
| `dual_person_policy_rules` | mvr_dual_person_policy | `backend/migrations/202607150001_mvr_dual_person_policy.sql` | 有 | 14 | 2 | 0 | 4 |
| `outbound_review_records` | mvr_downstream_enforcement | `backend/migrations/202607150002_mvr_downstream_enforcement.sql` | 有 | 10 | 1 | 0 | 4 |
| `task_groups` | mte_task_execution | `backend/migrations/202607150003_mte_task_execution.sql` | 有 | 11 | 0 | 0 | 2 |
| `task_group_memberships` | mte_task_execution | `backend/migrations/202607150003_mte_task_execution.sql` | 有 | 4 | 1 | 1 | 4 |
| `warehouse_tasks` | mte_task_execution | `backend/migrations/202607150003_mte_task_execution.sql` | 有 | 35 | 4 | 2 | 6 |
| `task_execution_events` | mte_task_execution | `backend/migrations/202607150003_mte_task_execution.sql` | 有 | 12 | 1 | 0 | 3 |
| `task_priority_rules` | mte_task_priority_rules | `backend/migrations/202607150006_mte_task_priority_rules.sql` | 有 | 9 | 0 | 0 | 1 |
| `stock_adjustment_orders` | msa_stock_loss | `backend/migrations/202607150008_msa_stock_loss.sql` | 有 | 27 | 2 | 1 | 6 |
| `stock_adjustment_execution_records` | msa_stock_loss | `backend/migrations/202607150008_msa_stock_loss.sql` | 有 | 13 | 1 | 0 | 5 |
| `stock_adjustment_erp_feedback_outbox` | msa_stock_loss | `backend/migrations/202607150008_msa_stock_loss.sql` | 有 | 11 | 1 | 0 | 2 |
| `quality_liaison_types` | mql_quality_liaison | `backend/migrations/202607150010_mql_quality_liaison.sql` | 有 | 12 | 0 | 0 | 2 |
| `quality_liaison_orders` | mql_quality_liaison | `backend/migrations/202607150010_mql_quality_liaison.sql` | 有 | 19 | 1 | 0 | 4 |
| `alert_instances` | hal_alert_runtime | `backend/migrations/202607150012_hal_alert_runtime.sql` | 有 | 27 | 2 | 0 | 3 |
| `alert_lifecycle_events` | hal_alert_runtime | `backend/migrations/202607150012_hal_alert_runtime.sql` | 有 | 11 | 1 | 0 | 2 |
| `alert_escalation_rules` | hal_alert_escalation | `backend/migrations/202607150013_hal_alert_escalation.sql` | 有 | 15 | 0 | 0 | 1 |
| `alert_escalation_levels` | hal_alert_escalation | `backend/migrations/202607150013_hal_alert_escalation.sql` | 有 | 8 | 0 | 0 | 2 |
| `alert_escalation_events` | hal_alert_escalation | `backend/migrations/202607150013_hal_alert_escalation.sql` | 有 | 10 | 1 | 0 | 2 |
| `auth_user_warehouse_scopes` | hal_alert_dashboard | `backend/migrations/202607150014_hal_alert_dashboard.sql` | 有 | 4 | 0 | 0 | 4 |
| `alert_report_exports` | hal_alert_dashboard | `backend/migrations/202607150014_hal_alert_dashboard.sql` | 有 | 18 | 1 | 0 | 2 |
| `alert_statistics_snapshots` | hal_alert_dashboard | `backend/migrations/202607150014_hal_alert_dashboard.sql` | 有 | 6 | 0 | 0 | 1 |
| `inventory_relocations` | m3_remaining_closeout | `backend/migrations/202607170004_m3_remaining_closeout.sql` | 有 | 18 | 1 | 0 | 0 |
| `inventory_status_erp_feedback_outbox` | m3_remaining_closeout | `backend/migrations/202607170004_m3_remaining_closeout.sql` | 有 | 12 | 1 | 0 | 0 |
| `inventory_alert_events` | m3_remaining_closeout | `backend/migrations/202607170004_m3_remaining_closeout.sql` | 有 | 16 | 2 | 0 | 0 |
| `inventory_abc_classifications` | m3_remaining_closeout | `backend/migrations/202607170004_m3_remaining_closeout.sql` | 有 | 12 | 1 | 0 | 0 |
| `putaway_strategy_profiles` | m2_inbound_closeout | `backend/migrations/202607170006_m2_inbound_closeout.sql` | 有 | 12 | 2 | 1 | 0 |
| `receiving_putaway_erp_feedback_outbox` | m2_putaway_lpn_erp | `backend/migrations/202607180003_m2_putaway_lpn_erp.sql` | 有 | 13 | 1 | 0 | 1 |
| `archive_revision_erp_feedback_outbox` | h8_erp_outbox_extensions | `backend/migrations/202607180005_h8_erp_outbox_extensions.sql` | 有 | 17 | 1 | 0 | 1 |
| `reconciliation_erp_feedback_outbox` | h8_erp_outbox_extensions | `backend/migrations/202607180005_h8_erp_outbox_extensions.sql` | 有 | 12 | 1 | 0 | 1 |
| `shipment_confirm_erp_feedback_outbox` | h8_erp_outbox_extensions | `backend/migrations/202607180005_h8_erp_outbox_extensions.sql` | 有 | 12 | 1 | 0 | 1 |
| `inventory_snapshot_erp_feedback_outbox` | h8_erp_outbox_extensions | `backend/migrations/202607180005_h8_erp_outbox_extensions.sql` | 有 | 11 | 1 | 0 | 1 |
| `h8_erp_connectors` | h8_erp_connectors | `backend/migrations/202607190002_h8_erp_connectors.sql` | 有 | 28 | 1 | 1 | 1 |
| `h8_erp_in_flight_messages` | h8_erp_connectors | `backend/migrations/202607190002_h8_erp_connectors.sql` | 有 | 12 | 1 | 0 | 2 |
| `h8_erp_message_registry` | h8_erp_messages | `backend/migrations/202607190005_h8_erp_messages.sql` | 有 | 6 | 0 | 0 | 1 |
| `h8_erp_messages` | h8_erp_messages | `backend/migrations/202607190005_h8_erp_messages.sql` | 有 | 25 | 4 | 2 | 3 |
| `h8_erp_message_stats_daily` | h8_erp_messages | `backend/migrations/202607190005_h8_erp_messages.sql` | 无 | 13 | 0 | 0 | 1 |
| `h8_erp_message_attempt_registry` | h8_erp_messages | `backend/migrations/202607190005_h8_erp_messages.sql` | 有 | 5 | 0 | 0 | 2 |
| `h8_erp_message_attempts` | h8_erp_messages | `backend/migrations/202607190005_h8_erp_messages.sql` | 有 | 11 | 1 | 1 | 3 |
| `h8_erp_message_retention_policy` | h8_erp_messages | `backend/migrations/202607190005_h8_erp_messages.sql` | 有 | 3 | 0 | 0 | 1 |
| `h8_erp_worker_heartbeats` | h8_worker_runtime_and_payload_retention | `backend/migrations/202607220002_h8_worker_runtime_and_payload_retention.sql` | 有 | 9 | 0 | 0 | 2 |
| `h8_erp_worker_claim_controls` | h8_worker_runtime_and_payload_retention | `backend/migrations/202607220002_h8_worker_runtime_and_payload_retention.sql` | 有 | 8 | 0 | 0 | 2 |
| `h8_erp_payload_retention_policies` | h8_worker_runtime_and_payload_retention | `backend/migrations/202607220002_h8_worker_runtime_and_payload_retention.sql` | 有 | 6 | 0 | 0 | 2 |
| `h8_erp_connector_versions` | h8_erp_connector_versions | `backend/migrations/202607230001_h8_erp_connector_versions.sql` | 有 | 5 | 0 | 0 | 1 |
| `parameter_mapping_dictionaries` | mpm_persistent_mapping | `backend/migrations/202607230002_mpm_persistent_mapping.sql` | 有 | 12 | 0 | 0 | 1 |
| `parameter_mapping_rules` | mpm_persistent_mapping | `backend/migrations/202607230002_mpm_persistent_mapping.sql` | 有 | 15 | 0 | 0 | 2 |
| `parameter_mapping_queue` | mpm_persistent_mapping | `backend/migrations/202607230002_mpm_persistent_mapping.sql` | 有 | 12 | 0 | 0 | 1 |
| `product_packaging_levels` | m1_complete_product_contract | `backend/migrations/202607230003_m1_complete_product_contract.sql` | 有 | 11 | 0 | 0 | 1 |
| `product_mapping_traces` | m1_complete_product_contract | `backend/migrations/202607230003_m1_complete_product_contract.sql` | 有 | 9 | 0 | 0 | 2 |
| `reconciliation_rules` | mrc_reconciliation | `backend/migrations/202607230004_mrc_reconciliation.sql` | 有 | 5 | 0 | 0 | 1 |
| `reconciliation_runs` | mrc_reconciliation | `backend/migrations/202607230004_mrc_reconciliation.sql` | 有 | 11 | 0 | 0 | 1 |
| `reconciliation_schedule_claims` | mrc_reconciliation | `backend/migrations/202607230004_mrc_reconciliation.sql` | 有 | 15 | 0 | 0 | 2 |
| `reconciliation_items` | mrc_reconciliation | `backend/migrations/202607230004_mrc_reconciliation.sql` | 有 | 15 | 0 | 0 | 2 |
| `reconciliation_item_adjustments` | mrc_reconciliation | `backend/migrations/202607230004_mrc_reconciliation.sql` | 有 | 6 | 0 | 0 | 4 |
| `reconciliation_item_locks` | mrc_reconciliation | `backend/migrations/202607230004_mrc_reconciliation.sql` | 有 | 6 | 0 | 0 | 3 |
| `h_file_upload_sessions` | h_file_attachments | `backend/migrations/202607250001_h_file_attachments.sql` | 有 | 17 | 2 | 0 | 2 |
| `attachments` | h_file_attachments | `backend/migrations/202607250001_h_file_attachments.sql` | 有 | 12 | 1 | 1 | 2 |
| `h_file_download_sessions` | h_file_attachments | `backend/migrations/202607250001_h_file_attachments.sql` | 有 | 7 | 1 | 0 | 3 |
| `drug_inspection_reports` | mdi_documents | `backend/migrations/202607250002_mdi_documents.sql` | 有 | 8 | 0 | 0 | 4 |
| `drug_inspection_report_versions` | mdi_documents | `backend/migrations/202607250002_mdi_documents.sql` | 有 | 25 | 2 | 1 | 6 |
| `drug_inspection_asn_links` | mdi_documents | `backend/migrations/202607250002_mdi_documents.sql` | 有 | 9 | 1 | 0 | 5 |
| `upstream_delivery_documents` | mdi_documents | `backend/migrations/202607250002_mdi_documents.sql` | 有 | 5 | 0 | 0 | 3 |
| `upstream_delivery_document_versions` | mdi_documents | `backend/migrations/202607250002_mdi_documents.sql` | 有 | 7 | 0 | 0 | 3 |
| `upstream_delivery_document_files` | mdi_documents | `backend/migrations/202607250002_mdi_documents.sql` | 无 | 3 | 0 | 0 | 2 |
| `upstream_delivery_document_asn_links` | mdi_documents | `backend/migrations/202607250002_mdi_documents.sql` | 有 | 6 | 0 | 0 | 4 |
| `upstream_delivery_asn_current` | mdi_documents | `backend/migrations/202607250002_mdi_documents.sql` | 有 | 4 | 0 | 0 | 3 |
| `drug_inspection_stamp_versions` | mdi_customer_copy | `backend/migrations/202607250003_mdi_customer_copy.sql` | 有 | 15 | 2 | 0 | 3 |
| `drug_inspection_customer_copy_jobs` | mdi_customer_copy | `backend/migrations/202607250003_mdi_customer_copy.sql` | 有 | 16 | 2 | 0 | 4 |
| `drug_inspection_processing_rule_versions` | mdi_customer_copy | `backend/migrations/202607250003_mdi_customer_copy.sql` | 有 | 8 | 0 | 0 | 2 |
| `drug_inspection_requirement_rules` | mdi_acceptance_validation | `backend/migrations/202607250005_mdi_acceptance_validation.sql` | 有 | 9 | 0 | 0 | 2 |
| `drug_inspection_acceptance_validations` | mdi_acceptance_validation | `backend/migrations/202607250005_mdi_acceptance_validation.sql` | 有 | 12 | 0 | 0 | 5 |
| `h9_route_bindings` | h9_delivery_note_aggregation | `backend/migrations/202607260003_h9_delivery_note_aggregation.sql` | 有 | 10 | 1 | 0 | 1 |
| `h9_cutoff_plans` | h9_delivery_note_aggregation | `backend/migrations/202607260003_h9_delivery_note_aggregation.sql` | 有 | 18 | 1 | 0 | 0 |
| `h9_outbound_route_snapshots` | h9_delivery_note_aggregation | `backend/migrations/202607260003_h9_delivery_note_aggregation.sql` | 有 | 8 | 1 | 0 | 2 |
| `h9_delivery_note_groups` | h9_delivery_note_aggregation | `backend/migrations/202607260003_h9_delivery_note_aggregation.sql` | 有 | 14 | 2 | 1 | 3 |
| `h9_delivery_note_group_orders` | h9_delivery_note_aggregation | `backend/migrations/202607260003_h9_delivery_note_aggregation.sql` | 有 | 8 | 0 | 0 | 2 |
| `purchase_return_orders` | m4_purchase_return_orders | `backend/migrations/202607260004_m4_purchase_return_orders.sql` | 有 | 20 | 2 | 0 | 0 |
| `h9_aggregation_field_catalog` | h9_aggregation_rules | `backend/migrations/202607260005_h9_aggregation_rules.sql` | 无 | 5 | 0 | 0 | 0 |
| `h9_aggregation_rule_versions` | h9_aggregation_rules | `backend/migrations/202607260005_h9_aggregation_rules.sql` | 有 | 15 | 1 | 0 | 0 |
| `h9_print_sites` | h9_print_devices | `backend/migrations/202607270001_h9_print_devices.sql` | 无 | 7 | 0 | 0 | 0 |
| `h9_print_site_owner_mappings` | h9_print_devices | `backend/migrations/202607270001_h9_print_devices.sql` | 有 | 9 | 1 | 0 | 1 |
| `h9_printers` | h9_print_devices | `backend/migrations/202607270001_h9_print_devices.sql` | 无 | 10 | 0 | 0 | 1 |
| `h9_printer_trays` | h9_print_devices | `backend/migrations/202607270001_h9_print_devices.sql` | 无 | 10 | 0 | 0 | 1 |
| `h9_device_leases` | h9_print_devices | `backend/migrations/202607270001_h9_print_devices.sql` | 无 | 14 | 2 | 0 | 1 |
| `h9_printer_test_prints` | h9_print_devices | `backend/migrations/202607270001_h9_print_devices.sql` | 无 | 9 | 1 | 0 | 2 |
| `h9_print_suite_versions` | h9_print_suites | `backend/migrations/202607270002_h9_print_suites.sql` | 有 | 21 | 1 | 0 | 1 |
| `h9_print_suite_items` | h9_print_suites | `backend/migrations/202607270002_h9_print_suites.sql` | 有 | 14 | 0 | 0 | 2 |
| `h9_print_suite_instances` | h9_print_suites | `backend/migrations/202607270002_h9_print_suites.sql` | 有 | 13 | 0 | 0 | 2 |
| `h9_print_suite_instance_items` | h9_print_suites | `backend/migrations/202607270002_h9_print_suites.sql` | 有 | 17 | 0 | 0 | 2 |
| `h9_document_file_bindings` | h_file_h9_category_pdfs | `backend/migrations/202607280001_h_file_h9_category_pdfs.sql` | 有 | 8 | 0 | 0 | 1 |
| `h9_category_pdf_preparations` | h_file_h9_category_pdfs | `backend/migrations/202607280001_h_file_h9_category_pdfs.sql` | 有 | 11 | 0 | 0 | 1 |
| `h9_category_pdf_outputs` | h_file_h9_category_pdfs | `backend/migrations/202607280001_h_file_h9_category_pdfs.sql` | 有 | 19 | 0 | 0 | 4 |

## 字段明细

### `audit_event`

- 模块：Wave 1 审计
- 迁移：`backend/migrations/202606020001_audit_event.sql`
- 货主字段：有
- 索引：`audit_event_actor_idx`, `audit_event_diff_changed_keys_idx`, `audit_event_module_idx`, `audit_event_owner_idx`
- ALTER 迁移：无
- 引用表：无

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
- ALTER 迁移：无
- 引用表：无

分区表，字段继承 `audit_event`。

### `audit_chain_seal`

- 模块：Wave 1 审计
- 迁移：`backend/migrations/202606020001_audit_event.sql`
- 货主字段：无
- 索引：无
- ALTER 迁移：无
- 引用表：无

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
- ALTER 迁移：无
- 引用表：无

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
- ALTER 迁移：无
- 引用表：无

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
- ALTER 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 引用表：`receiving_orders`

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
- ALTER 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`, `backend/migrations/202607130009_receiving_receipt_details.sql`
- 引用表：`receiving_orders`

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
- ALTER 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`, `backend/migrations/202607180002_m2_receive_inspect_gsp.sql`
- 引用表：`receiving_orders`

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
- ALTER 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`, `backend/migrations/202607150002_mvr_downstream_enforcement.sql`
- 引用表：`dual_person_policy_rules`, `h4_approval_records`, `receiving_orders`

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
- ALTER 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`, `backend/migrations/202607180003_m2_putaway_lpn_erp.sql`
- 引用表：`receiving_orders`

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
- ALTER 迁移：`backend/migrations/202607170001_m3_inventory_query_snapshot.sql`
- 引用表：无

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
- 索引：`inventory_movements_owner_batch_idx`, `inventory_movements_owner_from_location_occurred_idx`, `inventory_movements_owner_location_occurred_idx`, `inventory_movements_owner_to_location_occurred_idx`
- ALTER 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`, `backend/migrations/202607150008_msa_stock_loss.sql`
- 引用表：`inventory_batches`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `batch_id` | `batch_id UUID NOT NULL REFERENCES inventory_batches(id)` |
| `movement_type` | `movement_type TEXT NOT NULL` |
| `qty_delta` | `qty_delta BIGINT NOT NULL` |
| `source_document_type` | `source_document_type TEXT NOT NULL` |
| `source_document_id` | `source_document_id UUID NOT NULL` |
| `location_code` | `location_code TEXT` |
| `from_location_code` | `from_location_code TEXT` |
| `to_location_code` | `to_location_code TEXT` |
| `lpn_code` | `lpn_code TEXT` |
| `operator_user_id` | `operator_user_id UUID` |
| `operator_name` | `operator_name TEXT` |
| `volume_delta_cm3` | `volume_delta_cm3 BIGINT` |
| `occurred_at` | `occurred_at TIMESTAMPTZ NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `inventory_status_changes`

- 模块：Wave 3 入库 / 库存 / 冷链 / 计费
- 迁移：`backend/migrations/202606030001_wave3_core_tables.sql`
- 货主字段：有
- 索引：`inventory_status_changes_owner_batch_idx`
- ALTER 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 引用表：`inventory_batches`

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
- ALTER 迁移：无
- 引用表：无

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
- ALTER 迁移：无
- 引用表：无

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
- ALTER 迁移：无
- 引用表：无

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
- ALTER 迁移：无
- 引用表：无

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
- ALTER 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 引用表：`billing_accounts`

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
- ALTER 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 引用表：`billing_contracts`

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
- ALTER 迁移：`backend/migrations/202607130004_m4_outbound_document_type.sql`, `backend/migrations/202607260005_h9_aggregation_rules.sql`
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `wms_order_no` | `wms_order_no TEXT NOT NULL` |
| `erp_order_no` | `erp_order_no TEXT` |
| `customer_id` | `customer_id UUID NOT NULL` |
| `delivery_address_id` | `delivery_address_id UUID NOT NULL` |
| `delivery_address_snapshot` | `delivery_address_snapshot JSONB NOT NULL CHECK (jsonb_typeof(delivery_address_snapshot) = 'object')` |
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
- ALTER 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 引用表：`outbound_orders`

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
- ALTER 迁移：无
- 引用表：无

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
- ALTER 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 引用表：`outbound_orders`, `outbound_waves`

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
- ALTER 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`, `backend/migrations/202607250001_h_file_attachments.sql`
- 引用表：`attachments`, `auth_user_owner_bindings`, `outbound_orders`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `outbound_order_id` | `outbound_order_id UUID NOT NULL REFERENCES outbound_orders(id) ON DELETE CASCADE` |
| `delivery_provider_type` | `delivery_provider_type TEXT NOT NULL CHECK (delivery_provider_type IN ('own_fleet', 'third_party_express'))` |
| `vehicle_no` | `vehicle_no TEXT` |
| `plate_no` | `plate_no TEXT NOT NULL CHECK (length(btrim(plate_no)) BETWEEN 1 AND 32)` |
| `driver_user_id` | `driver_user_id UUID` |
| `driver_name` | `driver_name TEXT` |
| `courier_name` | `courier_name TEXT` |
| `courier_phone` | `courier_phone TEXT` |
| `signature_attachment_id` | `signature_attachment_id UUID` |
| `cold_chain` | `cold_chain BOOLEAN NOT NULL` |
| `loading_temperature_celsius` | `loading_temperature_celsius DOUBLE PRECISION` |
| `cold_chain_packages` | `cold_chain_packages JSONB NOT NULL DEFAULT '[]'::jsonb CHECK (jsonb_typeof(cold_chain_packages) = 'array')` |
| `package_count` | `package_count INT NOT NULL CHECK (package_count > 0)` |
| `handover_by` | `handover_by UUID NOT NULL` |
| `shipped_at` | `shipped_at TIMESTAMPTZ NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `traceability_outbound_reports`

- 模块：Wave 4 出库 / 追溯
- 迁移：`backend/migrations/202606040001_wave4_outbound_tables.sql`
- 货主字段：有
- 索引：`traceability_outbound_reports_owner_status_idx`
- ALTER 迁移：无
- 引用表：无

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
- ALTER 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 引用表：`traceability_outbound_reports`

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
- ALTER 迁移：无
- 引用表：无

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
- ALTER 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 引用表：`outbound_orders`, `packing_stations`

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
- ALTER 迁移：无
- 引用表：无

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
- ALTER 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 引用表：`outbound_orders`

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
- ALTER 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 引用表：`billing_contracts`

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
- ALTER 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 引用表：`billing_contracts`

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
- ALTER 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 引用表：`billing_charge_calculations`, `billing_statements`

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
- ALTER 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 引用表：`outbound_orders`

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
- ALTER 迁移：`backend/migrations/202606280002_database_design_standard_alignment.sql`
- 引用表：`tms_dispatches`

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
- ALTER 迁移：无
- 引用表：`tms_dispatches`

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
- ALTER 迁移：无
- 引用表：无

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
- ALTER 迁移：`backend/migrations/202607140002_m1_user_management.sql`
- 引用表：无

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
- ALTER 迁移：无
- 引用表：`auth_owners`, `auth_users`

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
- ALTER 迁移：`backend/migrations/202607120002_h1_role_management.sql`
- 引用表：`auth_owners`, `auth_roles`

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
- ALTER 迁移：无
- 引用表：无

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
- ALTER 迁移：无
- 引用表：`auth_roles`, `auth_user_owner_bindings`

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
- ALTER 迁移：无
- 引用表：`auth_permissions`, `auth_roles`

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
- ALTER 迁移：无
- 引用表：无

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
- ALTER 迁移：`backend/migrations/202607260001_h9_print_template_type_sort_order.sql`
- 引用表：`system_dictionary_categories`

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
- ALTER 迁移：`backend/migrations/202607020002_master_data_source.sql`, `backend/migrations/202607120005_m1_product_attrs.sql`, `backend/migrations/202607230003_m1_complete_product_contract.sql`
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `product_code` | `product_code TEXT NOT NULL` |
| `product_name` | `product_name TEXT NOT NULL` |
| `specification` | `specification TEXT NOT NULL` |
| `dosage_form` | `dosage_form TEXT` |
| `storage_condition` | `storage_condition TEXT` |
| `special_drug_category` | `special_drug_category TEXT` |
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
- ALTER 迁移：`backend/migrations/202607020002_master_data_source.sql`, `backend/migrations/202607170006_m2_inbound_closeout.sql`
- 引用表：无

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
- ALTER 迁移：`backend/migrations/202607020002_master_data_source.sql`, `backend/migrations/202607120001_customer_license_no.sql`, `backend/migrations/202607130012_m1_customer_profile_fields.sql`
- 引用表：无

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
- ALTER 迁移：无
- 引用表：`customers`

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
- ALTER 迁移：无
- 引用表：无

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
- ALTER 迁移：`backend/migrations/202607130007_m3_quality_color_status_mapping.sql`
- 引用表：`warehouses`

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
- ALTER 迁移：`backend/migrations/202607130006_m1_location_owner_binding.sql`
- 引用表：`auth_owners`, `warehouse_zones`, `warehouses`

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

### `document_number_rules`

- 模块：mcg_document_numbering
- 迁移：`backend/migrations/202607020001_mcg_document_numbering.sql`
- 货主字段：有
- 索引：`UNIQUE document_number_rules_scope_code_uidx`, `document_number_rules_effective_idx`
- ALTER 迁移：无
- 引用表：`auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `document_type` | `document_type TEXT NOT NULL` |
| `rule_code` | `rule_code TEXT NOT NULL` |
| `rule_name` | `rule_name TEXT NOT NULL` |
| `template` | `template TEXT NOT NULL` |
| `reset_policy` | `reset_policy TEXT NOT NULL` |
| `sequence_width` | `sequence_width INT NOT NULL CHECK (sequence_width > 0 AND sequence_width <= 18)` |
| `sequence_mode` | `sequence_mode TEXT NOT NULL DEFAULT 'no_gap'` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `effective_from` | `effective_from TIMESTAMPTZ` |
| `effective_to` | `effective_to TIMESTAMPTZ` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `document_number_counters`

- 模块：mcg_document_numbering
- 迁移：`backend/migrations/202607020001_mcg_document_numbering.sql`
- 货主字段：无
- 索引：无
- ALTER 迁移：无
- 引用表：`document_number_rules`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `rule_id` | `rule_id UUID NOT NULL REFERENCES document_number_rules(id) ON DELETE RESTRICT` |
| `counter_key` | `counter_key TEXT NOT NULL` |
| `current_value` | `current_value BIGINT NOT NULL DEFAULT 0 CHECK (current_value >= 0)` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `document_number_allocations`

- 模块：mcg_document_numbering
- 迁移：`backend/migrations/202607020001_mcg_document_numbering.sql`
- 货主字段：有
- 索引：`document_number_allocations_lookup_idx`
- ALTER 迁移：无
- 引用表：`auth_owners`, `document_number_rules`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `rule_id` | `rule_id UUID NOT NULL REFERENCES document_number_rules(id) ON DELETE RESTRICT` |
| `document_type` | `document_type TEXT NOT NULL` |
| `idempotency_key` | `idempotency_key TEXT NOT NULL` |
| `request_hash` | `request_hash TEXT NOT NULL` |
| `generated_no` | `generated_no TEXT NOT NULL` |
| `sequence_value` | `sequence_value BIGINT NOT NULL CHECK (sequence_value > 0)` |
| `counter_key` | `counter_key TEXT NOT NULL` |
| `source_module` | `source_module TEXT NOT NULL` |
| `source_document_id` | `source_document_id UUID` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `print_field_libraries`

- 模块：h9_print_template
- 迁移：`backend/migrations/202607050002_h9_print_template.sql`
- 货主字段：无
- 索引：无
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `library_code` | `library_code TEXT NOT NULL UNIQUE` |
| `library_name` | `library_name TEXT NOT NULL` |
| `business_module` | `business_module TEXT NOT NULL` |
| `source_schema` | `source_schema TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `print_field_library_versions`

- 模块：h9_print_template
- 迁移：`backend/migrations/202607050002_h9_print_template.sql`
- 货主字段：无
- 索引：`print_field_library_versions_lookup_idx`
- ALTER 迁移：无
- 引用表：`print_field_libraries`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `library_id` | `library_id UUID NOT NULL REFERENCES print_field_libraries(id) ON DELETE RESTRICT` |
| `version_no` | `version_no INT NOT NULL CHECK (version_no > 0)` |
| `status` | `status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'published'))` |
| `source_schema` | `source_schema TEXT NOT NULL` |
| `business_module` | `business_module TEXT NOT NULL` |
| `request_hash` | `request_hash TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `created_by` | `created_by UUID NOT NULL` |
| `published_at` | `published_at TIMESTAMPTZ` |
| `published_by` | `published_by UUID` |

### `print_field_definitions`

- 模块：h9_print_template
- 迁移：`backend/migrations/202607050002_h9_print_template.sql`
- 货主字段：无
- 索引：`print_field_definitions_version_order_idx`
- ALTER 迁移：无
- 引用表：`print_field_library_versions`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `library_version_id` | `library_version_id UUID NOT NULL REFERENCES print_field_library_versions(id) ON DELETE RESTRICT` |
| `field_path` | `field_path TEXT NOT NULL` |
| `field_type` | `field_type TEXT NOT NULL` |
| `source_schema` | `source_schema TEXT NOT NULL` |
| `display_name` | `display_name TEXT NOT NULL` |
| `group_code` | `group_code TEXT NOT NULL` |
| `group_name` | `group_name TEXT NOT NULL` |
| `description` | `description TEXT NOT NULL DEFAULT ''` |
| `example_value` | `example_value JSONB` |
| `printable` | `printable BOOLEAN NOT NULL DEFAULT TRUE` |
| `sensitive` | `sensitive BOOLEAN NOT NULL DEFAULT FALSE` |
| `masking_rule` | `masking_rule TEXT` |
| `formatting_rule` | `formatting_rule TEXT` |
| `supports_barcode` | `supports_barcode BOOLEAN NOT NULL DEFAULT FALSE` |
| `supports_qrcode` | `supports_qrcode BOOLEAN NOT NULL DEFAULT FALSE` |
| `is_table_detail` | `is_table_detail BOOLEAN NOT NULL DEFAULT FALSE` |
| `sort_order` | `sort_order INT NOT NULL DEFAULT 0` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `admin_menu_draft_nodes`

- 模块：h1_admin_menu
- 迁移：`backend/migrations/202607050003_h1_admin_menu.sql`
- 货主字段：无
- 索引：`UNIQUE admin_menu_draft_nodes_code_uidx`, `UNIQUE admin_menu_draft_nodes_path_uidx`, `UNIQUE admin_menu_draft_nodes_view_uidx`, `admin_menu_draft_nodes_parent_order_idx`
- ALTER 迁移：无
- 引用表：`admin_menu_draft_nodes`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `parent_id` | `parent_id UUID REFERENCES admin_menu_draft_nodes(id) ON DELETE CASCADE` |
| `level` | `level INT NOT NULL CHECK (level BETWEEN 1 AND 3)` |
| `code` | `code TEXT NOT NULL` |
| `path` | `path TEXT NOT NULL` |
| `title` | `title TEXT NOT NULL` |
| `view_id` | `view_id TEXT` |
| `icon_key` | `icon_key TEXT NOT NULL` |
| `permission_key` | `permission_key TEXT NOT NULL` |
| `sort_order` | `sort_order INT NOT NULL DEFAULT 0` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `admin_menu_draft_button_permissions`

- 模块：h1_admin_menu
- 迁移：`backend/migrations/202607050003_h1_admin_menu.sql`
- 货主字段：无
- 索引：无
- ALTER 迁移：无
- 引用表：`admin_menu_draft_nodes`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `menu_node_id` | `menu_node_id UUID NOT NULL REFERENCES admin_menu_draft_nodes(id) ON DELETE CASCADE` |
| `action_key` | `action_key TEXT NOT NULL` |
| `action_label` | `action_label TEXT NOT NULL` |
| `action_kind` | `action_kind TEXT NOT NULL CHECK (action_kind IN ('standard', 'private'))` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `sort_order` | `sort_order INT NOT NULL DEFAULT 0` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `admin_menu_versions`

- 模块：h1_admin_menu
- 迁移：`backend/migrations/202607050003_h1_admin_menu.sql`
- 货主字段：无
- 索引：无
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `version_no` | `version_no BIGINT NOT NULL UNIQUE` |
| `note` | `note TEXT` |
| `published_by` | `published_by UUID NOT NULL` |
| `published_at` | `published_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `admin_menu_version_nodes`

- 模块：h1_admin_menu
- 迁移：`backend/migrations/202607050003_h1_admin_menu.sql`
- 货主字段：无
- 索引：`admin_menu_version_nodes_version_order_idx`
- ALTER 迁移：无
- 引用表：`admin_menu_versions`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `version_id` | `version_id UUID NOT NULL REFERENCES admin_menu_versions(id) ON DELETE CASCADE` |
| `source_node_id` | `source_node_id UUID NOT NULL` |
| `parent_source_id` | `parent_source_id UUID` |
| `level` | `level INT NOT NULL CHECK (level BETWEEN 1 AND 3)` |
| `code` | `code TEXT NOT NULL` |
| `path` | `path TEXT NOT NULL` |
| `title` | `title TEXT NOT NULL` |
| `view_id` | `view_id TEXT` |
| `icon_key` | `icon_key TEXT NOT NULL` |
| `permission_key` | `permission_key TEXT NOT NULL` |
| `sort_order` | `sort_order INT NOT NULL DEFAULT 0` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `admin_menu_version_button_permissions`

- 模块：h1_admin_menu
- 迁移：`backend/migrations/202607050003_h1_admin_menu.sql`
- 货主字段：无
- 索引：无
- ALTER 迁移：无
- 引用表：`admin_menu_versions`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `version_id` | `version_id UUID NOT NULL REFERENCES admin_menu_versions(id) ON DELETE CASCADE` |
| `menu_source_node_id` | `menu_source_node_id UUID NOT NULL` |
| `action_key` | `action_key TEXT NOT NULL` |
| `action_label` | `action_label TEXT NOT NULL` |
| `action_kind` | `action_kind TEXT NOT NULL CHECK (action_kind IN ('standard', 'private'))` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `sort_order` | `sort_order INT NOT NULL DEFAULT 0` |

### `print_templates`

- 模块：h9_print_template_runtime
- 迁移：`backend/migrations/202607070001_h9_print_template_runtime.sql`
- 货主字段：有
- 索引：`print_templates_owner_type_lookup_idx`
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `template_code` | `template_code TEXT NOT NULL` |
| `template_name` | `template_name TEXT NOT NULL` |
| `template_type_code` | `template_type_code TEXT NOT NULL` |
| `scope` | `scope TEXT NOT NULL CHECK (scope IN ('global', 'owner'))` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `is_default` | `is_default BOOLEAN NOT NULL DEFAULT FALSE` |
| `remark` | `remark TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `created_by` | `created_by UUID NOT NULL` |
| `updated_by` | `updated_by UUID NOT NULL` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `print_template_versions`

- 模块：h9_print_template_runtime
- 迁移：`backend/migrations/202607070001_h9_print_template_runtime.sql`
- 货主字段：无
- 索引：`print_template_versions_published_lookup_idx`, `print_template_versions_template_lookup_idx`
- ALTER 迁移：无
- 引用表：`print_field_library_versions`, `print_templates`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `template_id` | `template_id UUID NOT NULL REFERENCES print_templates(id) ON DELETE RESTRICT` |
| `field_library_version_id` | `field_library_version_id UUID NOT NULL REFERENCES print_field_library_versions(id) ON DELETE RESTRICT` |
| `template_name` | `template_name TEXT NOT NULL` |
| `template_type_code` | `template_type_code TEXT NOT NULL` |
| `scope` | `scope TEXT NOT NULL CHECK (scope IN ('global', 'owner'))` |
| `is_default` | `is_default BOOLEAN NOT NULL DEFAULT FALSE` |
| `remark` | `remark TEXT` |
| `version_no` | `version_no INT NOT NULL CHECK (version_no > 0)` |
| `status` | `status TEXT NOT NULL CHECK (status IN ('draft', 'published'))` |
| `hiprint_json` | `hiprint_json JSONB NOT NULL` |
| `field_bindings` | `field_bindings JSONB NOT NULL DEFAULT '[]'::jsonb` |
| `paper` | `paper JSONB NOT NULL DEFAULT '{}'::jsonb` |
| `designer_version` | `designer_version TEXT NOT NULL` |
| `request_hash` | `request_hash TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `created_by` | `created_by UUID NOT NULL` |
| `published_at` | `published_at TIMESTAMPTZ` |
| `published_by` | `published_by UUID` |

### `print_records`

- 模块：h9_print_template_runtime
- 迁移：`backend/migrations/202607070001_h9_print_template_runtime.sql`
- 货主字段：有
- 索引：`print_records_owner_document_idx`
- ALTER 迁移：无
- 引用表：`print_template_versions`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `template_version_id` | `template_version_id UUID NOT NULL REFERENCES print_template_versions(id) ON DELETE RESTRICT` |
| `business_module` | `business_module TEXT NOT NULL` |
| `business_document_type` | `business_document_type TEXT NOT NULL` |
| `business_document_id` | `business_document_id TEXT NOT NULL` |
| `status` | `status TEXT NOT NULL CHECK (status IN ('printed', 'cancelled', 'failed'))` |
| `failure_reason` | `failure_reason TEXT` |
| `retry_count` | `retry_count INT NOT NULL DEFAULT 0 CHECK (retry_count >= 0)` |
| `printed_at` | `printed_at TIMESTAMPTZ NOT NULL` |
| `operator_id` | `operator_id UUID NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `audit_archive_partition_state`

- 模块：h2_lifecycle
- 迁移：`backend/migrations/202607080001_h2_lifecycle.sql`
- 货主字段：无
- 索引：`audit_archive_partition_state_tier_idx`
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `partition_name` | `partition_name TEXT PRIMARY KEY` |
| `partition_start` | `partition_start DATE NOT NULL` |
| `partition_end` | `partition_end DATE NOT NULL` |
| `storage_tier` | `storage_tier TEXT NOT NULL CHECK (storage_tier IN ('online', 'archive', 'deep_archive'))` |
| `target_tier` | `target_tier TEXT NOT NULL CHECK (target_tier IN ('online', 'archive', 'deep_archive'))` |
| `archived_at` | `archived_at TIMESTAMPTZ` |
| `last_run_id` | `last_run_id UUID` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `audit_archive_run`

- 模块：h2_lifecycle
- 迁移：`backend/migrations/202607080001_h2_lifecycle.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `idempotency_key` | `idempotency_key TEXT NOT NULL` |
| `reference_date` | `reference_date DATE NOT NULL` |
| `online_quarters` | `online_quarters INT NOT NULL CHECK (online_quarters > 0)` |
| `retention_years` | `retention_years INT NOT NULL CHECK (retention_years >= 5)` |
| `partitions_seen` | `partitions_seen INT NOT NULL CHECK (partitions_seen >= 0)` |
| `partitions_archived` | `partitions_archived INT NOT NULL CHECK (partitions_archived >= 0)` |
| `status` | `status TEXT NOT NULL CHECK (status IN ('completed', 'failed'))` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL` |

### `event_bus_subscription`

- 模块：h2_lifecycle
- 迁移：`backend/migrations/202607080001_h2_lifecycle.sql`
- 货主字段：有
- 索引：`event_bus_subscription_owner_active_idx`
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `subscriber_key` | `subscriber_key TEXT NOT NULL` |
| `event_pattern` | `event_pattern TEXT NOT NULL` |
| `active` | `active BOOLEAN NOT NULL DEFAULT TRUE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `event_bus_event`

- 模块：h2_lifecycle
- 迁移：`backend/migrations/202607080001_h2_lifecycle.sql`
- 货主字段：有
- 索引：`event_bus_event_owner_type_idx`
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `idempotency_key` | `idempotency_key TEXT NOT NULL` |
| `event_type` | `event_type TEXT NOT NULL` |
| `source_module` | `source_module TEXT NOT NULL` |
| `resource_type` | `resource_type TEXT NOT NULL` |
| `resource_id` | `resource_id TEXT NOT NULL` |
| `payload` | `payload JSONB NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL` |

### `event_bus_delivery`

- 模块：h2_lifecycle
- 迁移：`backend/migrations/202607080001_h2_lifecycle.sql`
- 货主字段：有
- 索引：`event_bus_delivery_owner_status_idx`
- ALTER 迁移：无
- 引用表：`event_bus_event`, `event_bus_subscription`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `event_id` | `event_id UUID NOT NULL REFERENCES event_bus_event(id) ON DELETE CASCADE` |
| `subscription_id` | `subscription_id UUID NOT NULL REFERENCES event_bus_subscription(id) ON DELETE CASCADE` |
| `status` | `status TEXT NOT NULL CHECK (status IN ('pending', 'delivered', 'dead_letter'))` |
| `attempt_count` | `attempt_count INT NOT NULL DEFAULT 0 CHECK (attempt_count >= 0)` |
| `last_error` | `last_error TEXT` |
| `next_attempt_at` | `next_attempt_at TIMESTAMPTZ` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `event_bus_dead_letter`

- 模块：h2_lifecycle
- 迁移：`backend/migrations/202607080001_h2_lifecycle.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`event_bus_delivery`, `event_bus_event`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `delivery_id` | `delivery_id UUID NOT NULL REFERENCES event_bus_delivery(id) ON DELETE CASCADE` |
| `event_id` | `event_id UUID NOT NULL REFERENCES event_bus_event(id) ON DELETE CASCADE` |
| `reason` | `reason TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL` |

### `business_retention_policy`

- 模块：h2_lifecycle
- 迁移：`backend/migrations/202607080001_h2_lifecycle.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `policy_code` | `policy_code TEXT NOT NULL` |
| `policy_name` | `policy_name TEXT NOT NULL` |
| `retention_years` | `retention_years INT` |
| `online_retention_months` | `online_retention_months INT NOT NULL CHECK (online_retention_months > 0)` |
| `permanent` | `permanent BOOLEAN NOT NULL DEFAULT FALSE` |
| `special_drug` | `special_drug BOOLEAN NOT NULL DEFAULT FALSE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `business_archive_job`

- 模块：h2_lifecycle
- 迁移：`backend/migrations/202607080001_h2_lifecycle.sql`
- 货主字段：有
- 索引：`business_archive_job_owner_status_idx`
- ALTER 迁移：无
- 引用表：`business_retention_policy`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `idempotency_key` | `idempotency_key TEXT NOT NULL` |
| `policy_id` | `policy_id UUID NOT NULL REFERENCES business_retention_policy(id)` |
| `table_name` | `table_name TEXT NOT NULL` |
| `target_layer` | `target_layer TEXT NOT NULL CHECK (target_layer IN ('archive', 'deep_archive', 'skip'))` |
| `cutoff_date` | `cutoff_date DATE` |
| `status` | `status TEXT NOT NULL CHECK (status IN ('planned', 'skipped'))` |
| `delete_allowed` | `delete_allowed BOOLEAN NOT NULL DEFAULT FALSE` |
| `skip_reason` | `skip_reason TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL` |

### `h4_notification_configs`

- 模块：h4_wechat_notify
- 迁移：`backend/migrations/202607080002_h4_wechat_notify.sql`
- 货主字段：有
- 索引：`h4_notification_configs_owner_event_idx`
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `event_type` | `event_type TEXT NOT NULL` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `template` | `template TEXT NOT NULL` |
| `recipient_rule` | `recipient_rule JSONB NOT NULL` |
| `channels` | `channels TEXT[] NOT NULL DEFAULT ARRAY['wechat']::TEXT[]` |
| `created_by` | `created_by UUID NOT NULL` |
| `updated_by` | `updated_by UUID NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `h4_notification_records`

- 模块：h4_wechat_notify
- 迁移：`backend/migrations/202607080002_h4_wechat_notify.sql`
- 货主字段：有
- 索引：`h4_notification_records_query_idx`
- ALTER 迁移：无
- 引用表：`h4_notification_configs`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `config_id` | `config_id UUID REFERENCES h4_notification_configs(id)` |
| `event_type` | `event_type TEXT NOT NULL` |
| `dedupe_key` | `dedupe_key TEXT NOT NULL` |
| `recipient` | `recipient TEXT NOT NULL` |
| `channel` | `channel TEXT NOT NULL` |
| `content` | `content TEXT NOT NULL` |
| `content_summary` | `content_summary TEXT NOT NULL` |
| `status` | `status TEXT NOT NULL CHECK (status IN ('success', 'failed', 'retrying'))` |
| `retry_count` | `retry_count INT NOT NULL DEFAULT 0` |
| `failure_reason` | `failure_reason TEXT` |
| `sent_at` | `sent_at TIMESTAMPTZ` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h4_approval_records`

- 模块：h4_wechat_notify
- 迁移：`backend/migrations/202607080002_h4_wechat_notify.sql`
- 货主字段：有
- 索引：`h4_approval_records_query_idx`
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `scenario` | `scenario TEXT NOT NULL` |
| `business_ref` | `business_ref TEXT NOT NULL` |
| `dedupe_key` | `dedupe_key TEXT NOT NULL` |
| `approver_user` | `approver_user TEXT NOT NULL` |
| `process_id` | `process_id TEXT NOT NULL` |
| `callback_path` | `callback_path TEXT NOT NULL` |
| `summary` | `summary TEXT NOT NULL` |
| `status` | `status TEXT NOT NULL CHECK (status IN ('pending', 'approved', 'rejected', 'failed'))` |
| `opinion` | `opinion TEXT` |
| `external_approval_id` | `external_approval_id TEXT` |
| `approved_by` | `approved_by TEXT` |
| `approved_at` | `approved_at TIMESTAMPTZ` |
| `failure_reason` | `failure_reason TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h5_express_carriers`

- 模块：h5_express
- 迁移：`backend/migrations/202607090002_h5_express.sql`
- 货主字段：有
- 索引：`h5_express_carriers_owner_status_idx`
- ALTER 迁移：无
- 引用表：`auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `carrier_code` | `carrier_code TEXT NOT NULL` |
| `carrier_name` | `carrier_name TEXT NOT NULL` |
| `api_url` | `api_url TEXT NOT NULL` |
| `api_key_alias` | `api_key_alias TEXT` |
| `api_secret_alias` | `api_secret_alias TEXT` |
| `account_no` | `account_no TEXT` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `priority` | `priority INT NOT NULL DEFAULT 100 CHECK (priority >= 0)` |
| `conditions` | `conditions JSONB NOT NULL DEFAULT '{}'::jsonb` |
| `status` | `status TEXT NOT NULL DEFAULT 'testing'` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h5_express_routing_rules`

- 模块：h5_express
- 迁移：`backend/migrations/202607090002_h5_express.sql`
- 货主字段：有
- 索引：`h5_express_routing_rules_owner_idx`
- ALTER 迁移：无
- 引用表：`auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `rule_code` | `rule_code TEXT NOT NULL` |
| `rule_name` | `rule_name TEXT NOT NULL` |
| `delivery_provider_type` | `delivery_provider_type TEXT NOT NULL` |
| `carrier_code` | `carrier_code TEXT` |
| `priority` | `priority INT NOT NULL DEFAULT 100 CHECK (priority >= 0)` |
| `conditions` | `conditions JSONB NOT NULL DEFAULT '{}'::jsonb` |
| `fallback_strategy` | `fallback_strategy TEXT` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `effective_from` | `effective_from TIMESTAMPTZ` |
| `effective_to` | `effective_to TIMESTAMPTZ` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h5_express_waybills`

- 模块：h5_express
- 迁移：`backend/migrations/202607090002_h5_express.sql`
- 货主字段：有
- 索引：`h5_express_waybills_owner_status_idx`
- ALTER 迁移：`backend/migrations/202607090003_h5_express_cancel_and_dedupe.sql`
- 引用表：`auth_owners`, `outbound_orders`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `outbound_order_id` | `outbound_order_id UUID REFERENCES outbound_orders(id) ON DELETE SET NULL` |
| `package_no` | `package_no TEXT NOT NULL` |
| `carrier_code` | `carrier_code TEXT NOT NULL` |
| `waybill_no` | `waybill_no TEXT NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'created'` |
| `sender_name` | `sender_name TEXT NOT NULL` |
| `sender_mobile` | `sender_mobile TEXT NOT NULL` |
| `sender_address` | `sender_address TEXT NOT NULL` |
| `receiver_name` | `receiver_name TEXT NOT NULL` |
| `receiver_mobile` | `receiver_mobile TEXT NOT NULL` |
| `receiver_address` | `receiver_address TEXT NOT NULL` |
| `weight_grams` | `weight_grams BIGINT NOT NULL CHECK (weight_grams > 0)` |
| `volume_cm3` | `volume_cm3 BIGINT NOT NULL CHECK (volume_cm3 >= 0)` |
| `package_count` | `package_count INT NOT NULL CHECK (package_count > 0)` |
| `eta_at` | `eta_at TIMESTAMPTZ` |
| `idempotency_key` | `idempotency_key TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h5_express_tracking_events`

- 模块：h5_express
- 迁移：`backend/migrations/202607090002_h5_express.sql`
- 货主字段：有
- 索引：`h5_express_tracking_events_waybill_idx`
- ALTER 迁移：无
- 引用表：`auth_owners`, `h5_express_waybills`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `waybill_id` | `waybill_id UUID NOT NULL REFERENCES h5_express_waybills(id) ON DELETE CASCADE` |
| `waybill_no` | `waybill_no TEXT NOT NULL` |
| `event_time` | `event_time TIMESTAMPTZ NOT NULL` |
| `status` | `status TEXT NOT NULL` |
| `location` | `location TEXT` |
| `description` | `description TEXT NOT NULL` |
| `source` | `source TEXT NOT NULL DEFAULT 'carrier_cache'` |
| `cached_at` | `cached_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h4_wechat_settings`

- 模块：h4_wechat_settings
- 迁移：`backend/migrations/202607090005_h4_wechat_settings.sql`
- 货主字段：有
- 索引：`h4_wechat_settings_owner_enabled_idx`
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL UNIQUE` |
| `corp_id` | `corp_id TEXT NOT NULL` |
| `agent_id` | `agent_id TEXT NOT NULL` |
| `secret_alias` | `secret_alias TEXT NOT NULL` |
| `callback_token_alias` | `callback_token_alias TEXT NOT NULL` |
| `aes_key_alias` | `aes_key_alias TEXT NOT NULL` |
| `callback_url` | `callback_url TEXT NOT NULL` |
| `approval_callback_path` | `approval_callback_path TEXT NOT NULL` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `retry_max_attempts` | `retry_max_attempts INT NOT NULL DEFAULT 3 CHECK (retry_max_attempts BETWEEN 0 AND 10)` |
| `retry_interval_seconds` | `retry_interval_seconds INT NOT NULL DEFAULT 60 CHECK (retry_interval_seconds BETWEEN 1 AND 3600)` |
| `created_by` | `created_by UUID NOT NULL` |
| `updated_by` | `updated_by UUID NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `auth_role_permission_exclusions`

- 模块：h1_role_management
- 迁移：`backend/migrations/202607120002_h1_role_management.sql`
- 货主字段：无
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_permissions`, `auth_roles`

| 字段 | SQL 定义 |
|---|---|
| `role_id` | `role_id UUID NOT NULL REFERENCES auth_roles(id) ON DELETE CASCADE` |
| `permission_id` | `permission_id UUID NOT NULL REFERENCES auth_permissions(id) ON DELETE CASCADE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `auth_sessions`

- 模块：h1_auth_sessions
- 迁移：`backend/migrations/202607120003_h1_auth_sessions.sql`
- 货主字段：有
- 索引：`auth_sessions_owner_user_active_idx`
- ALTER 迁移：无
- 引用表：`auth_user_owner_bindings`

| 字段 | SQL 定义 |
|---|---|
| `session_id` | `session_id TEXT PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `user_id` | `user_id UUID NOT NULL` |
| `device_name` | `device_name TEXT NOT NULL` |
| `ip` | `ip INET` |
| `logged_in_at` | `logged_in_at TIMESTAMPTZ NOT NULL` |
| `expires_at` | `expires_at TIMESTAMPTZ NOT NULL` |
| `revoked_at` | `revoked_at TIMESTAMPTZ` |
| `revoke_reason` | `revoke_reason TEXT` |
| `revoked_by` | `revoked_by UUID` |

### `auth_api_keys`

- 模块：h1_api_key_lifecycle
- 迁移：`backend/migrations/202607120004_h1_api_key_lifecycle.sql`
- 货主字段：有
- 索引：`auth_api_keys_owner_status_idx`, `auth_api_keys_responsible_user_idx`
- ALTER 迁移：无
- 引用表：`auth_api_keys`, `auth_owners`, `auth_users`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `caller_name` | `caller_name TEXT NOT NULL` |
| `purpose` | `purpose TEXT NOT NULL` |
| `warehouse_ids` | `warehouse_ids UUID[] NOT NULL DEFAULT '{}'::uuid[]` |
| `scopes` | `scopes TEXT[] NOT NULL` |
| `responsible_user_id` | `responsible_user_id UUID NOT NULL REFERENCES auth_users(id)` |
| `key_hash` | `key_hash TEXT NOT NULL UNIQUE` |
| `status` | `status TEXT NOT NULL DEFAULT 'active'` |
| `expires_at` | `expires_at TIMESTAMPTZ NOT NULL` |
| `grace_expires_at` | `grace_expires_at TIMESTAMPTZ` |
| `replaced_by_key_id` | `replaced_by_key_id UUID REFERENCES auth_api_keys(id)` |
| `revoked_at` | `revoked_at TIMESTAMPTZ` |
| `temporarily_disabled_until` | `temporarily_disabled_until TIMESTAMPTZ` |
| `failed_auth_count` | `failed_auth_count INT NOT NULL DEFAULT 0 CHECK (failed_auth_count >= 0)` |
| `failed_auth_window_started_at` | `failed_auth_window_started_at TIMESTAMPTZ` |
| `rate_limit_window_started_at` | `rate_limit_window_started_at TIMESTAMPTZ` |
| `rate_limit_count` | `rate_limit_count INT NOT NULL DEFAULT 0 CHECK (rate_limit_count >= 0)` |
| `last_used_at` | `last_used_at TIMESTAMPTZ` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `inventory_allocations`

- 模块：m3_inventory_allocations
- 迁移：`backend/migrations/202607130003_m3_inventory_allocations.sql`
- 货主字段：有
- 索引：`inventory_allocations_owner_order_status_idx`
- ALTER 迁移：无
- 引用表：`inventory_batches`, `outbound_orders`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `outbound_order_id` | `outbound_order_id UUID NOT NULL` |
| `line_no` | `line_no INT NOT NULL CHECK (line_no > 0)` |
| `batch_id` | `batch_id UUID NOT NULL` |
| `allocated_qty` | `allocated_qty BIGINT NOT NULL CHECK (allocated_qty > 0)` |
| `status` | `status TEXT NOT NULL CHECK (status IN ('locked', 'consumed'))` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `consumed_at` | `consumed_at TIMESTAMPTZ` |

### `warehouse_docks`

- 模块：dock_master
- 迁移：`backend/migrations/202607130008_dock_master.sql`
- 货主字段：无
- 索引：`UNIQUE warehouse_docks_id_warehouse_id_uidx`
- ALTER 迁移：无
- 引用表：`warehouses`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `warehouse_id` | `warehouse_id UUID NOT NULL REFERENCES warehouses(id) ON DELETE RESTRICT` |
| `dock_code` | `dock_code VARCHAR(32) NOT NULL` |
| `dock_type` | `dock_type TEXT NOT NULL` |
| `temperature_zone` | `temperature_zone TEXT NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'active'` |
| `maintenance_recovery_at` | `maintenance_recovery_at TIMESTAMPTZ` |
| `location_description` | `location_description TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `inventory_recall_actions`

- 模块：m3_recall_actions
- 迁移：`backend/migrations/202607130010_m3_recall_actions.sql`
- 货主字段：有
- 索引：`UNIQUE inventory_recall_actions_active_batch_idx`, `inventory_recall_actions_owner_batch_idx`
- ALTER 迁移：无
- 引用表：`inventory_batches`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `batch_id` | `batch_id UUID NOT NULL` |
| `recall_approval_source` | `recall_approval_source TEXT NOT NULL` |
| `recall_approval_id` | `recall_approval_id TEXT NOT NULL` |
| `previous_quality_status` | `previous_quality_status TEXT NOT NULL` |
| `marked_by` | `marked_by UUID NOT NULL` |
| `marked_at` | `marked_at TIMESTAMPTZ NOT NULL` |
| `canceled_by` | `canceled_by UUID` |
| `canceled_at` | `canceled_at TIMESTAMPTZ` |
| `cancel_approval_id` | `cancel_approval_id TEXT` |
| `cancel_reason` | `cancel_reason TEXT` |

### `dock_appointments`

- 模块：dock_maintenance_permissions
- 迁移：`backend/migrations/202607130014_dock_maintenance_permissions.sql`
- 货主字段：有
- 索引：`UNIQUE ux_dock_appointments_active`, `UNIQUE ux_dock_appointments_appointment_no`, `idx_dock_appointments_supersedes`
- ALTER 迁移：无
- 引用表：`dock_appointments`, `warehouse_docks`, `warehouses`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `dock_id` | `dock_id UUID NOT NULL` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `warehouse_id` | `warehouse_id UUID NOT NULL` |
| `appointment_no` | `appointment_no TEXT NOT NULL` |
| `document_type` | `document_type TEXT NOT NULL` |
| `document_no` | `document_no TEXT NOT NULL` |
| `window_start_at` | `window_start_at TIMESTAMPTZ NOT NULL` |
| `window_end_at` | `window_end_at TIMESTAMPTZ NOT NULL` |
| `vehicle_plate_no` | `vehicle_plate_no TEXT NOT NULL DEFAULT ''` |
| `vehicle_type` | `vehicle_type TEXT NOT NULL` |
| `driver_name` | `driver_name TEXT NOT NULL` |
| `driver_phone` | `driver_phone TEXT NOT NULL DEFAULT ''` |
| `status` | `status TEXT NOT NULL DEFAULT 'pending'` |
| `supersedes_id` | `supersedes_id UUID REFERENCES dock_appointments(id) ON DELETE RESTRICT` |
| `arrived_at` | `arrived_at TIMESTAMPTZ` |
| `arrival_deviation_minutes` | `arrival_deviation_minutes BIGINT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `alert_definitions`

- 模块：h1_alert_definitions
- 迁移：`backend/migrations/202607130015_h1_alert_definitions.sql`
- 货主字段：有
- 索引：`alert_definitions_owner_idx`
- ALTER 迁移：`backend/migrations/202607150011_hal_alert_definition_workflow.sql`
- 引用表：`auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `alert_code` | `alert_code TEXT NOT NULL` |
| `name` | `name TEXT NOT NULL` |
| `event_type` | `event_type TEXT NOT NULL` |
| `condition_expression` | `condition_expression TEXT NOT NULL` |
| `default_severity` | `default_severity TEXT NOT NULL` |
| `recipient_roles` | `recipient_roles TEXT[] NOT NULL DEFAULT '{}'` |
| `escalation_ref` | `escalation_ref TEXT` |
| `silence_period_seconds` | `silence_period_seconds BIGINT NOT NULL DEFAULT 0` |
| `is_disable_allowed` | `is_disable_allowed BOOLEAN NOT NULL DEFAULT TRUE` |
| `message_template` | `message_template TEXT NOT NULL` |
| `is_gsp_forced` | `is_gsp_forced BOOLEAN NOT NULL DEFAULT FALSE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `alert_definition_triggers`

- 模块：h1_alert_definitions
- 迁移：`backend/migrations/202607130015_h1_alert_definitions.sql`
- 货主字段：无
- 索引：`alert_definition_triggers_definition_idx`
- ALTER 迁移：无
- 引用表：`alert_definitions`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `alert_definition_id` | `alert_definition_id UUID NOT NULL REFERENCES alert_definitions(id) ON DELETE RESTRICT` |
| `event_type` | `event_type TEXT NOT NULL` |
| `occurred_at` | `occurred_at TIMESTAMPTZ NOT NULL` |
| `payload` | `payload JSONB NOT NULL DEFAULT '{}'::jsonb` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `config_center_feature_flags`

- 模块：m1_config_center_feature_flags
- 迁移：`backend/migrations/202607140001_m1_config_center_feature_flags.sql`
- 货主字段：有
- 索引：`config_center_feature_flags_owner_idx`
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `owner_id` | `owner_id UUID NOT NULL` |
| `flag_key` | `flag_key TEXT NOT NULL` |
| `owner` | `owner TEXT NOT NULL` |
| `created_at` | `created_at TEXT NOT NULL` |
| `cleanup_by` | `cleanup_by TEXT NOT NULL` |
| `enabled` | `enabled BOOLEAN NOT NULL` |
| `source` | `source TEXT NOT NULL` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `tms_route_plans`

- 模块：m10_tms_route_plans
- 迁移：`backend/migrations/202607140003_m10_tms_route_plans.sql`
- 货主字段：有
- 索引：`tms_route_plans_owner_delivery_date_idx`
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `dispatch_result_id` | `dispatch_result_id TEXT NOT NULL` |
| `delivery_date` | `delivery_date DATE NOT NULL` |
| `vehicle_no` | `vehicle_no TEXT NOT NULL` |
| `plate_no` | `plate_no TEXT NOT NULL` |
| `driver_user_id` | `driver_user_id UUID NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'received'` |
| `planning_version` | `planning_version INT NOT NULL CHECK (planning_version > 0)` |
| `payload_hash` | `payload_hash TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `tms_route_stops`

- 模块：m10_tms_route_plans
- 迁移：`backend/migrations/202607140003_m10_tms_route_plans.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`tms_route_plans`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `route_plan_id` | `route_plan_id UUID NOT NULL` |
| `store_id` | `store_id UUID NOT NULL` |
| `stop_sequence` | `stop_sequence INT NOT NULL CHECK (stop_sequence > 0)` |
| `estimated_arrival_at` | `estimated_arrival_at TIMESTAMPTZ NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `tms_route_orders`

- 模块：m10_tms_route_plans
- 迁移：`backend/migrations/202607140003_m10_tms_route_plans.sql`
- 货主字段：有
- 索引：`tms_route_orders_owner_order_idx`
- ALTER 迁移：无
- 引用表：`outbound_orders`, `tms_route_plans`, `tms_route_stops`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `route_plan_id` | `route_plan_id UUID NOT NULL` |
| `route_stop_id` | `route_stop_id UUID NOT NULL` |
| `outbound_order_id` | `outbound_order_id UUID NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `outbound_pick_tasks`

- 模块：m4_outbound_pick_tasks
- 迁移：`backend/migrations/202607140004_m4_outbound_pick_tasks.sql`
- 货主字段：有
- 索引：`outbound_pick_tasks_owner_wave_route_idx`
- ALTER 迁移：无
- 引用表：`inventory_batches`, `outbound_orders`, `outbound_waves`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `wave_id` | `wave_id UUID NOT NULL` |
| `outbound_order_id` | `outbound_order_id UUID NOT NULL` |
| `line_no` | `line_no INT NOT NULL CHECK (line_no > 0)` |
| `batch_id` | `batch_id UUID NOT NULL` |
| `product_code` | `product_code TEXT NOT NULL` |
| `batch_no` | `batch_no TEXT NOT NULL` |
| `location_id` | `location_id UUID NOT NULL` |
| `location_code` | `location_code TEXT NOT NULL` |
| `planned_qty` | `planned_qty BIGINT NOT NULL CHECK (planned_qty > 0)` |
| `picked_qty` | `picked_qty BIGINT NOT NULL DEFAULT 0 CHECK (picked_qty >= 0)` |
| `status` | `status TEXT NOT NULL DEFAULT 'pending_assignment' CHECK (status IN ('pending_assignment', 'assigned', 'dispatched', 'in_progress', 'completed', 'exception', 'cancelled'))` |
| `route_sequence` | `route_sequence INT NOT NULL CHECK (route_sequence > 0)` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `inventory_status_transitions`

- 模块：m3_inventory_status_transitions
- 迁移：`backend/migrations/202607140005_m3_inventory_status_transitions.sql`
- 货主字段：有
- 索引：`UNIQUE inventory_status_transitions_scope_uidx`, `inventory_status_transitions_owner_lookup_idx`
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID` |
| `from_status` | `from_status TEXT NOT NULL` |
| `to_status` | `to_status TEXT NOT NULL` |
| `approval_sources` | `approval_sources TEXT[] NOT NULL` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `inventory_maintenance_tasks`

- 模块：m3_inventory_maintenance
- 迁移：`backend/migrations/202607140006_m3_inventory_maintenance.sql`
- 货主字段：有
- 索引：`inventory_maintenance_tasks_owner_batch_idx`, `inventory_maintenance_tasks_owner_status_idx`
- ALTER 迁移：无
- 引用表：`inventory_batches`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `inventory_batch_id` | `inventory_batch_id UUID NOT NULL REFERENCES inventory_batches(id)` |
| `planned_at` | `planned_at TIMESTAMPTZ NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'pending'` |
| `assigned_user_id` | `assigned_user_id UUID` |
| `completed_at` | `completed_at TIMESTAMPTZ` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `inventory_maintenance_records`

- 模块：m3_inventory_maintenance
- 迁移：`backend/migrations/202607140006_m3_inventory_maintenance.sql`
- 货主字段：有
- 索引：`inventory_maintenance_records_owner_batch_idx`, `inventory_maintenance_records_owner_task_idx`
- ALTER 迁移：无
- 引用表：`inventory_batches`, `inventory_maintenance_tasks`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `task_id` | `task_id UUID NOT NULL REFERENCES inventory_maintenance_tasks(id)` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `inventory_batch_id` | `inventory_batch_id UUID NOT NULL REFERENCES inventory_batches(id)` |
| `product_code` | `product_code TEXT NOT NULL` |
| `batch_no` | `batch_no TEXT NOT NULL` |
| `expiry_date` | `expiry_date DATE NOT NULL` |
| `inventory_status` | `inventory_status TEXT NOT NULL` |
| `temperature_celsius` | `temperature_celsius DOUBLE PRECISION NOT NULL` |
| `humidity_percent` | `humidity_percent DOUBLE PRECISION NOT NULL` |
| `appearance` | `appearance TEXT NOT NULL` |
| `packaging` | `packaging TEXT NOT NULL` |
| `pest` | `pest TEXT NOT NULL` |
| `rodent` | `rodent TEXT NOT NULL` |
| `mildew` | `mildew TEXT NOT NULL` |
| `conclusion` | `conclusion TEXT NOT NULL` |
| `exception_type` | `exception_type TEXT` |
| `notes` | `notes TEXT` |
| `performed_by` | `performed_by UUID NOT NULL` |
| `performed_at` | `performed_at TIMESTAMPTZ NOT NULL` |
| `performed_date` | `performed_date DATE NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `inventory_counts`

- 模块：m3_inventory_counts
- 迁移：`backend/migrations/202607140008_m3_inventory_counts.sql`
- 货主字段：有
- 索引：`inventory_counts_owner_status_idx`
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `count_type` | `count_type TEXT NOT NULL CHECK (count_type IN ('cycle', 'full', 'blind'))` |
| `warehouse_id` | `warehouse_id UUID` |
| `zone_id` | `zone_id UUID` |
| `product_code` | `product_code TEXT` |
| `status` | `status TEXT NOT NULL DEFAULT 'in_progress' CHECK (status IN ('in_progress', 'pending_approval', 'approved'))` |
| `started_at` | `started_at TIMESTAMPTZ NOT NULL` |
| `created_by` | `created_by UUID NOT NULL` |
| `approved_by` | `approved_by UUID` |
| `approved_at` | `approved_at TIMESTAMPTZ` |
| `approval_source` | `approval_source TEXT` |
| `approval_id` | `approval_id TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `inventory_count_lines`

- 模块：m3_inventory_counts
- 迁移：`backend/migrations/202607140008_m3_inventory_counts.sql`
- 货主字段：有
- 索引：`inventory_count_lines_owner_batch_idx`
- ALTER 迁移：无
- 引用表：`inventory_batches`, `inventory_counts`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `count_id` | `count_id UUID NOT NULL` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `inventory_batch_id` | `inventory_batch_id UUID NOT NULL` |
| `location_id` | `location_id UUID NOT NULL` |
| `location_code` | `location_code TEXT NOT NULL` |
| `product_code` | `product_code TEXT NOT NULL` |
| `batch_no` | `batch_no TEXT NOT NULL` |
| `book_qty` | `book_qty BIGINT NOT NULL CHECK (book_qty >= 0)` |
| `physical_qty` | `physical_qty BIGINT CHECK (physical_qty >= 0)` |
| `variance_qty` | `variance_qty BIGINT` |
| `variance_type` | `variance_type TEXT CHECK (variance_type IS NULL OR variance_type IN ('gain', 'loss', 'none'))` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `drug_inspection_platforms`

- 模块：mdi_drug_inspection_platforms
- 迁移：`backend/migrations/202607140009_mdi_drug_inspection_platforms.sql`
- 货主字段：有
- 索引：`drug_inspection_platforms_owner_status_idx`
- ALTER 迁移：无
- 引用表：`auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `platform_code` | `platform_code TEXT NOT NULL` |
| `platform_name` | `platform_name TEXT NOT NULL` |
| `api_url` | `api_url TEXT NOT NULL` |
| `auth_method` | `auth_method TEXT NOT NULL` |
| `api_key_alias` | `api_key_alias TEXT` |
| `username` | `username TEXT` |
| `password_alias` | `password_alias TEXT` |
| `timeout_seconds` | `timeout_seconds INT NOT NULL DEFAULT 30` |
| `status` | `status TEXT NOT NULL DEFAULT 'testing'` |
| `created_by` | `created_by UUID NOT NULL` |
| `updated_by` | `updated_by UUID NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `task_types`

- 模块：mte_task_types
- 迁移：`backend/migrations/202607140010_mte_task_types.sql`
- 货主字段：有
- 索引：`UNIQUE task_types_owner_code_lower_idx`, `task_types_owner_enabled_idx`
- ALTER 迁移：`backend/migrations/202607150007_mte_task_release_control.sql`
- 引用表：`auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `task_type_code` | `task_type_code TEXT NOT NULL CHECK (task_type_code ~ '^[a-z0-9][a-z0-9_.-]{0,63}$')` |
| `task_type_name` | `task_type_name TEXT NOT NULL CHECK (length(trim(task_type_name)) BETWEEN 1 AND 128)` |
| `default_priority` | `default_priority INT NOT NULL CHECK (default_priority BETWEEN 0 AND 1000)` |
| `estimated_minutes` | `estimated_minutes INT NOT NULL CHECK (estimated_minutes BETWEEN 1 AND 10080)` |
| `mergeable` | `mergeable BOOLEAN NOT NULL` |
| `insertable` | `insertable BOOLEAN NOT NULL` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1 CHECK (version > 0)` |

### `dual_person_policy_rules`

- 模块：mvr_dual_person_policy
- 迁移：`backend/migrations/202607150001_mvr_dual_person_policy.sql`
- 货主字段：有
- 索引：`UNIQUE dual_person_policy_rules_scope_idx`, `dual_person_policy_rules_resolve_idx`
- ALTER 迁移：无
- 引用表：`auth_owners`, `auth_users`, `system_dictionary_items`, `warehouses`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `special_drug_category` | `special_drug_category TEXT NOT NULL` |
| `process_code` | `process_code TEXT NOT NULL` |
| `node_code` | `node_code TEXT NOT NULL` |
| `owner_id` | `owner_id UUID REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `warehouse_id` | `warehouse_id UUID` |
| `policy` | `policy TEXT NOT NULL CHECK (policy IN ('single', 'dual_scan', 'dual_scan_with_approval'))` |
| `priority` | `priority INT NOT NULL DEFAULT 100 CHECK (priority BETWEEN 0 AND 1000)` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `source_dictionary_item_id` | `source_dictionary_item_id UUID REFERENCES system_dictionary_items(id) ON DELETE CASCADE` |
| `confirmed_by_user_id` | `confirmed_by_user_id UUID REFERENCES auth_users(id) ON DELETE RESTRICT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1 CHECK (version > 0)` |

### `outbound_review_records`

- 模块：mvr_downstream_enforcement
- 迁移：`backend/migrations/202607150002_mvr_downstream_enforcement.sql`
- 货主字段：有
- 索引：`outbound_review_records_owner_order_idx`
- ALTER 迁移：无
- 引用表：`auth_owners`, `dual_person_policy_rules`, `h4_approval_records`, `outbound_orders`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `outbound_order_id` | `outbound_order_id UUID NOT NULL REFERENCES outbound_orders(id) ON DELETE RESTRICT` |
| `review_mode` | `review_mode TEXT NOT NULL` |
| `first_reviewer_id` | `first_reviewer_id UUID NOT NULL` |
| `second_reviewer_id` | `second_reviewer_id UUID` |
| `strategy_rule_id` | `strategy_rule_id UUID REFERENCES dual_person_policy_rules(id) ON DELETE RESTRICT` |
| `approval_record_id` | `approval_record_id UUID REFERENCES h4_approval_records(id) ON DELETE RESTRICT` |
| `reviewed_at` | `reviewed_at TIMESTAMPTZ NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `task_groups`

- 模块：mte_task_execution
- 迁移：`backend/migrations/202607150003_mte_task_execution.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`, `warehouses`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `task_group_code` | `task_group_code TEXT NOT NULL CHECK (task_group_code ~ '^[a-z0-9][a-z0-9_.-]{0,63}$')` |
| `task_group_name` | `task_group_name TEXT NOT NULL CHECK (length(trim(task_group_name)) BETWEEN 1 AND 128)` |
| `warehouse_id` | `warehouse_id UUID NOT NULL` |
| `zone_ids` | `zone_ids UUID[] NOT NULL DEFAULT '{}'` |
| `task_type_codes` | `task_type_codes TEXT[] NOT NULL CHECK (cardinality(task_type_codes) > 0)` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1 CHECK (version > 0)` |

### `task_group_memberships`

- 模块：mte_task_execution
- 迁移：`backend/migrations/202607150003_mte_task_execution.sql`
- 货主字段：有
- 索引：`task_group_memberships_owner_user_idx`
- ALTER 迁移：`backend/migrations/202607150005_mte_worker_qualifications.sql`
- 引用表：`auth_owners`, `auth_user_owner_bindings`, `auth_users`, `task_groups`

| 字段 | SQL 定义 |
|---|---|
| `task_group_id` | `task_group_id UUID NOT NULL REFERENCES task_groups(id) ON DELETE CASCADE` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `user_id` | `user_id UUID NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `warehouse_tasks`

- 模块：mte_task_execution
- 迁移：`backend/migrations/202607150003_mte_task_execution.sql`
- 货主字段：有
- 索引：`UNIQUE warehouse_tasks_source_identity_idx`, `warehouse_tasks_owner_assignee_idx`, `warehouse_tasks_owner_queue_idx`, `warehouse_tasks_owner_source_idx`
- ALTER 迁移：`backend/migrations/202607150006_mte_task_priority_rules.sql`, `backend/migrations/202607150007_mte_task_release_control.sql`
- 引用表：`auth_owners`, `auth_users`, `task_groups`, `task_types`, `warehouse_tasks`, `warehouses`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `task_no` | `task_no TEXT NOT NULL` |
| `task_type_code` | `task_type_code TEXT NOT NULL` |
| `source_module` | `source_module TEXT NOT NULL` |
| `source_doc_type` | `source_doc_type TEXT NOT NULL` |
| `source_doc_id` | `source_doc_id UUID` |
| `source_doc_no` | `source_doc_no TEXT NOT NULL` |
| `source_line_no` | `source_line_no INT` |
| `source_task_key` | `source_task_key TEXT NOT NULL` |
| `warehouse_id` | `warehouse_id UUID NOT NULL` |
| `task_group_code` | `task_group_code TEXT NOT NULL` |
| `product_id` | `product_id UUID` |
| `product_code` | `product_code TEXT NOT NULL` |
| `batch_id` | `batch_id UUID` |
| `batch_no` | `batch_no TEXT` |
| `planned_qty` | `planned_qty BIGINT NOT NULL CHECK (planned_qty > 0)` |
| `actual_qty` | `actual_qty BIGINT CHECK (actual_qty >= 0)` |
| `source_location_id` | `source_location_id UUID` |
| `source_location_code` | `source_location_code TEXT` |
| `target_location_id` | `target_location_id UUID` |
| `target_location_code` | `target_location_code TEXT` |
| `priority` | `priority INT NOT NULL CHECK (priority BETWEEN 0 AND 1000)` |
| `estimated_minutes` | `estimated_minutes INT NOT NULL CHECK (estimated_minutes BETWEEN 1 AND 10080)` |
| `assignee_user_id` | `assignee_user_id UUID REFERENCES auth_users(id)` |
| `status` | `status TEXT NOT NULL DEFAULT 'pending_assignment'` |
| `exception_code` | `exception_code TEXT` |
| `exception_note` | `exception_note TEXT` |
| `assigned_at` | `assigned_at TIMESTAMPTZ` |
| `dispatched_at` | `dispatched_at TIMESTAMPTZ` |
| `started_at` | `started_at TIMESTAMPTZ` |
| `completed_at` | `completed_at TIMESTAMPTZ` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1 CHECK (version > 0)` |

### `task_execution_events`

- 模块：mte_task_execution
- 迁移：`backend/migrations/202607150003_mte_task_execution.sql`
- 货主字段：有
- 索引：`task_execution_events_owner_task_idx`
- ALTER 迁移：无
- 引用表：`auth_owners`, `auth_users`, `warehouse_tasks`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `task_id` | `task_id UUID NOT NULL REFERENCES warehouse_tasks(id) ON DELETE RESTRICT` |
| `action` | `action TEXT NOT NULL` |
| `from_status` | `from_status TEXT` |
| `to_status` | `to_status TEXT NOT NULL` |
| `actor_user_id` | `actor_user_id UUID NOT NULL` |
| `assignee_user_id` | `assignee_user_id UUID REFERENCES auth_users(id) ON DELETE RESTRICT` |
| `actual_qty` | `actual_qty BIGINT` |
| `exception_code` | `exception_code TEXT` |
| `exception_note` | `exception_note TEXT` |
| `occurred_at` | `occurred_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `task_priority_rules`

- 模块：mte_task_priority_rules
- 迁移：`backend/migrations/202607150006_mte_task_priority_rules.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL UNIQUE REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `urgent_order_bonus` | `urgent_order_bonus INT NOT NULL DEFAULT 20 CHECK (urgent_order_bonus BETWEEN 0 AND 1000)` |
| `waiting_minutes_per_point` | `waiting_minutes_per_point INT NOT NULL DEFAULT 30 CHECK (waiting_minutes_per_point BETWEEN 1 AND 1440)` |
| `cold_chain_bonus` | `cold_chain_bonus INT NOT NULL DEFAULT 20 CHECK (cold_chain_bonus BETWEEN 0 AND 1000)` |
| `manual_expedite_bonus` | `manual_expedite_bonus INT NOT NULL DEFAULT 50 CHECK (manual_expedite_bonus BETWEEN 0 AND 1000)` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1 CHECK (version > 0)` |

### `stock_adjustment_orders`

- 模块：msa_stock_loss
- 迁移：`backend/migrations/202607150008_msa_stock_loss.sql`
- 货主字段：有
- 索引：`UNIQUE stock_adjustment_orders_erp_ref_uidx`, `stock_adjustment_orders_query_idx`
- ALTER 迁移：`backend/migrations/202607150009_msa_stock_surplus.sql`
- 引用表：`auth_owners`, `auth_users`, `dual_person_policy_rules`, `h4_approval_records`, `inventory_batches`, `warehouses`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `warehouse_id` | `warehouse_id UUID NOT NULL` |
| `order_no` | `order_no TEXT NOT NULL` |
| `adjustment_type` | `adjustment_type TEXT NOT NULL CHECK (adjustment_type IN ('loss', 'surplus'))` |
| `batch_id` | `batch_id UUID NOT NULL REFERENCES inventory_batches(id) ON DELETE RESTRICT` |
| `product_code` | `product_code TEXT NOT NULL` |
| `batch_no` | `batch_no TEXT NOT NULL` |
| `quantity` | `quantity BIGINT NOT NULL CHECK (quantity > 0)` |
| `reason_code` | `reason_code TEXT NOT NULL CHECK (reason_code IN ( 'expired', 'damaged', 'quality_unqualified', 'inventory_loss', 'destruction', 'recall_destruction', 'other' ))` |
| `recall_id` | `recall_id TEXT` |
| `source` | `source TEXT NOT NULL CHECK (source IN ('erp', 'manual'))` |
| `external_ref` | `external_ref TEXT` |
| `status` | `status TEXT NOT NULL CHECK (status IN ( 'pending_approval', 'pending_execution', 'in_progress', 'completed', 'rejected', 'cancelled', 'exception_suspended' ))` |
| `requires_quality_approval` | `requires_quality_approval BOOLEAN NOT NULL DEFAULT FALSE` |
| `quality_liaison_id` | `quality_liaison_id TEXT` |
| `policy` | `policy TEXT CHECK (policy IN ('single', 'dual_scan', 'dual_scan_with_approval'))` |
| `source_rule_id` | `source_rule_id UUID REFERENCES dual_person_policy_rules(id) ON DELETE RESTRICT` |
| `first_operator_id` | `first_operator_id UUID REFERENCES auth_users(id) ON DELETE RESTRICT` |
| `second_operator_id` | `second_operator_id UUID REFERENCES auth_users(id) ON DELETE RESTRICT` |
| `approval_record_id` | `approval_record_id UUID REFERENCES h4_approval_records(id) ON DELETE RESTRICT` |
| `started_at` | `started_at TIMESTAMPTZ` |
| `completed_at` | `completed_at TIMESTAMPTZ` |
| `created_by` | `created_by UUID NOT NULL REFERENCES auth_users(id) ON DELETE RESTRICT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1 CHECK (version > 0)` |

### `stock_adjustment_execution_records`

- 模块：msa_stock_loss
- 迁移：`backend/migrations/202607150008_msa_stock_loss.sql`
- 货主字段：有
- 索引：`stock_adjustment_execution_owner_idx`
- ALTER 迁移：无
- 引用表：`auth_owners`, `auth_users`, `dual_person_policy_rules`, `h4_approval_records`, `stock_adjustment_orders`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `order_id` | `order_id UUID NOT NULL UNIQUE REFERENCES stock_adjustment_orders(id) ON DELETE RESTRICT` |
| `process_code` | `process_code TEXT NOT NULL CHECK (process_code IN ('报损', '报溢', '销毁'))` |
| `node_code` | `node_code TEXT NOT NULL CHECK (node_code IN ('报损执行', '报溢执行', '销毁执行'))` |
| `policy` | `policy TEXT NOT NULL CHECK (policy IN ('single', 'dual_scan', 'dual_scan_with_approval'))` |
| `source_rule_id` | `source_rule_id UUID REFERENCES dual_person_policy_rules(id) ON DELETE RESTRICT` |
| `first_operator_id` | `first_operator_id UUID NOT NULL REFERENCES auth_users(id) ON DELETE RESTRICT` |
| `second_operator_id` | `second_operator_id UUID REFERENCES auth_users(id) ON DELETE RESTRICT` |
| `approval_record_id` | `approval_record_id UUID REFERENCES h4_approval_records(id) ON DELETE RESTRICT` |
| `quantity` | `quantity BIGINT NOT NULL CHECK (quantity > 0)` |
| `executed_at` | `executed_at TIMESTAMPTZ NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `stock_adjustment_erp_feedback_outbox`

- 模块：msa_stock_loss
- 迁移：`backend/migrations/202607150008_msa_stock_loss.sql`
- 货主字段：有
- 索引：`stock_adjustment_erp_feedback_pending_idx`
- ALTER 迁移：无
- 引用表：`auth_owners`, `stock_adjustment_orders`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `order_id` | `order_id UUID NOT NULL UNIQUE REFERENCES stock_adjustment_orders(id) ON DELETE RESTRICT` |
| `event_type` | `event_type TEXT NOT NULL CHECK (event_type IN ('stock_loss_completed', 'stock_surplus_completed'))` |
| `payload` | `payload JSONB NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'sending', 'succeeded', 'failed'))` |
| `attempt_count` | `attempt_count INT NOT NULL DEFAULT 0 CHECK (attempt_count >= 0)` |
| `next_attempt_at` | `next_attempt_at TIMESTAMPTZ NOT NULL` |
| `last_error` | `last_error TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `quality_liaison_types`

- 模块：mql_quality_liaison
- 迁移：`backend/migrations/202607150010_mql_quality_liaison.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`, `auth_users`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `type_code` | `type_code TEXT NOT NULL` |
| `type_name` | `type_name TEXT NOT NULL` |
| `approval_template_id` | `approval_template_id TEXT NOT NULL` |
| `approver_user_id` | `approver_user_id UUID NOT NULL REFERENCES auth_users(id) ON DELETE RESTRICT` |
| `timeout_seconds` | `timeout_seconds INT NOT NULL CHECK (timeout_seconds > 0)` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `created_by` | `created_by UUID NOT NULL REFERENCES auth_users(id) ON DELETE RESTRICT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1 CHECK (version > 0)` |

### `quality_liaison_orders`

- 模块：mql_quality_liaison
- 迁移：`backend/migrations/202607150010_mql_quality_liaison.sql`
- 货主字段：有
- 索引：`quality_liaison_orders_query_idx`
- ALTER 迁移：无
- 引用表：`auth_owners`, `auth_users`, `h4_approval_records`, `quality_liaison_types`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `liaison_no` | `liaison_no TEXT NOT NULL` |
| `type_code` | `type_code TEXT NOT NULL` |
| `related_document_type` | `related_document_type TEXT NOT NULL` |
| `related_document_no` | `related_document_no TEXT NOT NULL` |
| `problem_description` | `problem_description TEXT NOT NULL` |
| `disposition_suggestion` | `disposition_suggestion TEXT NOT NULL` |
| `trigger_source` | `trigger_source TEXT NOT NULL` |
| `business_payload` | `business_payload JSONB NOT NULL DEFAULT '{}'::jsonb` |
| `status` | `status TEXT NOT NULL CHECK (status IN ( 'pending_approval', 'approved', 'rejected', 'pending_erp_sync', 'landed', 'sync_failed', 'closed' ))` |
| `approval_record_id` | `approval_record_id UUID UNIQUE REFERENCES h4_approval_records(id) ON DELETE RESTRICT` |
| `approved_by` | `approved_by UUID REFERENCES auth_users(id) ON DELETE RESTRICT` |
| `approval_opinion` | `approval_opinion TEXT` |
| `approved_at` | `approved_at TIMESTAMPTZ` |
| `created_by` | `created_by UUID NOT NULL REFERENCES auth_users(id) ON DELETE RESTRICT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1 CHECK (version > 0)` |

### `alert_instances`

- 模块：hal_alert_runtime
- 迁移：`backend/migrations/202607150012_hal_alert_runtime.sql`
- 货主字段：有
- 索引：`alert_instances_owner_active_idx`, `alert_instances_owner_resource_idx`
- ALTER 迁移：无
- 引用表：`alert_definitions`, `auth_owners`, `event_bus_event`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `alert_definition_id` | `alert_definition_id UUID NOT NULL REFERENCES alert_definitions(id) ON DELETE RESTRICT` |
| `alert_code` | `alert_code TEXT NOT NULL` |
| `severity` | `severity TEXT NOT NULL CHECK (severity IN ('info', 'warning', 'critical'))` |
| `event_id` | `event_id UUID NOT NULL REFERENCES event_bus_event(id) ON DELETE RESTRICT` |
| `event_type` | `event_type TEXT NOT NULL` |
| `resource_type` | `resource_type TEXT NOT NULL` |
| `resource_id` | `resource_id TEXT NOT NULL` |
| `resource_path` | `resource_path TEXT` |
| `warehouse_id` | `warehouse_id UUID` |
| `event_payload` | `event_payload JSONB NOT NULL` |
| `recipients` | `recipients TEXT[] NOT NULL DEFAULT '{}'` |
| `status` | `status TEXT NOT NULL CHECK (status IN ( 'triggered', 'notified', 'acknowledged', 'handling', 'closed', 'ignored', 'timed_out', 'escalated', 'notification_failed' ))` |
| `dedup_key` | `dedup_key TEXT NOT NULL` |
| `escalation_level` | `escalation_level INT NOT NULL DEFAULT 0 CHECK (escalation_level BETWEEN 0 AND 3)` |
| `action_description` | `action_description TEXT` |
| `ignored_reason` | `ignored_reason TEXT` |
| `close_reason` | `close_reason TEXT` |
| `triggered_at` | `triggered_at TIMESTAMPTZ NOT NULL` |
| `notified_at` | `notified_at TIMESTAMPTZ` |
| `acknowledged_at` | `acknowledged_at TIMESTAMPTZ` |
| `handled_at` | `handled_at TIMESTAMPTZ` |
| `closed_at` | `closed_at TIMESTAMPTZ` |
| `last_escalated_at` | `last_escalated_at TIMESTAMPTZ` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL` |

### `alert_lifecycle_events`

- 模块：hal_alert_runtime
- 迁移：`backend/migrations/202607150012_hal_alert_runtime.sql`
- 货主字段：有
- 索引：`alert_lifecycle_events_instance_idx`
- ALTER 迁移：无
- 引用表：`alert_instances`, `auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `event_sequence` | `event_sequence BIGINT GENERATED ALWAYS AS IDENTITY UNIQUE` |
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `alert_instance_id` | `alert_instance_id UUID NOT NULL REFERENCES alert_instances(id) ON DELETE RESTRICT` |
| `from_status` | `from_status TEXT` |
| `to_status` | `to_status TEXT NOT NULL` |
| `action_description` | `action_description TEXT` |
| `actor_id` | `actor_id UUID` |
| `actor_name` | `actor_name TEXT NOT NULL` |
| `occurred_at` | `occurred_at TIMESTAMPTZ NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL` |

### `alert_escalation_rules`

- 模块：hal_alert_escalation
- 迁移：`backend/migrations/202607150013_hal_alert_escalation.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `rule_code` | `rule_code TEXT NOT NULL` |
| `rule_name` | `rule_name TEXT NOT NULL` |
| `notify_lower_levels` | `notify_lower_levels BOOLEAN NOT NULL DEFAULT TRUE` |
| `off_hours_start` | `off_hours_start TIME NOT NULL DEFAULT '18:00'` |
| `off_hours_end` | `off_hours_end TIME NOT NULL DEFAULT '08:00'` |
| `off_hours_handler_roles` | `off_hours_handler_roles TEXT[] NOT NULL DEFAULT '{}'` |
| `holiday_dates` | `holiday_dates DATE[] NOT NULL DEFAULT '{}'` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `version` | `version BIGINT NOT NULL DEFAULT 1 CHECK (version > 0)` |
| `created_by` | `created_by UUID` |
| `updated_by` | `updated_by UUID` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL` |

### `alert_escalation_levels`

- 模块：hal_alert_escalation
- 迁移：`backend/migrations/202607150013_hal_alert_escalation.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`alert_escalation_rules`, `auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `rule_id` | `rule_id UUID NOT NULL REFERENCES alert_escalation_rules(id) ON DELETE CASCADE` |
| `level_no` | `level_no INT NOT NULL CHECK (level_no BETWEEN 1 AND 3)` |
| `threshold_seconds` | `threshold_seconds BIGINT NOT NULL CHECK (threshold_seconds > 0)` |
| `recipient_roles` | `recipient_roles TEXT[] NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL` |

### `alert_escalation_events`

- 模块：hal_alert_escalation
- 迁移：`backend/migrations/202607150013_hal_alert_escalation.sql`
- 货主字段：有
- 索引：`alert_escalation_events_alert_idx`
- ALTER 迁移：无
- 引用表：`alert_instances`, `auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `alert_instance_id` | `alert_instance_id UUID NOT NULL REFERENCES alert_instances(id) ON DELETE RESTRICT` |
| `level_no` | `level_no INT NOT NULL CHECK (level_no BETWEEN 1 AND 3)` |
| `repeat_key` | `repeat_key TEXT NOT NULL` |
| `recipients` | `recipients TEXT[] NOT NULL` |
| `elapsed_seconds` | `elapsed_seconds BIGINT NOT NULL CHECK (elapsed_seconds >= 0)` |
| `reason` | `reason TEXT NOT NULL` |
| `occurred_at` | `occurred_at TIMESTAMPTZ NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL` |

### `auth_user_warehouse_scopes`

- 模块：hal_alert_dashboard
- 迁移：`backend/migrations/202607150014_hal_alert_dashboard.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`, `auth_user_owner_bindings`, `auth_users`, `warehouses`

| 字段 | SQL 定义 |
|---|---|
| `user_id` | `user_id UUID NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `warehouse_id` | `warehouse_id UUID NOT NULL REFERENCES warehouses(id) ON DELETE CASCADE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `alert_report_exports`

- 模块：hal_alert_dashboard
- 迁移：`backend/migrations/202607150014_hal_alert_dashboard.sql`
- 货主字段：有
- 索引：`alert_report_exports_owner_status_idx`
- ALTER 迁移：无
- 引用表：`auth_owners`, `auth_users`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `requested_by` | `requested_by UUID NOT NULL REFERENCES auth_users(id) ON DELETE RESTRICT` |
| `format` | `format TEXT NOT NULL CHECK (format IN ('excel', 'pdf'))` |
| `status` | `status TEXT NOT NULL CHECK (status IN ('queued', 'processing', 'ready', 'failed'))` |
| `filters` | `filters JSONB NOT NULL` |
| `row_count` | `row_count BIGINT NOT NULL CHECK (row_count >= 0)` |
| `content` | `content BYTEA` |
| `content_type` | `content_type TEXT` |
| `filename` | `filename TEXT` |
| `download_token` | `download_token UUID NOT NULL UNIQUE` |
| `recipient_email` | `recipient_email TEXT` |
| `email_notification_status` | `email_notification_status TEXT` |
| `error_message` | `error_message TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL` |
| `completed_at` | `completed_at TIMESTAMPTZ` |
| `expires_at` | `expires_at TIMESTAMPTZ NOT NULL` |

### `alert_statistics_snapshots`

- 模块：hal_alert_dashboard
- 迁移：`backend/migrations/202607150014_hal_alert_dashboard.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE` |
| `filter_key` | `filter_key TEXT NOT NULL` |
| `filters` | `filters JSONB NOT NULL` |
| `payload` | `payload JSONB NOT NULL` |
| `generated_at` | `generated_at TIMESTAMPTZ NOT NULL` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL` |

### `inventory_relocations`

- 模块：m3_remaining_closeout
- 迁移：`backend/migrations/202607170004_m3_remaining_closeout.sql`
- 货主字段：有
- 索引：`inventory_relocations_owner_created_idx`
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `batch_id` | `batch_id UUID NOT NULL` |
| `product_code` | `product_code TEXT NOT NULL` |
| `batch_no` | `batch_no TEXT NOT NULL` |
| `qty` | `qty BIGINT NOT NULL CHECK (qty > 0)` |
| `from_location_id` | `from_location_id UUID NOT NULL` |
| `from_location_code` | `from_location_code TEXT NOT NULL` |
| `to_location_id` | `to_location_id UUID NOT NULL` |
| `to_location_code` | `to_location_code TEXT NOT NULL` |
| `relocation_mode` | `relocation_mode TEXT NOT NULL DEFAULT 'direct' CHECK (relocation_mode IN ('direct', 'lpn_full', 'partial', 'piece'))` |
| `lpn_code` | `lpn_code TEXT` |
| `quality_status` | `quality_status TEXT NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'completed' CHECK (status IN ('completed', 'pending_supervisor', 'failed'))` |
| `reason` | `reason TEXT` |
| `created_by` | `created_by UUID NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `inventory_status_erp_feedback_outbox`

- 模块：m3_remaining_closeout
- 迁移：`backend/migrations/202607170004_m3_remaining_closeout.sql`
- 货主字段：有
- 索引：`inventory_status_erp_feedback_pending_idx`
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `batch_id` | `batch_id UUID NOT NULL` |
| `status_change_id` | `status_change_id UUID` |
| `event_type` | `event_type TEXT NOT NULL DEFAULT 'inventory_status_changed'` |
| `payload` | `payload JSONB NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'sending', 'succeeded', 'failed'))` |
| `attempt_count` | `attempt_count INT NOT NULL DEFAULT 0 CHECK (attempt_count >= 0)` |
| `next_attempt_at` | `next_attempt_at TIMESTAMPTZ NOT NULL` |
| `last_error` | `last_error TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `inventory_alert_events`

- 模块：m3_remaining_closeout
- 迁移：`backend/migrations/202607170004_m3_remaining_closeout.sql`
- 货主字段：有
- 索引：`inventory_alert_events_owner_status_idx`, `inventory_alert_events_owner_type_idx`
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `alert_type` | `alert_type TEXT NOT NULL CHECK (alert_type IN ( 'near_expiry', 'expired', 'safety_stock', 'overstock', 'maintenance_overdue', 'temperature' ))` |
| `product_code` | `product_code TEXT` |
| `batch_id` | `batch_id UUID` |
| `batch_no` | `batch_no TEXT` |
| `location_code` | `location_code TEXT` |
| `severity` | `severity TEXT NOT NULL DEFAULT 'medium' CHECK (severity IN ('low', 'medium', 'high', 'critical'))` |
| `title` | `title TEXT NOT NULL` |
| `message` | `message TEXT NOT NULL` |
| `lifecycle_status` | `lifecycle_status TEXT NOT NULL DEFAULT 'open' CHECK (lifecycle_status IN ('open', 'handled', 'ignored'))` |
| `handled_by` | `handled_by UUID` |
| `handled_at` | `handled_at TIMESTAMPTZ` |
| `payload` | `payload JSONB NOT NULL DEFAULT '{}'::jsonb` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `inventory_abc_classifications`

- 模块：m3_remaining_closeout
- 迁移：`backend/migrations/202607170004_m3_remaining_closeout.sql`
- 货主字段：有
- 索引：`inventory_abc_owner_class_idx`
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `product_code` | `product_code TEXT NOT NULL` |
| `abc_class` | `abc_class TEXT NOT NULL CHECK (abc_class IN ('A', 'B', 'C'))` |
| `score` | `score NUMERIC(18, 4) NOT NULL DEFAULT 0` |
| `outbound_qty` | `outbound_qty BIGINT NOT NULL DEFAULT 0` |
| `period_start` | `period_start DATE NOT NULL` |
| `period_end` | `period_end DATE NOT NULL` |
| `source` | `source TEXT NOT NULL DEFAULT 'system' CHECK (source IN ('system', 'manual'))` |
| `override_reason` | `override_reason TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `putaway_strategy_profiles`

- 模块：m2_inbound_closeout
- 迁移：`backend/migrations/202607170006_m2_inbound_closeout.sql`
- 货主字段：有
- 索引：`UNIQUE putaway_strategy_profiles_one_default_idx`, `putaway_strategy_profiles_owner_status_idx`
- ALTER 迁移：`backend/migrations/202607180001_m2_putaway_strategy_config.sql`
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `profile_code` | `profile_code TEXT NOT NULL` |
| `profile_name` | `profile_name TEXT NOT NULL` |
| `is_default` | `is_default BOOLEAN NOT NULL DEFAULT FALSE` |
| `top_n` | `top_n INT NOT NULL DEFAULT 3 CHECK (top_n > 0 AND top_n <= 50)` |
| `enabled_rules` | `enabled_rules JSONB NOT NULL DEFAULT '{ "temperature_match": true, "owner_isolation": true, "capacity_match": true, "same_product_cluster": true, "quality_color_match": true }'::jsonb` |
| `rule_priority` | `rule_priority JSONB NOT NULL DEFAULT '[ "temperature_match", "owner_isolation", "capacity_match", "quality_color_match", "same_product_cluster" ]'::jsonb` |
| `status` | `status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'disabled'))` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `receiving_putaway_erp_feedback_outbox`

- 模块：m2_putaway_lpn_erp
- 迁移：`backend/migrations/202607180003_m2_putaway_lpn_erp.sql`
- 货主字段：有
- 索引：`receiving_putaway_erp_outbox_owner_status_idx`
- ALTER 迁移：无
- 引用表：`receiving_putaways`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `putaway_id` | `putaway_id UUID NOT NULL REFERENCES receiving_putaways(id) ON DELETE CASCADE` |
| `receiving_order_id` | `receiving_order_id UUID NOT NULL` |
| `batch_id` | `batch_id UUID` |
| `event_type` | `event_type TEXT NOT NULL DEFAULT 'inbound_putaway_completed'` |
| `payload` | `payload JSONB NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'failed', 'succeeded'))` |
| `attempt_count` | `attempt_count INT NOT NULL DEFAULT 0 CHECK (attempt_count >= 0)` |
| `next_attempt_at` | `next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `last_error` | `last_error TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `archive_revision_erp_feedback_outbox`

- 模块：h8_erp_outbox_extensions
- 迁移：`backend/migrations/202607180005_h8_erp_outbox_extensions.sql`
- 货主字段：有
- 索引：`archive_revision_erp_outbox_poll_idx`
- ALTER 迁移：无
- 引用表：`auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `liaison_id` | `liaison_id UUID` |
| `asn_id` | `asn_id UUID` |
| `receipt_record_id` | `receipt_record_id UUID` |
| `product_code` | `product_code TEXT NOT NULL` |
| `field_name` | `field_name TEXT NOT NULL` |
| `event_type` | `event_type TEXT NOT NULL DEFAULT 'archive_revision' CHECK (event_type = 'archive_revision')` |
| `payload` | `payload JSONB NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'failed', 'succeeded', 'dead'))` |
| `attempt_count` | `attempt_count INT NOT NULL DEFAULT 0 CHECK (attempt_count BETWEEN 0 AND 5)` |
| `max_attempts` | `max_attempts INT NOT NULL DEFAULT 5 CHECK (max_attempts = 5)` |
| `next_attempt_at` | `next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `deadline_at` | `deadline_at TIMESTAMPTZ NOT NULL DEFAULT (now() + interval '24 hours')` |
| `last_error` | `last_error TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `reconciliation_erp_feedback_outbox`

- 模块：h8_erp_outbox_extensions
- 迁移：`backend/migrations/202607180005_h8_erp_outbox_extensions.sql`
- 货主字段：有
- 索引：`reconciliation_erp_outbox_poll_idx`
- ALTER 迁移：无
- 引用表：`auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `recon_doc_no` | `recon_doc_no TEXT` |
| `event_type` | `event_type TEXT NOT NULL DEFAULT 'reconciliation_diff' CHECK (event_type = 'reconciliation_diff')` |
| `payload` | `payload JSONB NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'failed', 'succeeded', 'dead'))` |
| `attempt_count` | `attempt_count INT NOT NULL DEFAULT 0 CHECK (attempt_count BETWEEN 0 AND 5)` |
| `max_attempts` | `max_attempts INT NOT NULL DEFAULT 5 CHECK (max_attempts = 5)` |
| `next_attempt_at` | `next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `last_error` | `last_error TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `shipment_confirm_erp_feedback_outbox`

- 模块：h8_erp_outbox_extensions
- 迁移：`backend/migrations/202607180005_h8_erp_outbox_extensions.sql`
- 货主字段：有
- 索引：`shipment_confirm_erp_outbox_poll_idx`
- ALTER 迁移：无
- 引用表：`auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `shipment_id` | `shipment_id UUID` |
| `outbound_order_id` | `outbound_order_id UUID` |
| `event_type` | `event_type TEXT NOT NULL DEFAULT 'shipment_confirm' CHECK (event_type = 'shipment_confirm')` |
| `payload` | `payload JSONB NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'failed', 'succeeded'))` |
| `attempt_count` | `attempt_count INT NOT NULL DEFAULT 0 CHECK (attempt_count >= 0)` |
| `next_attempt_at` | `next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `last_error` | `last_error TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `inventory_snapshot_erp_feedback_outbox`

- 模块：h8_erp_outbox_extensions
- 迁移：`backend/migrations/202607180005_h8_erp_outbox_extensions.sql`
- 货主字段：有
- 索引：`inventory_snapshot_erp_outbox_poll_idx`
- ALTER 迁移：无
- 引用表：`auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `snapshot_no` | `snapshot_no TEXT` |
| `event_type` | `event_type TEXT NOT NULL DEFAULT 'inventory_snapshot' CHECK (event_type = 'inventory_snapshot')` |
| `payload` | `payload JSONB NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'failed', 'succeeded'))` |
| `attempt_count` | `attempt_count INT NOT NULL DEFAULT 0 CHECK (attempt_count >= 0)` |
| `next_attempt_at` | `next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `last_error` | `last_error TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h8_erp_connectors`

- 模块：h8_erp_connectors
- 迁移：`backend/migrations/202607190002_h8_erp_connectors.sql`
- 货主字段：有
- 索引：`h8_erp_connectors_owner_status_idx`
- ALTER 迁移：`backend/migrations/202607220002_h8_worker_runtime_and_payload_retention.sql`
- 引用表：`auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `connector_code` | `connector_code TEXT NOT NULL` |
| `connector_name` | `connector_name TEXT NOT NULL` |
| `warehouse_ids` | `warehouse_ids UUID[] NOT NULL DEFAULT '{}'` |
| `directions` | `directions TEXT[] NOT NULL` |
| `message_types` | `message_types TEXT[] NOT NULL` |
| `channel_mode` | `channel_mode TEXT NOT NULL CHECK (channel_mode IN ('rest', 'interface_table', 'rest_primary_table_fallback'))` |
| `api_base_url` | `api_base_url TEXT` |
| `interface_db_host` | `interface_db_host TEXT` |
| `interface_db_port` | `interface_db_port INT` |
| `interface_db_name` | `interface_db_name TEXT` |
| `interface_db_username` | `interface_db_username TEXT` |
| `interface_probe_db_username` | `interface_probe_db_username TEXT` |
| `api_key_id` | `api_key_id UUID` |
| `bearer_secret_alias` | `bearer_secret_alias TEXT` |
| `interface_db_password_alias` | `interface_db_password_alias TEXT` |
| `interface_probe_db_password_alias` | `interface_probe_db_password_alias TEXT` |
| `interface_probe_config_version` | `interface_probe_config_version BIGINT NOT NULL DEFAULT 1 CHECK (interface_probe_config_version >= 1)` |
| `status` | `status TEXT NOT NULL DEFAULT 'testing' CHECK (status IN ('testing', 'active', 'disabled'))` |
| `config_version` | `config_version BIGINT NOT NULL DEFAULT 1 CHECK (config_version >= 1)` |
| `first_activated_at` | `first_activated_at TIMESTAMPTZ` |
| `last_tested_version` | `last_tested_version BIGINT` |
| `last_tested_at` | `last_tested_at TIMESTAMPTZ` |
| `last_tested_succeeded` | `last_tested_succeeded BOOLEAN` |
| `last_tested_error_summary` | `last_tested_error_summary TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h8_erp_in_flight_messages`

- 模块：h8_erp_connectors
- 迁移：`backend/migrations/202607190002_h8_erp_connectors.sql`
- 货主字段：有
- 索引：`h8_erp_inflight_connector_status_idx`
- ALTER 迁移：无
- 引用表：`auth_owners`, `h8_erp_connectors`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `connector_id` | `connector_id UUID NOT NULL REFERENCES h8_erp_connectors(id) ON DELETE RESTRICT` |
| `idempotency_key` | `idempotency_key TEXT NOT NULL` |
| `direction` | `direction TEXT NOT NULL CHECK (direction IN ('inbound', 'outbound'))` |
| `message_type` | `message_type TEXT NOT NULL` |
| `channel_stage` | `channel_stage TEXT NOT NULL DEFAULT 'rest' CHECK (channel_stage IN ('rest', 'interface_table'))` |
| `status` | `status TEXT NOT NULL DEFAULT 'paused' CHECK (status IN ('paused', 'running', 'succeeded', 'failed', 'dead'))` |
| `payload_ref` | `payload_ref TEXT` |
| `last_error` | `last_error TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h8_erp_message_registry`

- 模块：h8_erp_messages
- 迁移：`backend/migrations/202607190005_h8_erp_messages.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `message_type` | `message_type TEXT NOT NULL` |
| `external_ref` | `external_ref TEXT NOT NULL` |
| `idempotency_key` | `idempotency_key TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL` |

### `h8_erp_messages`

- 模块：h8_erp_messages
- 迁移：`backend/migrations/202607190005_h8_erp_messages.sql`
- 货主字段：有
- 索引：`h8_erp_messages_correlation_idx`, `h8_erp_messages_owner_created_idx`, `h8_erp_messages_owner_status_idx`, `h8_erp_messages_owner_type_idx`
- ALTER 迁移：`backend/migrations/202607220001_h8_erp_message_receipts.sql`, `backend/migrations/202607220002_h8_worker_runtime_and_payload_retention.sql`
- 引用表：`auth_owners`, `h8_erp_connectors`, `h8_erp_message_registry`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID NOT NULL` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `warehouse_id` | `warehouse_id UUID` |
| `connector_id` | `connector_id UUID REFERENCES h8_erp_connectors(id) ON DELETE RESTRICT` |
| `connector_code` | `connector_code TEXT` |
| `config_version` | `config_version BIGINT` |
| `direction` | `direction TEXT NOT NULL CHECK (direction IN ('inbound', 'outbound'))` |
| `message_type` | `message_type TEXT NOT NULL` |
| `schema_version` | `schema_version TEXT NOT NULL DEFAULT '1'` |
| `channel` | `channel TEXT NOT NULL CHECK (channel IN ('rest', 'interface_table'))` |
| `external_ref` | `external_ref TEXT NOT NULL` |
| `wms_resource_id` | `wms_resource_id TEXT` |
| `idempotency_key` | `idempotency_key TEXT NOT NULL` |
| `correlation_id` | `correlation_id TEXT NOT NULL` |
| `sync_status` | `sync_status TEXT NOT NULL DEFAULT 'pending' CHECK (sync_status IN ('pending', 'processing', 'succeeded', 'failed', 'dead', 'acked'))` |
| `retry_count` | `retry_count INT NOT NULL DEFAULT 0 CHECK (retry_count >= 0)` |
| `next_retry_at` | `next_retry_at TIMESTAMPTZ` |
| `last_error_summary` | `last_error_summary TEXT` |
| `payload_digest` | `payload_digest TEXT NOT NULL` |
| `claimed_by` | `claimed_by TEXT` |
| `lease_expires_at` | `lease_expires_at TIMESTAMPTZ` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `completed_at` | `completed_at TIMESTAMPTZ` |
| `acked_at` | `acked_at TIMESTAMPTZ` |

### `h8_erp_message_stats_daily`

- 模块：h8_erp_messages
- 迁移：`backend/migrations/202607190005_h8_erp_messages.sql`
- 货主字段：无
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `stat_date` | `stat_date DATE NOT NULL` |
| `connector_code` | `connector_code TEXT NOT NULL` |
| `channel` | `channel TEXT NOT NULL` |
| `message_type` | `message_type TEXT NOT NULL` |
| `warehouse_id` | `warehouse_id TEXT NOT NULL` |
| `total` | `total BIGINT NOT NULL DEFAULT 0 CHECK (total >= 0)` |
| `succeeded` | `succeeded BIGINT NOT NULL DEFAULT 0 CHECK (succeeded >= 0)` |
| `failed` | `failed BIGINT NOT NULL DEFAULT 0 CHECK (failed >= 0)` |
| `dead` | `dead BIGINT NOT NULL DEFAULT 0 CHECK (dead >= 0)` |
| `processing` | `processing BIGINT NOT NULL DEFAULT 0 CHECK (processing >= 0)` |
| `pending` | `pending BIGINT NOT NULL DEFAULT 0 CHECK (pending >= 0)` |
| `retry_total` | `retry_total BIGINT NOT NULL DEFAULT 0 CHECK (retry_total >= 0)` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h8_erp_message_attempt_registry`

- 模块：h8_erp_messages
- 迁移：`backend/migrations/202607190005_h8_erp_messages.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`, `h8_erp_message_registry`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `message_id` | `message_id UUID NOT NULL` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `attempt_no` | `attempt_no INT NOT NULL CHECK (attempt_no >= 1)` |
| `started_at` | `started_at TIMESTAMPTZ NOT NULL` |

### `h8_erp_message_attempts`

- 模块：h8_erp_messages
- 迁移：`backend/migrations/202607190005_h8_erp_messages.sql`
- 货主字段：有
- 索引：`h8_erp_message_attempts_msg_idx`
- ALTER 迁移：`backend/migrations/202607190008_h8_erp_message_attempts_archived.sql`
- 引用表：`auth_owners`, `h8_erp_message_attempt_registry`, `h8_erp_message_registry`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID NOT NULL` |
| `message_id` | `message_id UUID NOT NULL` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `attempt_no` | `attempt_no INT NOT NULL CHECK (attempt_no >= 1)` |
| `channel` | `channel TEXT NOT NULL CHECK (channel IN ('rest', 'interface_table'))` |
| `started_at` | `started_at TIMESTAMPTZ NOT NULL` |
| `finished_at` | `finished_at TIMESTAMPTZ` |
| `result` | `result TEXT NOT NULL CHECK (result IN ('succeeded', 'failed', 'dead', 'replayed', 'claimed'))` |
| `error_summary` | `error_summary TEXT` |
| `actor` | `actor TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h8_erp_message_retention_policy`

- 模块：h8_erp_messages
- 迁移：`backend/migrations/202607190005_h8_erp_messages.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `owner_id` | `owner_id UUID PRIMARY KEY REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `retention_days` | `retention_days INT NOT NULL CHECK (retention_days > 0)` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h8_erp_worker_heartbeats`

- 模块：h8_worker_runtime_and_payload_retention
- 迁移：`backend/migrations/202607220002_h8_worker_runtime_and_payload_retention.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`, `h8_erp_connectors`

| 字段 | SQL 定义 |
|---|---|
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `worker_id` | `worker_id TEXT NOT NULL CHECK (btrim(worker_id) <> '' AND length(worker_id) <= 128)` |
| `worker_version` | `worker_version TEXT NOT NULL CHECK (btrim(worker_version) <> '' AND length(worker_version) <= 64)` |
| `connector_id` | `connector_id UUID NOT NULL` |
| `directions` | `directions TEXT[] NOT NULL` |
| `current_claims` | `current_claims INT NOT NULL CHECK (current_claims >= 0)` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL` |
| `last_heartbeat_at` | `last_heartbeat_at TIMESTAMPTZ NOT NULL` |
| `heartbeat_expires_at` | `heartbeat_expires_at TIMESTAMPTZ NOT NULL` |

### `h8_erp_worker_claim_controls`

- 模块：h8_worker_runtime_and_payload_retention
- 迁移：`backend/migrations/202607220002_h8_worker_runtime_and_payload_retention.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`, `h8_erp_connectors`

| 字段 | SQL 定义 |
|---|---|
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `connector_id` | `connector_id UUID NOT NULL` |
| `direction` | `direction TEXT NOT NULL CHECK (direction IN ('inbound', 'outbound'))` |
| `paused` | `paused BOOLEAN NOT NULL` |
| `reason` | `reason TEXT NOT NULL CHECK (btrim(reason) <> '' AND length(reason) <= 500)` |
| `paused_until` | `paused_until TIMESTAMPTZ` |
| `updated_by` | `updated_by TEXT NOT NULL` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL` |

### `h8_erp_payload_retention_policies`

- 模块：h8_worker_runtime_and_payload_retention
- 迁移：`backend/migrations/202607220002_h8_worker_runtime_and_payload_retention.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`, `h8_erp_connectors`

| 字段 | SQL 定义 |
|---|---|
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `connector_id` | `connector_id UUID NOT NULL` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT FALSE` |
| `retention_days` | `retention_days INT NOT NULL DEFAULT 7 CHECK (retention_days BETWEEN 1 AND 30)` |
| `updated_by` | `updated_by TEXT NOT NULL` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL` |

### `h8_erp_connector_versions`

- 模块：h8_erp_connector_versions
- 迁移：`backend/migrations/202607230001_h8_erp_connector_versions.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`h8_erp_connectors`

| 字段 | SQL 定义 |
|---|---|
| `owner_id` | `owner_id UUID NOT NULL` |
| `connector_id` | `connector_id UUID NOT NULL` |
| `config_version` | `config_version BIGINT NOT NULL CHECK (config_version >= 1)` |
| `runtime_config` | `runtime_config JSONB NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `parameter_mapping_dictionaries`

- 模块：mpm_persistent_mapping
- 迁移：`backend/migrations/202607230002_mpm_persistent_mapping.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID REFERENCES auth_owners(id)` |
| `dict_code` | `dict_code TEXT NOT NULL` |
| `dict_name` | `dict_name TEXT NOT NULL` |
| `target_values` | `target_values JSONB NOT NULL` |
| `case_sensitive` | `case_sensitive BOOLEAN NOT NULL DEFAULT FALSE` |
| `normalize_whitespace` | `normalize_whitespace BOOLEAN NOT NULL DEFAULT TRUE` |
| `default_strategy` | `default_strategy TEXT NOT NULL DEFAULT 'mark_unmapped'` |
| `fallback_value` | `fallback_value TEXT` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `parameter_mapping_rules`

- 模块：mpm_persistent_mapping
- 迁移：`backend/migrations/202607230002_mpm_persistent_mapping.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`, `parameter_mapping_dictionaries`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `dictionary_id` | `dictionary_id UUID NOT NULL REFERENCES parameter_mapping_dictionaries(id)` |
| `owner_id` | `owner_id UUID REFERENCES auth_owners(id)` |
| `source_system` | `source_system TEXT NOT NULL` |
| `match_type` | `match_type TEXT NOT NULL` |
| `source_pattern` | `source_pattern TEXT NOT NULL` |
| `normalized_source_pattern` | `normalized_source_pattern TEXT NOT NULL` |
| `target_value` | `target_value TEXT NOT NULL` |
| `priority` | `priority INT NOT NULL DEFAULT 100` |
| `confidence` | `confidence INT NOT NULL DEFAULT 100` |
| `effective_from` | `effective_from TIMESTAMPTZ` |
| `effective_to` | `effective_to TIMESTAMPTZ` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `parameter_mapping_queue`

- 模块：mpm_persistent_mapping
- 迁移：`backend/migrations/202607230002_mpm_persistent_mapping.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`parameter_mapping_dictionaries`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `dictionary_id` | `dictionary_id UUID NOT NULL REFERENCES parameter_mapping_dictionaries(id)` |
| `source_system` | `source_system TEXT NOT NULL` |
| `source_record_id` | `source_record_id TEXT` |
| `source_value` | `source_value TEXT NOT NULL` |
| `normalized_source_value` | `normalized_source_value TEXT NOT NULL` |
| `occurrence_count` | `occurrence_count BIGINT NOT NULL DEFAULT 1` |
| `status` | `status TEXT NOT NULL DEFAULT 'pending_mapping'` |
| `first_seen_at` | `first_seen_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `last_seen_at` | `last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `handled_by` | `handled_by UUID` |

### `product_packaging_levels`

- 模块：m1_complete_product_contract
- 迁移：`backend/migrations/202607230003_m1_complete_product_contract.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`products`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `product_id` | `product_id UUID NOT NULL` |
| `unit_code` | `unit_code TEXT NOT NULL` |
| `unit_name` | `unit_name TEXT NOT NULL` |
| `ratio_to_base` | `ratio_to_base BIGINT NOT NULL CHECK (ratio_to_base > 0)` |
| `is_base` | `is_base BOOLEAN NOT NULL DEFAULT FALSE` |
| `is_default` | `is_default BOOLEAN NOT NULL DEFAULT FALSE` |
| `sort_order` | `sort_order INT NOT NULL CHECK (sort_order >= 0)` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `product_mapping_traces`

- 模块：m1_complete_product_contract
- 迁移：`backend/migrations/202607230003_m1_complete_product_contract.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`parameter_mapping_rules`, `products`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `product_id` | `product_id UUID NOT NULL` |
| `field_name` | `field_name TEXT NOT NULL` |
| `rule_id` | `rule_id UUID` |
| `source_system` | `source_system TEXT NOT NULL` |
| `source_value` | `source_value TEXT NOT NULL` |
| `target_value` | `target_value TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `reconciliation_rules`

- 模块：mrc_reconciliation
- 迁移：`backend/migrations/202607230004_mrc_reconciliation.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `owner_id` | `owner_id UUID PRIMARY KEY REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `interval_hours` | `interval_hours INT NOT NULL DEFAULT 24 CHECK (interval_hours BETWEEN 1 AND 168)` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `updated_by` | `updated_by UUID NOT NULL` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `reconciliation_runs`

- 模块：mrc_reconciliation
- 迁移：`backend/migrations/202607230004_mrc_reconciliation.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `window_key` | `window_key TEXT NOT NULL` |
| `request_hash` | `request_hash TEXT NOT NULL` |
| `snapshot_at` | `snapshot_at TIMESTAMPTZ NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'completed' CHECK (status IN ('completed', 'failed'))` |
| `matched_count` | `matched_count INT NOT NULL DEFAULT 0` |
| `wms_more_count` | `wms_more_count INT NOT NULL DEFAULT 0` |
| `erp_more_count` | `erp_more_count INT NOT NULL DEFAULT 0` |
| `created_by` | `created_by UUID NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `reconciliation_schedule_claims`

- 模块：mrc_reconciliation
- 迁移：`backend/migrations/202607230004_mrc_reconciliation.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`, `reconciliation_runs`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `window_key` | `window_key TEXT NOT NULL` |
| `claim_token` | `claim_token UUID NOT NULL UNIQUE` |
| `worker_id` | `worker_id TEXT NOT NULL CHECK (length(btrim(worker_id)) BETWEEN 1 AND 128)` |
| `attempt_no` | `attempt_no INT NOT NULL CHECK (attempt_no > 0)` |
| `status` | `status TEXT NOT NULL CHECK (status IN ('active', 'completed', 'failed', 'expired'))` |
| `lease_expires_at` | `lease_expires_at TIMESTAMPTZ NOT NULL` |
| `run_id` | `run_id UUID REFERENCES reconciliation_runs(id) ON DELETE RESTRICT` |
| `failure_stage` | `failure_stage TEXT CHECK (failure_stage IN ('pull', 'submit', 'lease'))` |
| `failure_code` | `failure_code TEXT CHECK ( failure_code IS NULL OR failure_code IN ('erp_pull_failed', 'snapshot_submit_failed', 'lease_expired') )` |
| `claimed_at` | `claimed_at TIMESTAMPTZ NOT NULL` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL` |
| `completed_at` | `completed_at TIMESTAMPTZ` |
| `failed_at` | `failed_at TIMESTAMPTZ` |

### `reconciliation_items`

- 模块：mrc_reconciliation
- 迁移：`backend/migrations/202607230004_mrc_reconciliation.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`, `reconciliation_runs`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `run_id` | `run_id UUID NOT NULL REFERENCES reconciliation_runs(id) ON DELETE RESTRICT` |
| `product_code` | `product_code TEXT NOT NULL` |
| `batch_no` | `batch_no TEXT NOT NULL` |
| `wms_qty` | `wms_qty BIGINT NOT NULL CHECK (wms_qty >= 0)` |
| `erp_qty` | `erp_qty BIGINT NOT NULL CHECK (erp_qty >= 0)` |
| `difference_qty` | `difference_qty BIGINT NOT NULL` |
| `difference_type` | `difference_type TEXT NOT NULL CHECK (difference_type IN ('matched', 'wms_more', 'erp_more'))` |
| `resolution_status` | `resolution_status TEXT NOT NULL CHECK (resolution_status IN ( 'matched', 'open', 'adjustment_pending', 'erp_feedback_pending', 'exception', 'resolved', 'known_difference' ))` |
| `disposition` | `disposition TEXT CHECK (disposition IN ('wms_truth', 'erp_truth', 'known_difference'))` |
| `resolved_by` | `resolved_by UUID` |
| `resolved_at` | `resolved_at TIMESTAMPTZ` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `reconciliation_item_adjustments`

- 模块：mrc_reconciliation
- 迁移：`backend/migrations/202607230004_mrc_reconciliation.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`, `inventory_batches`, `reconciliation_items`, `stock_adjustment_orders`

| 字段 | SQL 定义 |
|---|---|
| `item_id` | `item_id UUID NOT NULL REFERENCES reconciliation_items(id) ON DELETE RESTRICT` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `inventory_batch_id` | `inventory_batch_id UUID NOT NULL REFERENCES inventory_batches(id) ON DELETE RESTRICT` |
| `quantity` | `quantity BIGINT NOT NULL CHECK (quantity > 0)` |
| `adjustment_order_id` | `adjustment_order_id UUID NOT NULL REFERENCES stock_adjustment_orders(id) ON DELETE RESTRICT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL` |

### `reconciliation_item_locks`

- 模块：mrc_reconciliation
- 迁移：`backend/migrations/202607230004_mrc_reconciliation.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`, `inventory_batches`, `reconciliation_items`

| 字段 | SQL 定义 |
|---|---|
| `item_id` | `item_id UUID NOT NULL REFERENCES reconciliation_items(id) ON DELETE RESTRICT` |
| `inventory_batch_id` | `inventory_batch_id UUID NOT NULL REFERENCES inventory_batches(id) ON DELETE RESTRICT` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT` |
| `previous_status` | `previous_status TEXT NOT NULL` |
| `locked_at` | `locked_at TIMESTAMPTZ NOT NULL` |
| `released_at` | `released_at TIMESTAMPTZ` |

### `h_file_upload_sessions`

- 模块：h_file_attachments
- 迁移：`backend/migrations/202607250001_h_file_attachments.sql`
- 货主字段：有
- 索引：`h_file_upload_sessions_expires_idx`, `h_file_upload_sessions_owner_entity_idx`
- ALTER 迁移：无
- 引用表：`auth_owners`, `auth_users`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id)` |
| `module` | `module TEXT NOT NULL CHECK (length(btrim(module)) BETWEEN 1 AND 32)` |
| `entity_type` | `entity_type TEXT NOT NULL CHECK (length(btrim(entity_type)) BETWEEN 1 AND 64)` |
| `entity_id` | `entity_id UUID NOT NULL` |
| `file_name` | `file_name TEXT NOT NULL CHECK (length(btrim(file_name)) BETWEEN 1 AND 255)` |
| `content_type` | `content_type TEXT NOT NULL CHECK (content_type IN ( 'image/jpeg', 'image/png', 'application/pdf', 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet', 'text/csv' ))` |
| `expected_size` | `expected_size BIGINT NOT NULL CHECK (expected_size BETWEEN 1 AND 52428800)` |
| `uploaded_size` | `uploaded_size BIGINT` |
| `storage_key` | `storage_key TEXT NOT NULL UNIQUE` |
| `token_hash` | `token_hash TEXT NOT NULL` |
| `sha256` | `sha256 TEXT` |
| `status` | `status TEXT NOT NULL CHECK (status IN ('created', 'uploaded', 'confirmed'))` |
| `uploaded_by` | `uploaded_by UUID NOT NULL REFERENCES auth_users(id)` |
| `expires_at` | `expires_at TIMESTAMPTZ NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `attachments`

- 模块：h_file_attachments
- 迁移：`backend/migrations/202607250001_h_file_attachments.sql`
- 货主字段：有
- 索引：`attachments_owner_entity_idx`
- ALTER 迁移：`backend/migrations/202607280001_h_file_h9_category_pdfs.sql`
- 引用表：`auth_owners`, `auth_users`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id)` |
| `module` | `module TEXT NOT NULL` |
| `entity_type` | `entity_type TEXT NOT NULL` |
| `entity_id` | `entity_id UUID NOT NULL` |
| `file_name` | `file_name TEXT NOT NULL` |
| `content_type` | `content_type TEXT NOT NULL` |
| `size_bytes` | `size_bytes BIGINT NOT NULL CHECK (size_bytes > 0)` |
| `storage_key` | `storage_key TEXT NOT NULL` |
| `sha256` | `sha256 TEXT NOT NULL` |
| `uploaded_by` | `uploaded_by UUID NOT NULL REFERENCES auth_users(id)` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h_file_download_sessions`

- 模块：h_file_attachments
- 迁移：`backend/migrations/202607250001_h_file_attachments.sql`
- 货主字段：有
- 索引：`h_file_download_sessions_expires_idx`
- ALTER 迁移：无
- 引用表：`attachments`, `auth_owners`, `auth_users`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id)` |
| `attachment_id` | `attachment_id UUID NOT NULL REFERENCES attachments(id)` |
| `token_hash` | `token_hash TEXT NOT NULL` |
| `created_by` | `created_by UUID NOT NULL REFERENCES auth_users(id)` |
| `expires_at` | `expires_at TIMESTAMPTZ NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `drug_inspection_reports`

- 模块：mdi_documents
- 迁移：`backend/migrations/202607250002_mdi_documents.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`, `auth_users`, `drug_inspection_report_versions`, `products`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id)` |
| `product_id` | `product_id UUID NOT NULL` |
| `batch_no` | `batch_no TEXT NOT NULL CHECK (length(btrim(batch_no)) BETWEEN 1 AND 128)` |
| `current_version_id` | `current_version_id UUID` |
| `created_by` | `created_by UUID NOT NULL REFERENCES auth_users(id)` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `drug_inspection_report_versions`

- 模块：mdi_documents
- 迁移：`backend/migrations/202607250002_mdi_documents.sql`
- 货主字段：有
- 索引：`UNIQUE drug_inspection_report_versions_one_current_idx`, `UNIQUE drug_inspection_report_versions_one_open_idx`
- ALTER 迁移：`backend/migrations/202607250003_mdi_customer_copy.sql`
- 引用表：`attachments`, `auth_owners`, `auth_users`, `drug_inspection_report_versions`, `drug_inspection_reports`, `drug_inspection_stamp_versions`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `report_id` | `report_id UUID NOT NULL REFERENCES drug_inspection_reports(id)` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id)` |
| `version_number` | `version_number INT NOT NULL CHECK (version_number > 0)` |
| `report_no` | `report_no TEXT NOT NULL CHECK (length(btrim(report_no)) BETWEEN 1 AND 128)` |
| `original_file_id` | `original_file_id UUID NOT NULL REFERENCES attachments(id)` |
| `original_file_hash` | `original_file_hash TEXT NOT NULL` |
| `source` | `source TEXT NOT NULL CHECK (source IN ('manual_upload', 'upstream_platform'))` |
| `processing_mode` | `processing_mode TEXT NOT NULL CHECK (processing_mode IN ( 'none', 'color_enhance', 'black_white_enhance' ))` |
| `qualified` | `qualified BOOLEAN NOT NULL` |
| `status` | `status TEXT NOT NULL CHECK (status IN ( 'draft', 'pending_confirmation', 'confirmed', 'superseded' ))` |
| `replaces_version_id` | `replaces_version_id UUID REFERENCES drug_inspection_report_versions(id)` |
| `modification_reason` | `modification_reason TEXT` |
| `uploaded_by` | `uploaded_by UUID NOT NULL REFERENCES auth_users(id)` |
| `submitted_at` | `submitted_at TIMESTAMPTZ` |
| `reviewed_by` | `reviewed_by UUID REFERENCES auth_users(id)` |
| `reviewed_at` | `reviewed_at TIMESTAMPTZ` |
| `review_result` | `review_result TEXT CHECK (review_result IN ('confirmed', 'rejected'))` |
| `review_comment` | `review_comment TEXT` |
| `customer_copy_status` | `customer_copy_status TEXT NOT NULL DEFAULT 'not_requested' CHECK ( customer_copy_status IN ('not_requested', 'queued', 'processing', 'available', 'failed') )` |
| `customer_copy_file_id` | `customer_copy_file_id UUID REFERENCES attachments(id)` |
| `customer_copy_hash` | `customer_copy_hash TEXT` |
| `stamp_version_id` | `stamp_version_id UUID` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `drug_inspection_asn_links`

- 模块：mdi_documents
- 迁移：`backend/migrations/202607250002_mdi_documents.sql`
- 货主字段：有
- 索引：`drug_inspection_asn_links_report_idx`
- ALTER 迁移：无
- 引用表：`auth_owners`, `auth_users`, `drug_inspection_report_versions`, `drug_inspection_reports`, `receiving_orders`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id)` |
| `asn_id` | `asn_id UUID NOT NULL` |
| `batch_no` | `batch_no TEXT NOT NULL` |
| `report_id` | `report_id UUID NOT NULL REFERENCES drug_inspection_reports(id)` |
| `source_version_id` | `source_version_id UUID NOT NULL REFERENCES drug_inspection_report_versions(id)` |
| `source` | `source TEXT NOT NULL CHECK (source IN ('uploaded', 'reused'))` |
| `linked_by` | `linked_by UUID NOT NULL REFERENCES auth_users(id)` |
| `linked_at` | `linked_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `upstream_delivery_documents`

- 模块：mdi_documents
- 迁移：`backend/migrations/202607250002_mdi_documents.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`, `auth_users`, `suppliers`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id)` |
| `supplier_id` | `supplier_id UUID NOT NULL` |
| `created_by` | `created_by UUID NOT NULL REFERENCES auth_users(id)` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `upstream_delivery_document_versions`

- 模块：mdi_documents
- 迁移：`backend/migrations/202607250002_mdi_documents.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`, `auth_users`, `upstream_delivery_documents`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `document_id` | `document_id UUID NOT NULL REFERENCES upstream_delivery_documents(id)` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id)` |
| `version_number` | `version_number INT NOT NULL CHECK (version_number > 0)` |
| `modification_reason` | `modification_reason TEXT` |
| `uploaded_by` | `uploaded_by UUID NOT NULL REFERENCES auth_users(id)` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `upstream_delivery_document_files`

- 模块：mdi_documents
- 迁移：`backend/migrations/202607250002_mdi_documents.sql`
- 货主字段：无
- 索引：无
- ALTER 迁移：无
- 引用表：`attachments`, `upstream_delivery_document_versions`

| 字段 | SQL 定义 |
|---|---|
| `version_id` | `version_id UUID NOT NULL REFERENCES upstream_delivery_document_versions(id)` |
| `attachment_id` | `attachment_id UUID NOT NULL REFERENCES attachments(id)` |
| `position` | `position INT NOT NULL CHECK (position > 0)` |

### `upstream_delivery_document_asn_links`

- 模块：mdi_documents
- 迁移：`backend/migrations/202607250002_mdi_documents.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`, `auth_users`, `receiving_orders`, `upstream_delivery_document_versions`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id)` |
| `version_id` | `version_id UUID NOT NULL REFERENCES upstream_delivery_document_versions(id)` |
| `asn_id` | `asn_id UUID NOT NULL` |
| `linked_by` | `linked_by UUID NOT NULL REFERENCES auth_users(id)` |
| `linked_at` | `linked_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `upstream_delivery_asn_current`

- 模块：mdi_documents
- 迁移：`backend/migrations/202607250002_mdi_documents.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`, `receiving_orders`, `upstream_delivery_document_versions`

| 字段 | SQL 定义 |
|---|---|
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id)` |
| `asn_id` | `asn_id UUID NOT NULL` |
| `version_id` | `version_id UUID NOT NULL REFERENCES upstream_delivery_document_versions(id)` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `drug_inspection_stamp_versions`

- 模块：mdi_customer_copy
- 迁移：`backend/migrations/202607250003_mdi_customer_copy.sql`
- 货主字段：有
- 索引：`UNIQUE drug_inspection_stamp_one_open_idx`, `UNIQUE drug_inspection_stamp_one_published_idx`
- ALTER 迁移：无
- 引用表：`attachments`, `auth_owners`, `auth_users`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id)` |
| `version_number` | `version_number INT NOT NULL CHECK (version_number > 0)` |
| `png_attachment_id` | `png_attachment_id UUID NOT NULL REFERENCES attachments(id)` |
| `relative_x` | `relative_x DOUBLE PRECISION NOT NULL CHECK (relative_x BETWEEN 0 AND 1)` |
| `relative_y` | `relative_y DOUBLE PRECISION NOT NULL CHECK (relative_y BETWEEN 0 AND 1)` |
| `relative_width` | `relative_width DOUBLE PRECISION NOT NULL CHECK ( relative_width > 0 AND relative_width <= 1 )` |
| `status` | `status TEXT NOT NULL CHECK ( status IN ('draft', 'pending_review', 'published', 'superseded') )` |
| `configured_by` | `configured_by UUID NOT NULL REFERENCES auth_users(id)` |
| `submitted_at` | `submitted_at TIMESTAMPTZ` |
| `reviewed_by` | `reviewed_by UUID REFERENCES auth_users(id)` |
| `reviewed_at` | `reviewed_at TIMESTAMPTZ` |
| `review_comment` | `review_comment TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `drug_inspection_customer_copy_jobs`

- 模块：mdi_customer_copy
- 迁移：`backend/migrations/202607250003_mdi_customer_copy.sql`
- 货主字段：有
- 索引：`UNIQUE drug_inspection_customer_copy_one_active_idx`, `drug_inspection_customer_copy_jobs_poll_idx`
- ALTER 迁移：无
- 引用表：`attachments`, `auth_owners`, `auth_users`, `drug_inspection_report_versions`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id)` |
| `report_version_id` | `report_version_id UUID NOT NULL REFERENCES drug_inspection_report_versions(id)` |
| `status` | `status TEXT NOT NULL CHECK ( status IN ('queued', 'processing', 'succeeded', 'failed', 'oversize_review') )` |
| `attempt_count` | `attempt_count INT NOT NULL DEFAULT 0 CHECK (attempt_count >= 0)` |
| `processing_rule` | `processing_rule TEXT NOT NULL DEFAULT 'mdi-image-v1'` |
| `oversize_reason` | `oversize_reason TEXT` |
| `oversize_approved_by` | `oversize_approved_by UUID REFERENCES auth_users(id)` |
| `candidate_file_id` | `candidate_file_id UUID REFERENCES attachments(id)` |
| `candidate_hash` | `candidate_hash TEXT` |
| `candidate_size` | `candidate_size BIGINT CHECK (candidate_size IS NULL OR candidate_size > 0)` |
| `last_error` | `last_error TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `started_at` | `started_at TIMESTAMPTZ` |
| `finished_at` | `finished_at TIMESTAMPTZ` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `drug_inspection_processing_rule_versions`

- 模块：mdi_customer_copy
- 迁移：`backend/migrations/202607250003_mdi_customer_copy.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`, `auth_users`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id)` |
| `version_number` | `version_number INT NOT NULL CHECK (version_number > 0)` |
| `rule_code` | `rule_code TEXT NOT NULL` |
| `apply_scope` | `apply_scope TEXT NOT NULL CHECK ( apply_scope IN ('future_only', 'reprocess_current') )` |
| `reprocess_job_count` | `reprocess_job_count INT NOT NULL DEFAULT 0 CHECK (reprocess_job_count >= 0)` |
| `published_by` | `published_by UUID NOT NULL REFERENCES auth_users(id)` |
| `published_at` | `published_at TIMESTAMPTZ NOT NULL` |

### `drug_inspection_requirement_rules`

- 模块：mdi_acceptance_validation
- 迁移：`backend/migrations/202607250005_mdi_acceptance_validation.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`, `auth_users`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id)` |
| `special_drug_category` | `special_drug_category TEXT NOT NULL` |
| `missing_behavior` | `missing_behavior TEXT NOT NULL CHECK (missing_behavior IN ('warning', 'block'))` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `version` | `version BIGINT NOT NULL DEFAULT 1 CHECK (version > 0)` |
| `updated_by` | `updated_by UUID NOT NULL REFERENCES auth_users(id)` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `drug_inspection_acceptance_validations`

- 模块：mdi_acceptance_validation
- 迁移：`backend/migrations/202607250005_mdi_acceptance_validation.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`auth_owners`, `drug_inspection_report_versions`, `drug_inspection_requirement_rules`, `products`, `receiving_orders`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL REFERENCES auth_owners(id)` |
| `receiving_order_id` | `receiving_order_id UUID NOT NULL` |
| `batch_no` | `batch_no TEXT NOT NULL` |
| `product_id` | `product_id UUID NOT NULL` |
| `rule_id` | `rule_id UUID` |
| `rule_version` | `rule_version BIGINT` |
| `result` | `result TEXT NOT NULL CHECK ( result IN ('not_required', 'valid', 'missing_warning', 'missing_blocked', 'unqualified_blocked') )` |
| `report_version_id` | `report_version_id UUID REFERENCES drug_inspection_report_versions(id)` |
| `idempotency_key` | `idempotency_key TEXT NOT NULL` |
| `detail` | `detail JSONB NOT NULL` |
| `validated_at` | `validated_at TIMESTAMPTZ NOT NULL` |

### `h9_route_bindings`

- 模块：h9_delivery_note_aggregation
- 迁移：`backend/migrations/202607260003_h9_delivery_note_aggregation.sql`
- 货主字段：有
- 索引：`h9_route_bindings_resolution_idx`
- ALTER 迁移：无
- 引用表：`customer_addresses`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `warehouse_id` | `warehouse_id UUID NOT NULL` |
| `customer_id` | `customer_id UUID NOT NULL` |
| `delivery_address_id` | `delivery_address_id UUID NOT NULL` |
| `route_code` | `route_code TEXT NOT NULL` |
| `effective_from` | `effective_from TIMESTAMPTZ NOT NULL` |
| `effective_to` | `effective_to TIMESTAMPTZ` |
| `created_by` | `created_by UUID NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h9_cutoff_plans`

- 模块：h9_delivery_note_aggregation
- 迁移：`backend/migrations/202607260003_h9_delivery_note_aggregation.sql`
- 货主字段：有
- 索引：`h9_cutoff_plans_resolution_idx`
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `name` | `name TEXT NOT NULL` |
| `warehouse_id` | `warehouse_id UUID NOT NULL` |
| `scope_type` | `scope_type TEXT NOT NULL` |
| `customer_id` | `customer_id UUID` |
| `route_code` | `route_code TEXT` |
| `utc_offset_minutes` | `utc_offset_minutes SMALLINT NOT NULL` |
| `weekly_schedule` | `weekly_schedule JSONB NOT NULL` |
| `exceptions` | `exceptions JSONB NOT NULL DEFAULT '[]'::jsonb` |
| `effective_from` | `effective_from TIMESTAMPTZ NOT NULL` |
| `effective_to` | `effective_to TIMESTAMPTZ` |
| `status` | `status TEXT NOT NULL DEFAULT 'draft'` |
| `created_by` | `created_by UUID NOT NULL` |
| `published_by` | `published_by UUID` |
| `published_at` | `published_at TIMESTAMPTZ` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h9_outbound_route_snapshots`

- 模块：h9_delivery_note_aggregation
- 迁移：`backend/migrations/202607260003_h9_delivery_note_aggregation.sql`
- 货主字段：有
- 索引：`h9_outbound_route_snapshots_boundary_idx`
- ALTER 迁移：无
- 引用表：`customer_addresses`, `outbound_orders`

| 字段 | SQL 定义 |
|---|---|
| `outbound_order_id` | `outbound_order_id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `warehouse_id` | `warehouse_id UUID NOT NULL` |
| `customer_id` | `customer_id UUID NOT NULL` |
| `delivery_address_id` | `delivery_address_id UUID NOT NULL` |
| `route_code` | `route_code TEXT NOT NULL` |
| `frozen_at` | `frozen_at TIMESTAMPTZ NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h9_delivery_note_groups`

- 模块：h9_delivery_note_aggregation
- 迁移：`backend/migrations/202607260003_h9_delivery_note_aggregation.sql`
- 货主字段：有
- 索引：`UNIQUE h9_delivery_note_groups_scheduled_once_uidx`, `h9_delivery_note_groups_boundary_idx`
- ALTER 迁移：`backend/migrations/202607260005_h9_aggregation_rules.sql`
- 引用表：`customer_addresses`, `h9_aggregation_rule_versions`, `h9_cutoff_plans`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `warehouse_id` | `warehouse_id UUID NOT NULL` |
| `customer_id` | `customer_id UUID NOT NULL` |
| `delivery_address_id` | `delivery_address_id UUID NOT NULL` |
| `route_code` | `route_code TEXT NOT NULL` |
| `delivery_note_no` | `delivery_note_no TEXT NOT NULL` |
| `cutoff_mode` | `cutoff_mode TEXT NOT NULL` |
| `cutoff_reason` | `cutoff_reason TEXT` |
| `cutoff_plan_id` | `cutoff_plan_id UUID` |
| `scheduled_cutoff_at` | `scheduled_cutoff_at TIMESTAMPTZ` |
| `cutoff_at` | `cutoff_at TIMESTAMPTZ NOT NULL` |
| `created_by` | `created_by UUID NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h9_delivery_note_group_orders`

- 模块：h9_delivery_note_aggregation
- 迁移：`backend/migrations/202607260003_h9_delivery_note_aggregation.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`h9_delivery_note_groups`, `h9_outbound_route_snapshots`

| 字段 | SQL 定义 |
|---|---|
| `group_id` | `group_id UUID NOT NULL` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `outbound_order_id` | `outbound_order_id UUID NOT NULL` |
| `warehouse_id` | `warehouse_id UUID NOT NULL` |
| `customer_id` | `customer_id UUID NOT NULL` |
| `delivery_address_id` | `delivery_address_id UUID NOT NULL` |
| `route_code` | `route_code TEXT NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `purchase_return_orders`

- 模块：m4_purchase_return_orders
- 迁移：`backend/migrations/202607260004_m4_purchase_return_orders.sql`
- 货主字段：有
- 索引：`UNIQUE purchase_return_orders_owner_id_uidx`, `purchase_return_orders_owner_status_idx`
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `warehouse_id` | `warehouse_id UUID NOT NULL` |
| `return_no` | `return_no TEXT NOT NULL` |
| `document_type` | `document_type TEXT NOT NULL DEFAULT 'purchase_return_outbound'` |
| `source_purchase_order_no` | `source_purchase_order_no TEXT NOT NULL` |
| `supplier_id` | `supplier_id UUID` |
| `supplier_name` | `supplier_name TEXT NOT NULL` |
| `reason` | `reason TEXT NOT NULL` |
| `approval_source` | `approval_source TEXT NOT NULL DEFAULT 'purchase_return_approval'` |
| `status` | `status TEXT NOT NULL` |
| `product_code` | `product_code TEXT NOT NULL` |
| `qty` | `qty BIGINT NOT NULL CHECK (qty > 0)` |
| `reject_reason` | `reject_reason TEXT` |
| `shipped_at` | `shipped_at TIMESTAMPTZ` |
| `shipped_by` | `shipped_by UUID` |
| `shipped_by_name` | `shipped_by_name TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `version` | `version BIGINT NOT NULL DEFAULT 1` |

### `h9_aggregation_field_catalog`

- 模块：h9_aggregation_rules
- 迁移：`backend/migrations/202607260005_h9_aggregation_rules.sql`
- 货主字段：无
- 索引：无
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `field_code` | `field_code TEXT PRIMARY KEY` |
| `display_name` | `display_name TEXT NOT NULL` |
| `value_type` | `value_type TEXT NOT NULL` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `sort_order` | `sort_order INT NOT NULL` |

### `h9_aggregation_rule_versions`

- 模块：h9_aggregation_rules
- 迁移：`backend/migrations/202607260005_h9_aggregation_rules.sql`
- 货主字段：有
- 索引：`UNIQUE h9_aggregation_rule_one_published_uidx`
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `version_no` | `version_no INT NOT NULL` |
| `name` | `name TEXT NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'draft'` |
| `dimensions` | `dimensions JSONB NOT NULL` |
| `test_result` | `test_result JSONB` |
| `tested_by` | `tested_by UUID` |
| `tested_at` | `tested_at TIMESTAMPTZ` |
| `published_by` | `published_by UUID` |
| `published_at` | `published_at TIMESTAMPTZ` |
| `disabled_by` | `disabled_by UUID` |
| `disabled_at` | `disabled_at TIMESTAMPTZ` |
| `created_by` | `created_by UUID NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h9_print_sites`

- 模块：h9_print_devices
- 迁移：`backend/migrations/202607270001_h9_print_devices.sql`
- 货主字段：无
- 索引：无
- ALTER 迁移：无
- 引用表：无

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `site_code` | `site_code TEXT NOT NULL UNIQUE` |
| `site_name` | `site_name TEXT NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'active'` |
| `created_by` | `created_by UUID NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h9_print_site_owner_mappings`

- 模块：h9_print_devices
- 迁移：`backend/migrations/202607270001_h9_print_devices.sql`
- 货主字段：有
- 索引：`UNIQUE h9_print_site_owner_mappings_active_uidx`
- ALTER 迁移：无
- 引用表：`h9_print_sites`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `site_id` | `site_id UUID NOT NULL REFERENCES h9_print_sites(id) ON DELETE RESTRICT` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `warehouse_id` | `warehouse_id UUID NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'active'` |
| `created_by` | `created_by UUID NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `disabled_by` | `disabled_by UUID` |
| `disabled_at` | `disabled_at TIMESTAMPTZ` |

### `h9_printers`

- 模块：h9_print_devices
- 迁移：`backend/migrations/202607270001_h9_print_devices.sql`
- 货主字段：无
- 索引：无
- ALTER 迁移：无
- 引用表：`h9_print_sites`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `site_id` | `site_id UUID NOT NULL REFERENCES h9_print_sites(id) ON DELETE RESTRICT` |
| `printer_name` | `printer_name TEXT NOT NULL` |
| `printer_model` | `printer_model TEXT` |
| `connection_type` | `connection_type TEXT NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'active'` |
| `release_mode_override` | `release_mode_override TEXT` |
| `created_by` | `created_by UUID NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h9_printer_trays`

- 模块：h9_print_devices
- 迁移：`backend/migrations/202607270001_h9_print_devices.sql`
- 货主字段：无
- 索引：无
- ALTER 迁移：无
- 引用表：`h9_printers`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `site_id` | `site_id UUID NOT NULL` |
| `printer_id` | `printer_id UUID NOT NULL` |
| `tray_code` | `tray_code TEXT NOT NULL` |
| `paper_size` | `paper_size TEXT NOT NULL` |
| `paper_type` | `paper_type TEXT NOT NULL` |
| `enabled` | `enabled BOOLEAN NOT NULL DEFAULT TRUE` |
| `created_by` | `created_by UUID NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h9_device_leases`

- 模块：h9_print_devices
- 迁移：`backend/migrations/202607270001_h9_print_devices.sql`
- 货主字段：无
- 索引：`UNIQUE h9_device_leases_one_active_uidx`, `h9_device_leases_site_idx`
- ALTER 迁移：无
- 引用表：`h9_printers`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `site_id` | `site_id UUID NOT NULL` |
| `printer_id` | `printer_id UUID NOT NULL` |
| `holder_agent_id` | `holder_agent_id UUID` |
| `lease_token` | `lease_token TEXT NOT NULL` |
| `release_mode` | `release_mode TEXT NOT NULL` |
| `busy_state` | `busy_state TEXT NOT NULL DEFAULT 'idle'` |
| `status` | `status TEXT NOT NULL DEFAULT 'active'` |
| `assigned_at` | `assigned_at TIMESTAMPTZ NOT NULL` |
| `acquired_at` | `acquired_at TIMESTAMPTZ` |
| `released_at` | `released_at TIMESTAMPTZ` |
| `released_by` | `released_by UUID` |
| `release_reason` | `release_reason TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h9_printer_test_prints`

- 模块：h9_print_devices
- 迁移：`backend/migrations/202607270001_h9_print_devices.sql`
- 货主字段：无
- 索引：`h9_printer_test_prints_printer_idx`
- ALTER 迁移：无
- 引用表：`h9_printer_trays`, `h9_printers`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `site_id` | `site_id UUID NOT NULL` |
| `printer_id` | `printer_id UUID NOT NULL` |
| `tray_id` | `tray_id UUID NOT NULL` |
| `result` | `result TEXT NOT NULL DEFAULT 'dispatched'` |
| `result_note` | `result_note TEXT` |
| `requested_by` | `requested_by UUID NOT NULL` |
| `requested_at` | `requested_at TIMESTAMPTZ NOT NULL` |
| `result_at` | `result_at TIMESTAMPTZ` |

### `h9_print_suite_versions`

- 模块：h9_print_suites
- 迁移：`backend/migrations/202607270002_h9_print_suites.sql`
- 货主字段：有
- 索引：`h9_print_suite_versions_resolution_idx`
- ALTER 迁移：无
- 引用表：`customer_addresses`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `version_no` | `version_no INT NOT NULL` |
| `name` | `name TEXT NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'draft'` |
| `warehouse_id` | `warehouse_id UUID NOT NULL` |
| `scope_type` | `scope_type TEXT NOT NULL` |
| `customer_id` | `customer_id UUID` |
| `delivery_address_id` | `delivery_address_id UUID` |
| `route_code` | `route_code TEXT` |
| `effective_from` | `effective_from TIMESTAMPTZ NOT NULL` |
| `effective_to` | `effective_to TIMESTAMPTZ` |
| `test_result` | `test_result JSONB` |
| `tested_by` | `tested_by UUID` |
| `tested_at` | `tested_at TIMESTAMPTZ` |
| `published_by` | `published_by UUID` |
| `published_at` | `published_at TIMESTAMPTZ` |
| `disabled_by` | `disabled_by UUID` |
| `disabled_at` | `disabled_at TIMESTAMPTZ` |
| `created_by` | `created_by UUID NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h9_print_suite_items`

- 模块：h9_print_suites
- 迁移：`backend/migrations/202607270002_h9_print_suites.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`h9_print_suite_versions`, `print_template_versions`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `suite_version_id` | `suite_version_id UUID NOT NULL` |
| `category_code` | `category_code TEXT NOT NULL` |
| `copies` | `copies INT NOT NULL` |
| `sort_order` | `sort_order INT NOT NULL` |
| `output_slot` | `output_slot TEXT NOT NULL` |
| `required` | `required BOOLEAN NOT NULL` |
| `ready_policy` | `ready_policy TEXT NOT NULL` |
| `failure_policy` | `failure_policy TEXT NOT NULL` |
| `source_mode` | `source_mode TEXT NOT NULL` |
| `template_version_id` | `template_version_id UUID` |
| `external_file_ref` | `external_file_ref TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h9_print_suite_instances`

- 模块：h9_print_suites
- 迁移：`backend/migrations/202607270002_h9_print_suites.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`h9_delivery_note_groups`, `h9_print_suite_versions`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `group_id` | `group_id UUID NOT NULL` |
| `suite_version_id` | `suite_version_id UUID NOT NULL` |
| `suite_version_no` | `suite_version_no INT NOT NULL` |
| `suite_snapshot` | `suite_snapshot JSONB NOT NULL` |
| `aggregation_rule_version_id` | `aggregation_rule_version_id UUID` |
| `aggregation_rule_version_no` | `aggregation_rule_version_no INT` |
| `source_documents` | `source_documents JSONB NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'waiting_documents'` |
| `hold_scope` | `hold_scope TEXT` |
| `created_by` | `created_by UUID NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h9_print_suite_instance_items`

- 模块：h9_print_suites
- 迁移：`backend/migrations/202607270002_h9_print_suites.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`h9_print_suite_instances`, `print_template_versions`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `instance_id` | `instance_id UUID NOT NULL` |
| `category_code` | `category_code TEXT NOT NULL` |
| `copies` | `copies INT NOT NULL` |
| `sort_order` | `sort_order INT NOT NULL` |
| `output_slot` | `output_slot TEXT NOT NULL` |
| `required` | `required BOOLEAN NOT NULL` |
| `ready_policy` | `ready_policy TEXT NOT NULL` |
| `failure_policy` | `failure_policy TEXT NOT NULL` |
| `source_mode` | `source_mode TEXT NOT NULL` |
| `template_version_id` | `template_version_id UUID` |
| `external_file_ref` | `external_file_ref TEXT` |
| `file_bindings` | `file_bindings JSONB NOT NULL DEFAULT '[]'::jsonb` |
| `ready` | `ready BOOLEAN NOT NULL DEFAULT FALSE` |
| `missing` | `missing JSONB NOT NULL DEFAULT '[]'::jsonb` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h9_document_file_bindings`

- 模块：h_file_h9_category_pdfs
- 迁移：`backend/migrations/202607280001_h_file_h9_category_pdfs.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`attachments`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `category_code` | `category_code TEXT NOT NULL` |
| `attachment_id` | `attachment_id UUID NOT NULL` |
| `invoice_no` | `invoice_no TEXT` |
| `product_code` | `product_code TEXT` |
| `batch_no` | `batch_no TEXT` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |

### `h9_category_pdf_preparations`

- 模块：h_file_h9_category_pdfs
- 迁移：`backend/migrations/202607280001_h_file_h9_category_pdfs.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`h9_print_suite_instances`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `instance_id` | `instance_id UUID NOT NULL` |
| `idempotency_key` | `idempotency_key TEXT NOT NULL` |
| `request_hash` | `request_hash TEXT NOT NULL` |
| `status` | `status TEXT NOT NULL DEFAULT 'processing'` |
| `last_error` | `last_error TEXT` |
| `created_by` | `created_by UUID NOT NULL` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `updated_at` | `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `completed_at` | `completed_at TIMESTAMPTZ` |

### `h9_category_pdf_outputs`

- 模块：h_file_h9_category_pdfs
- 迁移：`backend/migrations/202607280001_h_file_h9_category_pdfs.sql`
- 货主字段：有
- 索引：无
- ALTER 迁移：无
- 引用表：`attachments`, `h9_category_pdf_preparations`, `h9_print_suite_instances`, `print_template_versions`

| 字段 | SQL 定义 |
|---|---|
| `id` | `id UUID PRIMARY KEY` |
| `owner_id` | `owner_id UUID NOT NULL` |
| `preparation_id` | `preparation_id UUID NOT NULL` |
| `instance_id` | `instance_id UUID NOT NULL` |
| `instance_item_id` | `instance_item_id UUID NOT NULL` |
| `category_code` | `category_code TEXT NOT NULL` |
| `source_mode` | `source_mode TEXT NOT NULL` |
| `source_data_version` | `source_data_version TEXT` |
| `source_file_bindings` | `source_file_bindings JSONB NOT NULL DEFAULT '[]'::jsonb` |
| `template_version_id` | `template_version_id UUID` |
| `attachment_id` | `attachment_id UUID` |
| `content_hash` | `content_hash TEXT` |
| `processing_status` | `processing_status TEXT NOT NULL DEFAULT 'pending'` |
| `failure_reason` | `failure_reason TEXT` |
| `retention_policy` | `retention_policy TEXT NOT NULL` |
| `cache_expires_at` | `cache_expires_at TIMESTAMPTZ` |
| `attempt_count` | `attempt_count INT NOT NULL DEFAULT 0` |
| `created_at` | `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` |
| `processed_at` | `processed_at TIMESTAMPTZ` |

## Schema 变更事件

| 类型 | 表 | 目标 | 迁移 |
|---|---|---|---|
| alter | `billing_charge_calculations` | — | `backend/migrations/202606280002_database_design_standard_alignment.sql` |
| alter | `billing_contracts` | — | `backend/migrations/202606280002_database_design_standard_alignment.sql` |
| alter | `billing_rules` | — | `backend/migrations/202606280002_database_design_standard_alignment.sql` |
| alter | `billing_statement_charges` | — | `backend/migrations/202606280002_database_design_standard_alignment.sql` |
| alter | `billing_statements` | — | `backend/migrations/202606280002_database_design_standard_alignment.sql` |
| alter | `crossdock_plans` | — | `backend/migrations/202606280002_database_design_standard_alignment.sql` |
| alter | `inventory_movements` | — | `backend/migrations/202606280002_database_design_standard_alignment.sql` |
| alter | `inventory_status_changes` | — | `backend/migrations/202606280002_database_design_standard_alignment.sql` |
| alter | `outbound_order_lines` | — | `backend/migrations/202606280002_database_design_standard_alignment.sql` |
| alter | `outbound_shipments` | — | `backend/migrations/202606280002_database_design_standard_alignment.sql` |
| alter | `outbound_wave_orders` | — | `backend/migrations/202606280002_database_design_standard_alignment.sql` |
| alter | `packing_jobs` | — | `backend/migrations/202606280002_database_design_standard_alignment.sql` |
| alter | `receiving_inspection_signatures` | — | `backend/migrations/202606280002_database_design_standard_alignment.sql` |
| alter | `receiving_inspections` | — | `backend/migrations/202606280002_database_design_standard_alignment.sql` |
| alter | `receiving_order_lines` | — | `backend/migrations/202606280002_database_design_standard_alignment.sql` |
| alter | `receiving_order_receipts` | — | `backend/migrations/202606280002_database_design_standard_alignment.sql` |
| alter | `receiving_putaways` | — | `backend/migrations/202606280002_database_design_standard_alignment.sql` |
| alter | `tms_dispatches` | — | `backend/migrations/202606280002_database_design_standard_alignment.sql` |
| alter | `traceability_outbound_report_events` | — | `backend/migrations/202606280002_database_design_standard_alignment.sql` |
| alter | `transit_temperature_readings` | — | `backend/migrations/202606280002_database_design_standard_alignment.sql` |
| alter | `customers` | — | `backend/migrations/202607020002_master_data_source.sql` |
| alter | `products` | — | `backend/migrations/202607020002_master_data_source.sql` |
| alter | `suppliers` | — | `backend/migrations/202607020002_master_data_source.sql` |
| alter | `h5_express_waybills` | — | `backend/migrations/202607090003_h5_express_cancel_and_dedupe.sql` |
| alter | `customers` | — | `backend/migrations/202607120001_customer_license_no.sql` |
| alter | `auth_roles` | — | `backend/migrations/202607120002_h1_role_management.sql` |
| alter | `products` | — | `backend/migrations/202607120005_m1_product_attrs.sql` |
| alter | `outbound_orders` | — | `backend/migrations/202607130004_m4_outbound_document_type.sql` |
| alter | `warehouse_locations` | — | `backend/migrations/202607130006_m1_location_owner_binding.sql` |
| alter | `warehouse_zones` | — | `backend/migrations/202607130007_m3_quality_color_status_mapping.sql` |
| alter | `receiving_order_receipts` | — | `backend/migrations/202607130009_receiving_receipt_details.sql` |
| alter | `customers` | — | `backend/migrations/202607130012_m1_customer_profile_fields.sql` |
| alter | `auth_users` | — | `backend/migrations/202607140002_m1_user_management.sql` |
| alter | `receiving_inspection_signatures` | — | `backend/migrations/202607150002_mvr_downstream_enforcement.sql` |
| alter | `task_group_memberships` | — | `backend/migrations/202607150005_mte_worker_qualifications.sql` |
| alter | `warehouse_tasks` | — | `backend/migrations/202607150006_mte_task_priority_rules.sql` |
| alter | `task_types` | — | `backend/migrations/202607150007_mte_task_release_control.sql` |
| alter | `warehouse_tasks` | — | `backend/migrations/202607150007_mte_task_release_control.sql` |
| alter | `inventory_movements` | — | `backend/migrations/202607150008_msa_stock_loss.sql` |
| alter | `stock_adjustment_orders` | — | `backend/migrations/202607150009_msa_stock_surplus.sql` |
| alter | `alert_definitions` | — | `backend/migrations/202607150011_hal_alert_definition_workflow.sql` |
| alter | `inventory_batches` | — | `backend/migrations/202607170001_m3_inventory_query_snapshot.sql` |
| alter | `suppliers` | — | `backend/migrations/202607170006_m2_inbound_closeout.sql` |
| alter | `putaway_strategy_profiles` | — | `backend/migrations/202607180001_m2_putaway_strategy_config.sql` |
| alter | `receiving_inspections` | — | `backend/migrations/202607180002_m2_receive_inspect_gsp.sql` |
| alter | `receiving_putaways` | — | `backend/migrations/202607180003_m2_putaway_lpn_erp.sql` |
| alter | `h8_erp_message_attempts` | — | `backend/migrations/202607190008_h8_erp_message_attempts_archived.sql` |
| alter | `h8_erp_messages` | — | `backend/migrations/202607220001_h8_erp_message_receipts.sql` |
| alter | `h8_erp_connectors` | — | `backend/migrations/202607220002_h8_worker_runtime_and_payload_retention.sql` |
| alter | `h8_erp_messages` | — | `backend/migrations/202607220002_h8_worker_runtime_and_payload_retention.sql` |
| alter | `products` | — | `backend/migrations/202607230003_m1_complete_product_contract.sql` |
| alter | `outbound_shipments` | — | `backend/migrations/202607250001_h_file_attachments.sql` |
| alter | `drug_inspection_reports` | — | `backend/migrations/202607250002_mdi_documents.sql` |
| alter | `drug_inspection_report_versions` | — | `backend/migrations/202607250003_mdi_customer_copy.sql` |
| alter | `system_dictionary_items` | — | `backend/migrations/202607260001_h9_print_template_type_sort_order.sql` |
| alter | `h9_delivery_note_groups` | — | `backend/migrations/202607260005_h9_aggregation_rules.sql` |
| alter | `outbound_orders` | — | `backend/migrations/202607260005_h9_aggregation_rules.sql` |
| alter | `attachments` | — | `backend/migrations/202607280001_h_file_h9_category_pdfs.sql` |
| drop | `h9_ingested_document_files` | — | `backend/migrations/202607280001_h_file_h9_category_pdfs.sql` |
