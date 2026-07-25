//! E2E DB seed body.
use std::error::Error;

use sqlx::PgPool;
use uuid::Uuid;
// bcrypt used via crate dep of wms-api example

pub async fn seed_e2e_data(pool: &PgPool) -> Result<(), Box<dyn Error>> {
    let password_hash = bcrypt::hash("CorrectHorse1!", 4)?;
    sqlx::query(
        r#"
        INSERT INTO auth_owners (id, owner_code, owner_name)
        VALUES ('00000000-0000-0000-0000-000000000001', 'PY_OWNER', '鹏鹞药业')
        ON CONFLICT (id) DO UPDATE
        SET owner_code = EXCLUDED.owner_code,
            owner_name = EXCLUDED.owner_name
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO auth_users (id, username, display_name, password_hash, status, failed_login_count, locked_until)
        VALUES ('00000000-0000-0000-0000-000000000101', 'admin', '系统管理员', $1, 'active', 0, NULL)
        ON CONFLICT (id) DO UPDATE
        SET username = EXCLUDED.username,
            display_name = EXCLUDED.display_name,
            password_hash = EXCLUDED.password_hash,
            status = 'active',
            failed_login_count = 0,
            locked_until = NULL,
            updated_at = now()
        "#,
    )
    .bind(&password_hash)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary)
        VALUES ('00000000-0000-0000-0000-000000000101', '00000000-0000-0000-0000-000000000001', TRUE, TRUE)
        ON CONFLICT (user_id, owner_id) DO UPDATE
        SET is_active = TRUE,
            is_primary = TRUE
        "#,
    )
    .execute(pool)
    .await?;
    let system_admin_role_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO auth_roles (id, owner_id, role_code, role_name)
        VALUES ('00000000-0000-0000-0000-000000000102', '00000000-0000-0000-0000-000000000001', 'system_admin', '系统管理员')
        ON CONFLICT (owner_id, lower(role_code)) DO UPDATE
        SET role_code = EXCLUDED.role_code,
            role_name = EXCLUDED.role_name,
            data_scope = 'all'
        RETURNING id
        "#,
    )
    .fetch_one(pool)
    .await?;
    for (id, code, name) in [
        (
            "00000000-0000-0000-0000-000000000111",
            "m1.master_data.read",
            "基础档案读取",
        ),
        (
            "00000000-0000-0000-0000-000000000112",
            "m1.system_dictionary.read",
            "系统字典读取",
        ),
        (
            "00000000-0000-0000-0000-000000000113",
            "m1.master_data.write",
            "基础档案写入",
        ),
        (
            "00000000-0000-0000-0000-000000000114",
            "m2.write",
            "入库作业写入",
        ),
        (
            "00000000-0000-0000-0000-000000000115",
            "m3.write",
            "库存作业写入",
        ),
        (
            "00000000-0000-0000-0000-000000000116",
            "m3.read",
            "库存读取",
        ),
        (
            "00000000-0000-0000-0000-000000000126",
            "m3.recall.cancel",
            "M3 取消召回",
        ),
        (
            "00000000-0000-0000-0000-000000000127",
            "m3.recall.approve",
            "M3 召回取消质量审批",
        ),
        (
            "00000000-0000-0000-0000-000000000124",
            "m4.read",
            "出库读取",
        ),
        (
            "00000000-0000-0000-0000-000000000125",
            "m4.write",
            "出库作业写入",
        ),
        (
            "00000000-0000-0000-0000-000000000117",
            "audit.read",
            "审计读取",
        ),
        (
            "00000000-0000-0000-0000-000000000118",
            "h1.roles.manage",
            "H1 角色权限维护",
        ),
        (
            "00000000-0000-0000-0000-000000000119",
            "h1.api_keys.manage",
            "H1 API Key 生命周期管理",
        ),
        (
            "00000000-0000-0000-0000-000000000120",
            "h9.print_template.read",
            "H9 打印模板读取",
        ),
        (
            "00000000-0000-0000-0000-000000000121",
            "h9.print_template.print",
            "H9 业务打印",
        ),
        (
            "00000000-0000-0000-0000-000000000122",
            "mcg.document_numbering.read",
            "M-CG 单据号规则读取",
        ),
        (
            "00000000-0000-0000-0000-000000000123",
            "mcg.document_numbering.write",
            "M-CG 单据号规则维护",
        ),
        (
            "00000000-0000-0000-0000-000000000128",
            "m1.config.write",
            "M1 配置中心维护",
        ),
        (
            "00000000-0000-0000-0000-000000000129",
            "h8.erp_connector.read",
            "H8 ERP 连接只读",
        ),
        (
            "00000000-0000-0000-0000-00000000012a",
            "h8.erp_connector.write",
            "H8 ERP 连接维护",
        ),
        (
            "00000000-0000-0000-0000-00000000012b",
            "h8.erp_interface_table.read",
            "H8 ERP 接口表探查只读",
        ),
    ] {
        let permission_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO auth_permissions (id, permission_code, permission_name)
            VALUES ($1, $2, $3)
            ON CONFLICT (lower(permission_code)) DO UPDATE
            SET permission_name = EXCLUDED.permission_name
            RETURNING id
            "#,
        )
        .bind(Uuid::parse_str(id)?)
        .bind(code)
        .bind(name)
        .fetch_one(pool)
        .await?;
        sqlx::query(
            r#"
            INSERT INTO auth_role_permissions (role_id, permission_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(system_admin_role_id)
        .bind(permission_id)
        .execute(pool)
        .await?;
    }
    sqlx::query(
        r#"
        INSERT INTO auth_user_roles (user_id, owner_id, role_id)
        VALUES ('00000000-0000-0000-0000-000000000101', '00000000-0000-0000-0000-000000000001', $1)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(system_admin_role_id)
    .execute(pool)
    .await?;

    crate::wms_api_e2e_seed::seed_mvr_matrix_approver(pool, &password_hash, system_admin_role_id)
        .await?;
    crate::wms_api_e2e_seed::seed_quality_approver(pool, &password_hash).await?;
    crate::wms_api_e2e_seed::seed_h8_warehouse_manager(pool, &password_hash).await?;
    crate::wms_api_e2e_seed::seed_m9_m10_capabilities(pool).await?;
    crate::wms_api_e2e_seed::seed_m4_review_data(pool).await?;

    sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification, dosage_form,
            storage_condition, special_drug_category, approval_no, manufacturer, status,
            udi_code, electronic_regulatory_code,
            length_mm, width_mm, height_mm, volume_cm3, weight_g
        )
        VALUES (
            '00000000-0000-0000-0000-000000001001', '00000000-0000-0000-0000-000000000001',
            'P-M1-E2E-001', 'E2E 冷藏胰岛素', '10ml*1支', '注射剂',
            'cold', 'none', '国药准字E2E001', 'E2E 示例药业', 'active',
            '06901234567891', '81000000000000000001',
            120, 100, 30, 360, 180
        )
        ON CONFLICT (owner_id, product_code) DO UPDATE
        SET product_name = EXCLUDED.product_name,
            specification = EXCLUDED.specification,
            storage_condition = EXCLUDED.storage_condition,
            updated_at = now()
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO product_packaging_levels (
            id, owner_id, product_id, unit_code, unit_name, ratio_to_base,
            is_base, is_default, sort_order
        )
        VALUES
            (
                '00000000-0000-0000-0000-000000001011',
                '00000000-0000-0000-0000-000000000001',
                '00000000-0000-0000-0000-000000001001',
                'piece', '支', 1, TRUE, FALSE, 1
            ),
            (
                '00000000-0000-0000-0000-000000001012',
                '00000000-0000-0000-0000-000000000001',
                '00000000-0000-0000-0000-000000001001',
                'box', '盒', 10, FALSE, TRUE, 2
            ),
            (
                '00000000-0000-0000-0000-000000001013',
                '00000000-0000-0000-0000-000000000001',
                '00000000-0000-0000-0000-000000001001',
                'case', '箱', 200, FALSE, FALSE, 3
            )
        ON CONFLICT (owner_id, product_id, unit_code) DO UPDATE
        SET unit_name = EXCLUDED.unit_name,
            ratio_to_base = EXCLUDED.ratio_to_base,
            is_base = EXCLUDED.is_base,
            is_default = EXCLUDED.is_default,
            sort_order = EXCLUDED.sort_order
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO suppliers (id, owner_id, supplier_code, supplier_name, uscc, contact_name, status)
        VALUES (
            '00000000-0000-0000-0000-000000001101', '00000000-0000-0000-0000-000000000001',
            'S-M1-E2E-001', 'E2E 供应商', '91310000E2E000001', '王供应', 'active'
        )
        ON CONFLICT (owner_id, supplier_code) DO UPDATE
        SET supplier_name = EXCLUDED.supplier_name,
            uscc = EXCLUDED.uscc,
            contact_name = EXCLUDED.contact_name,
            updated_at = now()
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO customers (id, owner_id, customer_code, customer_name, customer_type, contact_name, status)
        VALUES (
            '00000000-0000-0000-0000-000000001201', '00000000-0000-0000-0000-000000000001',
            'C-M1-E2E-001', 'E2E 客户门店', 'store', '李客户', 'active'
        )
        ON CONFLICT (owner_id, customer_code) DO UPDATE
        SET customer_name = EXCLUDED.customer_name,
            customer_type = EXCLUDED.customer_type,
            contact_name = EXCLUDED.contact_name,
            updated_at = now()
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, address, status)
        VALUES (
            '00000000-0000-0000-0000-000000001301', '00000000-0000-0000-0000-000000000001',
            'WH-M1-E2E-001', 'E2E 冷链仓', 'physical', '上海 E2E 园区', 'active'
        )
        ON CONFLICT (owner_id, warehouse_code) DO UPDATE
        SET warehouse_name = EXCLUDED.warehouse_name,
            warehouse_type = EXCLUDED.warehouse_type,
            updated_at = now()
        "#,
    )
    .execute(pool)
    .await?;
    crate::wms_api_e2e_seed::seed_h8_interface_connector(pool).await?;
    sqlx::raw_sql(
        r#"
        INSERT INTO auth_owners (id, owner_code, owner_name)
        VALUES ('00000000-0000-0000-0000-000000000002', 'H8_OTHER_OWNER', 'H8 隔离验收货主')
        ON CONFLICT (id) DO UPDATE SET owner_name = EXCLUDED.owner_name;

        SELECT h8_erp_messages_ensure_month_partition(CURRENT_DATE);
        DELETE FROM h8_erp_message_attempts
        WHERE message_id IN (
            '00000000-0000-0000-0000-000000008901',
            '00000000-0000-0000-0000-000000008902',
            '00000000-0000-0000-0000-000000008999'
        );
        DELETE FROM h8_erp_messages
        WHERE id IN (
            '00000000-0000-0000-0000-000000008901',
            '00000000-0000-0000-0000-000000008902',
            '00000000-0000-0000-0000-000000008999'
        );

        INSERT INTO h8_erp_messages (
            id, owner_id, warehouse_id, connector_id, connector_code, config_version,
            direction, message_type, schema_version, channel, external_ref, wms_resource_id,
            idempotency_key, correlation_id, sync_status, retry_count, last_error_summary,
            payload_digest, created_at, updated_at, completed_at
        ) VALUES
        (
            '00000000-0000-0000-0000-000000008901',
            '00000000-0000-0000-0000-000000000001',
            '00000000-0000-0000-0000-000000001301',
            '00000000-0000-0000-0000-000000008801', 'H8-IF-E2E', 1,
            'inbound', 'asn', '1', 'interface_table', 'H8-MSG-E2E-DEAD', NULL,
            'h8-msg-e2e-dead-idem', 'h8-msg-e2e-dead-corr', 'dead', 2,
            'ERP 返回业务拒绝', 'h8-msg-e2e-dead-digest', now(), now(), now()
        ),
        (
            '00000000-0000-0000-0000-000000008902',
            '00000000-0000-0000-0000-000000000001',
            '00000000-0000-0000-0000-000000001301',
            '00000000-0000-0000-0000-000000008801', 'H8-IF-E2E', 1,
            'inbound', 'asn', '1', 'interface_table', 'H8-MSG-E2E-FAIL', NULL,
            'h8-msg-e2e-fail-idem', 'h8-msg-e2e-fail-corr', 'failed', 1,
            'ERP 暂时不可用', 'h8-msg-e2e-fail-digest', now(), now(), NULL
        ),
        (
            '00000000-0000-0000-0000-000000008999',
            '00000000-0000-0000-0000-000000000002', NULL, NULL, 'OTHER-ERP', 1,
            'inbound', 'asn', '1', 'rest', 'H8-MSG-E2E-OTHER-OWNER', NULL,
            'h8-msg-e2e-other-idem', 'h8-msg-e2e-other-corr', 'dead', 1,
            '跨货主隔离样本', 'h8-msg-e2e-other-digest', now(), now(), now()
        );

        INSERT INTO h8_erp_message_attempts (
            id, message_id, owner_id, attempt_no, channel, started_at, finished_at,
            result, error_summary, actor
        ) VALUES
        (
            '00000000-0000-0000-0000-000000008911',
            '00000000-0000-0000-0000-000000008901',
            '00000000-0000-0000-0000-000000000001', 1, 'interface_table',
            now() - interval '2 seconds', now(), 'dead', 'ERP 返回业务拒绝', 'e2e-worker'
        ),
        (
            '00000000-0000-0000-0000-000000008912',
            '00000000-0000-0000-0000-000000008902',
            '00000000-0000-0000-0000-000000000001', 1, 'interface_table',
            now() - interval '1 second', now(), 'failed', 'ERP 暂时不可用', 'e2e-worker'
        );

        INSERT INTO h8_erp_worker_heartbeats (
            owner_id, worker_id, worker_version, connector_id, directions, current_claims,
            created_at, last_heartbeat_at, heartbeat_expires_at
        ) VALUES (
            '00000000-0000-0000-0000-000000000001', 'h8-worker-real-e2e', '1.0.0',
            '00000000-0000-0000-0000-000000008801', ARRAY['inbound', 'outbound'], 1,
            now(), now(), now() + interval '1 day'
        )
        ON CONFLICT (owner_id, worker_id) DO UPDATE SET
            connector_id = EXCLUDED.connector_id,
            directions = EXCLUDED.directions,
            current_claims = EXCLUDED.current_claims,
            last_heartbeat_at = EXCLUDED.last_heartbeat_at,
            heartbeat_expires_at = EXCLUDED.heartbeat_expires_at;

        DELETE FROM h8_erp_worker_claim_controls
        WHERE owner_id = '00000000-0000-0000-0000-000000000001'
          AND connector_id = '00000000-0000-0000-0000-000000008801';

        INSERT INTO h8_erp_payload_retention_policies (
            owner_id, connector_id, enabled, retention_days, updated_by, updated_at
        ) VALUES (
            '00000000-0000-0000-0000-000000000001',
            '00000000-0000-0000-0000-000000008801', FALSE, 7, 'e2e-seed', now()
        )
        ON CONFLICT (owner_id, connector_id) DO UPDATE SET
            enabled = FALSE,
            retention_days = 7,
            updated_by = 'e2e-seed',
            updated_at = now();
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO warehouse_zones (
            id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status
        )
        VALUES (
            '00000000-0000-0000-0000-000000001302', '00000000-0000-0000-0000-000000000001',
            '00000000-0000-0000-0000-000000001301', 'A01', 'E2E 冷藏区', 'cold',
            'qualified_green', 'active'
        )
        ON CONFLICT (owner_id, warehouse_id, zone_code) DO UPDATE
        SET zone_name = EXCLUDED.zone_name,
            temperature_zone = EXCLUDED.temperature_zone,
            updated_at = now()
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no,
            max_volume_cm3, used_volume_cm3, max_sku_count, location_type, bound_owner_id, status
        )
        VALUES (
            '00000000-0000-0000-0000-000000001401', '00000000-0000-0000-0000-000000000001',
            '00000000-0000-0000-0000-000000001301', '00000000-0000-0000-0000-000000001302',
            'A01-01-02-03', 1, 2, 3, 1000000, 0, 1, 'storage', NULL, 'available'
        )
        ON CONFLICT (owner_id, location_code) DO UPDATE
        SET warehouse_id = EXCLUDED.warehouse_id,
            zone_id = EXCLUDED.zone_id,
            row_no = EXCLUDED.row_no,
            column_no = EXCLUDED.column_no,
            layer_no = EXCLUDED.layer_no,
            updated_at = now()
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_locked, quality_status, location_id, location_code
            , container_lpn
        )
        VALUES (
            '00000000-0000-0000-0000-000000001501', '00000000-0000-0000-0000-000000000001',
            'P-M1-E2E-001', 'B-M4-E2E-001', '2026-01-01', '2028-01-01',
            100, 0, 'qualified', '00000000-0000-0000-0000-000000001401', 'A01-01-02-03',
            'LPN-E2E-001'
        )
        ON CONFLICT (owner_id, product_code, batch_no, location_id, quality_status)
        DO UPDATE SET qty_on_hand = EXCLUDED.qty_on_hand,
                      qty_locked = 0,
                      container_lpn = EXCLUDED.container_lpn,
                      updated_at = now()
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO print_field_libraries (id, library_code, library_name, source_schema)
        VALUES (
            '00000000-0000-0000-0000-000000002802',
            'm2_acceptance_record',
            'M2 验收记录字段库',
            'ReceivingOrder'
        )
        ON CONFLICT (library_code) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO print_field_library_versions (
            id, library_id, version_no, status, published_at, published_by, request_hash
        )
        SELECT
            '00000000-0000-0000-0000-000000002902',
            libraries.id,
            1,
            'published',
            now(),
            '00000000-0000-0000-0000-000000000101',
            'wms-api-e2e-m2-acceptance-v1'
        FROM print_field_libraries libraries
        WHERE libraries.library_code = 'm2_acceptance_record'
        ON CONFLICT (library_id, version_no) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO print_field_definitions (
            id, library_version_id, field_path, field_type, source_schema,
            display_name, group_code, group_name, metadata, sort_order
        )
        SELECT
            seed.id,
            versions.id,
            seed.field_path,
            seed.field_type,
            seed.source_schema,
            seed.display_name,
            seed.group_code,
            seed.group_name,
            seed.metadata,
            seed.sort_order
        FROM print_field_library_versions versions
        JOIN print_field_libraries libraries ON libraries.id = versions.library_id
        CROSS JOIN (VALUES
            (
                '00000000-0000-0000-0000-000000004803'::uuid,
                'asn.code', 'string', 'ReceivingOrder', 'ASN 号', 'order', '订单信息',
                '{"printable": true, "sensitive": false}'::jsonb, 10
            ),
            (
                '00000000-0000-0000-0000-000000004804'::uuid,
                'product.code', 'string', 'ReceivingOrderLine', '商品编码', 'product', '商品信息',
                '{"printable": true, "sensitive": false}'::jsonb, 20
            )
        ) AS seed(id, field_path, field_type, source_schema, display_name, group_code, group_name, metadata, sort_order)
        WHERE libraries.library_code = 'm2_acceptance_record'
          AND versions.version_no = 1
        ON CONFLICT (library_version_id, field_path) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO print_templates (
            id, owner_id, template_code, template_name, template_type_code, scope,
            enabled, is_default, remark, created_by, updated_by
        )
        VALUES (
            '00000000-0000-0000-0000-000000003802',
            '00000000-0000-0000-0000-000000000001',
            'm2_acceptance_e2e', 'M2 验收记录 E2E 模板', 'acceptance_record', 'global',
            TRUE, TRUE, '真实数据 E2E 验收记录模板',
            '00000000-0000-0000-0000-000000000101',
            '00000000-0000-0000-0000-000000000101'
        )
        ON CONFLICT (owner_id, template_code) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO print_template_versions (
            id, template_id, field_library_version_id, version_no, status,
            hiprint_json, field_bindings, paper, designer_version, request_hash,
            created_by, published_at, published_by
        )
        SELECT
            '00000000-0000-0000-0000-000000003902',
            templates.id,
            versions.id,
            1,
            'published',
            '{"panels":[{"index":0,"paperType":"A4","width":210,"height":297,"printElements":[{"options":{"field":"asn.code","title":"ASN 号","left":20,"top":20,"width":120,"height":20},"printElementType":{"type":"text"}}]}]}'::jsonb,
            '[{"field_path":"asn.code","required":true},{"field_path":"product.code","required":false}]'::jsonb,
            '{"paperType":"A4","width":210,"height":297,"direction":"portrait"}'::jsonb,
            'hiprint@0.4.0',
            'wms-api-e2e-m2-acceptance-template-v1',
            '00000000-0000-0000-0000-000000000101',
            now(),
            '00000000-0000-0000-0000-000000000101'
        FROM print_templates templates
        JOIN print_field_libraries libraries ON libraries.library_code = 'm2_acceptance_record'
        JOIN print_field_library_versions versions
          ON versions.library_id = libraries.id AND versions.version_no = 1
        WHERE templates.owner_id = '00000000-0000-0000-0000-000000000001'
          AND templates.template_code = 'm2_acceptance_e2e'
        ON CONFLICT (template_id, version_no) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO print_field_libraries (id, library_code, library_name, source_schema)
        VALUES (
            '00000000-0000-0000-0000-000000002801',
            'm2_asn',
            'M2 ASN 字段库',
            'ReceivingOrder'
        )
        ON CONFLICT (library_code) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO print_field_library_versions (
            id, library_id, version_no, status, published_at, published_by, request_hash
        )
        SELECT
            '00000000-0000-0000-0000-000000002901',
            libraries.id,
            1,
            'published',
            now(),
            '00000000-0000-0000-0000-000000000101',
            'wms-api-e2e-m2-asn-v1'
        FROM print_field_libraries libraries
        WHERE libraries.library_code = 'm2_asn'
        ON CONFLICT (library_id, version_no) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO print_field_definitions (
            id, library_version_id, field_path, field_type, source_schema,
            display_name, group_code, group_name, metadata, sort_order
        )
        SELECT
            seed.id,
            versions.id,
            seed.field_path,
            seed.field_type,
            seed.source_schema,
            seed.display_name,
            seed.group_code,
            seed.group_name,
            seed.metadata,
            seed.sort_order
        FROM print_field_library_versions versions
        JOIN print_field_libraries libraries ON libraries.id = versions.library_id
        CROSS JOIN (VALUES
            (
                '00000000-0000-0000-0000-000000004801'::uuid,
                'asn.code', 'string', 'ReceivingOrder', 'ASN 号', 'order', '订单信息',
                '{"printable": true, "sensitive": false}'::jsonb, 10
            ),
            (
                '00000000-0000-0000-0000-000000004802'::uuid,
                'product.code', 'string', 'ReceivingOrderLine', '商品编码', 'product', '商品信息',
                '{"printable": true, "sensitive": false}'::jsonb, 20
            )
        ) AS seed(id, field_path, field_type, source_schema, display_name, group_code, group_name, metadata, sort_order)
        WHERE libraries.library_code = 'm2_asn'
          AND versions.version_no = 1
        ON CONFLICT (library_version_id, field_path) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO print_templates (
            id, owner_id, template_code, template_name, template_type_code, scope,
            enabled, is_default, remark, created_by, updated_by
        )
        VALUES (
            '00000000-0000-0000-0000-000000003801',
            '00000000-0000-0000-0000-000000000001',
            'm2_asn_e2e', 'M2 ASN E2E 模板', 'asn', 'global',
            TRUE, TRUE, '真实数据 E2E 模板',
            '00000000-0000-0000-0000-000000000101',
            '00000000-0000-0000-0000-000000000101'
        )
        ON CONFLICT (owner_id, template_code) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO print_template_versions (
            id, template_id, field_library_version_id, version_no, status,
            hiprint_json, field_bindings, paper, designer_version, request_hash,
            created_by, published_at, published_by
        )
        SELECT
            '00000000-0000-0000-0000-000000003901',
            templates.id,
            versions.id,
            1,
            'published',
            '{"panels":[{"index":0,"paperType":"A4","width":210,"height":297,"printElements":[{"options":{"field":"asn.code","title":"ASN 号","left":20,"top":20,"width":120,"height":20},"printElementType":{"type":"text"}}]}]}'::jsonb,
            '[{"field_path":"asn.code","required":true},{"field_path":"product.code","required":false}]'::jsonb,
            '{"paperType":"A4","width":210,"height":297,"direction":"portrait"}'::jsonb,
            'hiprint@0.4.0',
            'wms-api-e2e-m2-asn-template-v1',
            '00000000-0000-0000-0000-000000000101',
            now(),
            '00000000-0000-0000-0000-000000000101'
        FROM print_templates templates
        JOIN print_field_libraries libraries ON libraries.library_code = 'm2_asn'
        JOIN print_field_library_versions versions
          ON versions.library_id = libraries.id AND versions.version_no = 1
        WHERE templates.owner_id = '00000000-0000-0000-0000-000000000001'
          AND templates.template_code = 'm2_asn_e2e'
        ON CONFLICT (template_id, version_no) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO document_number_rules (
            id, owner_id, document_type, rule_code, rule_name, template,
            reset_policy, sequence_width, enabled, effective_from, created_at, updated_at
        )
        VALUES (
            '00000000-0000-0000-0000-000000005801',
            '00000000-0000-0000-0000-000000000001',
            'purchase_inbound',
            'purchase-inbound',
            '采购入库单号',
            '{OWNER}-ASN-{YYYY}{MM}{DD}-{SEQ}',
            'daily', 4, TRUE, now(), now(), now()
        )
        ON CONFLICT ((COALESCE(owner_id, '00000000-0000-0000-0000-000000000000'::uuid)), rule_code) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO document_number_rules (
            id, owner_id, document_type, rule_code, rule_name, template,
            reset_policy, sequence_width, enabled, effective_from, created_at, updated_at
        )
        VALUES (
            '00000000-0000-0000-0000-000000005802',
            '00000000-0000-0000-0000-000000000001',
            'sales_outbound',
            'sales-outbound',
            '销售出库单号',
            '{OWNER}-OUT-{YYYY}{MM}{DD}-{SEQ}',
            'daily', 4, TRUE, now(), now(), now()
        )
        ON CONFLICT ((COALESCE(owner_id, '00000000-0000-0000-0000-000000000000'::uuid)), rule_code) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    crate::wms_api_e2e_seed::seed_hal_alert_capabilities(pool).await?;
    crate::wms_api_e2e_seed_mrc::seed_mrc_data(pool).await?;
    Ok(())
}
