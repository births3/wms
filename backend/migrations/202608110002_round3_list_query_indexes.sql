-- Round 3 列表查询索引：owner_id 过滤 + 排序键的组合索引（分页规范 §6）。
-- 列名均按实际表结构核对（grep migrations）；表/列不存在的路径跳过并在注释说明。

-- 说明：auth_users 无 owner_id 列（owner 归属经 auth_user_owner_bindings 解析，
-- 见 202606060001_h1_auth_tables.sql），role_users / worker_candidates 列表
-- 均为 binding.owner_id = $1 AND binding.is_active 过滤，故按实际查询路径为
-- auth_user_owner_bindings 建等价组合索引（覆盖 is_active 过滤，替代无法落库的
-- idx_auth_users_owner_username / idx_auth_users_owner_display）。
CREATE INDEX IF NOT EXISTS idx_auth_user_owner_bindings_owner_active_user
    ON auth_user_owner_bindings (owner_id, is_active, user_id);

-- api-keys 列表：WHERE owner_id = $1 ORDER BY created_at DESC, id DESC
CREATE INDEX IF NOT EXISTS idx_auth_api_keys_owner_created
    ON auth_api_keys (owner_id, created_at DESC, id DESC);

-- dock 预约列表：WHERE owner_id = $1 ORDER BY window_start_at ASC, dock_id ASC, id ASC
CREATE INDEX IF NOT EXISTS idx_dock_appointments_owner_window
    ON dock_appointments (owner_id, window_start_at ASC, dock_id, id);

-- review-queue 列表：WHERE owner_id = $1 ORDER BY submitted_at, created_at, id
CREATE INDEX IF NOT EXISTS idx_drug_inspection_report_versions_owner_submitted
    ON drug_inspection_report_versions (owner_id, submitted_at, created_at, id);

-- copy-jobs 列表：WHERE owner_id = $1 ORDER BY created_at DESC, id
-- （实际表名为 drug_inspection_customer_copy_jobs，见 202607250003_mdi_customer_copy.sql，
--   索引名按实际表名命名）
CREATE INDEX IF NOT EXISTS idx_drug_inspection_customer_copy_jobs_owner_created
    ON drug_inspection_customer_copy_jobs (owner_id, created_at DESC, id);

-- suite-instances 列表：WHERE owner_id = $1 ORDER BY created_at DESC, id
CREATE INDEX IF NOT EXISTS idx_h9_print_suite_instances_owner_created
    ON h9_print_suite_instances (owner_id, created_at DESC, id);
