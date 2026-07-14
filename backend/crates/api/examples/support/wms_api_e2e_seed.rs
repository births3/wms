use sqlx::PgPool;
use uuid::Uuid;

pub async fn seed_mvr_matrix_approver(
    pool: &PgPool,
    password_hash: &str,
    system_admin_role_id: Uuid,
) -> Result<(), sqlx::Error> {
    let owner_id = Uuid::from_u128(1);
    let user_id = Uuid::from_u128(0x103);
    sqlx::query(
        r#"
        INSERT INTO auth_users (id, username, display_name, password_hash, status)
        VALUES ($1, 'mvr-matrix-approver', 'M-VR 矩阵确认人', $2, 'active')
        ON CONFLICT (id) DO UPDATE
        SET display_name = EXCLUDED.display_name,
            password_hash = EXCLUDED.password_hash,
            status = 'active',
            updated_at = now()
        "#,
    )
    .bind(user_id)
    .bind(password_hash)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary)
        VALUES ($1, $2, TRUE, FALSE)
        ON CONFLICT (user_id, owner_id) DO UPDATE SET is_active = TRUE
        "#,
    )
    .bind(user_id)
    .bind(owner_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO auth_user_roles (user_id, owner_id, role_id)
        VALUES ($1, $2, $3)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(owner_id)
    .bind(system_admin_role_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn seed_quality_approver(pool: &PgPool, password_hash: &str) -> Result<(), sqlx::Error> {
    let owner_id = Uuid::from_u128(1);
    let user_id = Uuid::from_u128(0x201);
    let role_id = Uuid::from_u128(0x202);
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, 'm3-quality-approver', 'M3 质量审批人', $2, 'active') ON CONFLICT (id) DO UPDATE SET status = 'active', updated_at = now()",
    )
    .bind(user_id)
    .bind(password_hash)
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, FALSE) ON CONFLICT (user_id, owner_id) DO UPDATE SET is_active = TRUE",
    )
    .bind(user_id)
    .bind(owner_id)
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO auth_roles (id, owner_id, role_code, role_name) VALUES ($1, $2, 'm3_quality_approver', 'M3 质量审批人') ON CONFLICT (owner_id, lower(role_code)) DO UPDATE SET role_name = EXCLUDED.role_name",
    )
    .bind(role_id)
    .bind(owner_id)
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO auth_user_roles (user_id, owner_id, role_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .bind(owner_id)
    .bind(role_id)
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO auth_role_permissions (role_id, permission_id) SELECT $1, id FROM auth_permissions WHERE permission_code = 'm3.recall.approve' ON CONFLICT DO NOTHING",
    )
    .bind(role_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// 为真实管理端 E2E 提供 M9/M10 所需的权限和最小关联业务数据。
pub async fn seed_m9_m10_capabilities(pool: &PgPool) -> Result<(), sqlx::Error> {
    let owner_id = Uuid::from_u128(1);
    let admin_role_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM auth_roles WHERE owner_id = $1 AND lower(role_code) = 'system_admin'",
    )
    .bind(owner_id)
    .fetch_one(pool)
    .await?;
    for (id, code, name) in [
        (Uuid::from_u128(0x128), "m9.billing.read", "M9 计费读取"),
        (Uuid::from_u128(0x129), "m9.write", "M9 计费维护"),
        (Uuid::from_u128(0x12a), "m10.tms.read", "M10 TMS 读取"),
        (Uuid::from_u128(0x12b), "m10.write", "M10 TMS 写入"),
    ] {
        let permission_id: Uuid = sqlx::query_scalar(
            "INSERT INTO auth_permissions (id, permission_code, permission_name) VALUES ($1, $2, $3) ON CONFLICT (lower(permission_code)) DO UPDATE SET permission_name = EXCLUDED.permission_name RETURNING id",
        )
        .bind(id)
        .bind(code)
        .bind(name)
        .fetch_one(pool)
        .await?;
        sqlx::query(
            "INSERT INTO auth_role_permissions (role_id, permission_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(admin_role_id)
        .bind(permission_id)
        .execute(pool)
        .await?;
    }

    sqlx::query(
        r#"
        INSERT INTO outbound_orders (
            id, owner_id, wms_order_no, erp_order_no, customer_id, warehouse_id,
            required_ship_at, status, short_pick, document_type
        )
        VALUES (
            '00000000-0000-0000-0000-000000001701', $1, 'OUT-M10-E2E-001', 'ERP-M10-E2E-001',
            '00000000-0000-0000-0000-000000001201', '00000000-0000-0000-0000-000000001301',
            '2026-07-14T12:00:00Z', 'created', FALSE, 'sales_outbound'
        )
        ON CONFLICT (owner_id, wms_order_no) DO UPDATE
        SET customer_id = EXCLUDED.customer_id,
            warehouse_id = EXCLUDED.warehouse_id,
            status = EXCLUDED.status,
            short_pick = EXCLUDED.short_pick,
            document_type = EXCLUDED.document_type,
            updated_at = now()
        "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// 为真实管理端 E2E 提供一条可复核的已拣货出库单。
pub async fn seed_m4_review_data(pool: &PgPool) -> Result<(), sqlx::Error> {
    let owner_id = Uuid::from_u128(1);
    sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification,
            storage_condition, special_drug_category, status
        )
        VALUES (
            '00000000-0000-0000-0000-000000001704', $1,
            'P-M4-REVIEW-E2E-001', 'M4 复核策略 E2E 商品', '1 unit',
            'normal', 'none', 'active'
        )
        ON CONFLICT (owner_id, product_code) DO UPDATE
        SET special_drug_category = 'none', status = 'active', updated_at = now()
        "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO auth_users (id, username, display_name, password_hash, status)
        VALUES ('00000000-0000-4000-8000-000000000104', 'm4-review-second', 'M4 第二复核员', 'test-hash', 'active')
        ON CONFLICT (id) DO UPDATE SET status = 'active', updated_at = now()
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary)
        VALUES ('00000000-0000-4000-8000-000000000104', $1, TRUE, FALSE)
        ON CONFLICT (user_id, owner_id) DO UPDATE SET is_active = TRUE
        "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO auth_user_roles (user_id, owner_id, role_id)
        SELECT '00000000-0000-4000-8000-000000000104', $1, id
          FROM auth_roles
         WHERE owner_id = $1 AND role_code = 'custodian'
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO outbound_orders (
            id, owner_id, document_type, wms_order_no, erp_order_no, customer_id,
            warehouse_id, required_ship_at, status, short_pick
        )
        VALUES (
            '00000000-0000-0000-0000-000000001702', $1, 'sales_outbound',
            'OUT-M4-REVIEW-E2E-001', 'ERP-M4-REVIEW-E2E-001',
            '00000000-0000-0000-0000-000000001201',
            '00000000-0000-0000-0000-000000001301',
            '2026-07-14T12:00:00Z', 'picked', FALSE
        )
        ON CONFLICT (id) DO UPDATE
        SET status = EXCLUDED.status,
            short_pick = EXCLUDED.short_pick,
            updated_at = now()
        "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO outbound_order_lines (
            id, outbound_order_id, owner_id, line_no, product_code,
            batch_no, planned_qty, picked_qty, reviewed_qty, shipped_qty,
            short_pick_qty
        )
        VALUES (
            '00000000-0000-0000-0000-000000001703',
            '00000000-0000-0000-0000-000000001702', $1, 1,
            'P-M4-REVIEW-E2E-001', 'B-M4-REVIEW-E2E-001', 8, 8, 0, 0, 0
        )
        ON CONFLICT (id) DO UPDATE
        SET planned_qty = EXCLUDED.planned_qty,
            picked_qty = EXCLUDED.picked_qty,
            reviewed_qty = EXCLUDED.reviewed_qty,
            shipped_qty = EXCLUDED.shipped_qty,
            short_pick_qty = EXCLUDED.short_pick_qty
        "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await?;
    Ok(())
}
