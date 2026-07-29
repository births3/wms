use std::error::Error;

use sqlx::PgPool;
use uuid::Uuid;

/// US-H9-008：打印组套样本数据——一个已截单的样本归集组（含发票号与商品批号）
/// 和一张待截单候选订单。权威 PDF 通过 H-FILE 种子单独写入。
pub(super) async fn seed_h9_print_suite_samples(
    pool: &PgPool,
    owner_id: Uuid,
    warehouse_id: Uuid,
    customer_id: Uuid,
    address_id: Uuid,
    actor_id: Uuid,
) -> Result<(), Box<dyn Error>> {
    let sample_order_id = Uuid::parse_str("00000000-0000-0000-0000-000000009609")?;
    let sample_group_id = Uuid::parse_str("00000000-0000-0000-0000-000000009610")?;
    let candidate_order_id = Uuid::parse_str("00000000-0000-0000-0000-000000009611")?;
    // 样本归集组的源订单：已归集，不再出现在待截单列表。
    sqlx::query(
        r#"
        INSERT INTO outbound_orders (
            id, owner_id, document_type, wms_order_no, erp_order_no,
            customer_id, delivery_address_id, delivery_address_snapshot,
            warehouse_id, invoice_no, status, created_at
        )
        VALUES ($1, $2, 'sales_outbound', 'OUT-H9-E2E-009', 'ERP-H9-E2E-009',
                $3, $4, '{}'::jsonb, $5, 'INV-H9-E2E-009', 'confirmed',
                '2026-07-26T08:10:00Z')
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(sample_order_id)
    .bind(owner_id)
    .bind(customer_id)
    .bind(address_id)
    .bind(warehouse_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO outbound_order_lines (
            id, outbound_order_id, owner_id, line_no, product_code, batch_no, planned_qty
        )
        VALUES ('00000000-0000-0000-0000-000000009612', $1, $2, 1,
                'PROD-H9-E2E', 'BATCH-H9-E2E', 10)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(sample_order_id)
    .bind(owner_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO h9_outbound_route_snapshots (
            outbound_order_id, owner_id, warehouse_id, customer_id,
            delivery_address_id, route_code, frozen_at
        )
        VALUES ($1, $2, $3, $4, $5, 'LINE-H9-E2E-006', '2026-07-26T08:10:00Z')
        ON CONFLICT (outbound_order_id) DO NOTHING
        "#,
    )
    .bind(sample_order_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(customer_id)
    .bind(address_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO h9_delivery_note_groups (
            id, owner_id, warehouse_id, customer_id, delivery_address_id,
            route_code, delivery_note_no, cutoff_mode, cutoff_reason,
            cutoff_at, created_by
        )
        VALUES ($1, $2, $3, $4, $5, 'LINE-H9-E2E-006', 'SHTX-E2E-H9-008-0001',
                'manual', 'US-H9-008 组套样本种子', '2026-07-26T09:00:00Z', $6)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(sample_group_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(customer_id)
    .bind(address_id)
    .bind(actor_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO h9_delivery_note_group_orders (
            group_id, owner_id, outbound_order_id, warehouse_id,
            customer_id, delivery_address_id, route_code
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'LINE-H9-E2E-006')
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(sample_group_id)
    .bind(owner_id)
    .bind(sample_order_id)
    .bind(warehouse_id)
    .bind(customer_id)
    .bind(address_id)
    .execute(pool)
    .await?;
    // 组套发布后由页面人工截单生成组套实例的候选订单。
    sqlx::query(
        r#"
        INSERT INTO outbound_orders (
            id, owner_id, document_type, wms_order_no, erp_order_no,
            customer_id, delivery_address_id, delivery_address_snapshot,
            warehouse_id, invoice_no, status, created_at
        )
        VALUES ($1, $2, 'sales_outbound', 'OUT-H9-E2E-010', 'ERP-H9-E2E-010',
                $3, $4, '{}'::jsonb, $5, 'INV-H9-E2E-010', 'confirmed',
                '2026-07-26T08:15:00Z')
        ON CONFLICT (id) DO UPDATE
        SET invoice_no = EXCLUDED.invoice_no,
            status = CASE
            WHEN EXISTS (
                SELECT 1 FROM h9_delivery_note_group_orders
                 WHERE owner_id = EXCLUDED.owner_id AND outbound_order_id = EXCLUDED.id
            ) THEN outbound_orders.status
            ELSE 'confirmed'
        END
        "#,
    )
    .bind(candidate_order_id)
    .bind(owner_id)
    .bind(customer_id)
    .bind(address_id)
    .bind(warehouse_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO h9_outbound_route_snapshots (
            outbound_order_id, owner_id, warehouse_id, customer_id,
            delivery_address_id, route_code, frozen_at
        )
        VALUES ($1, $2, $3, $4, $5, 'LINE-H9-E2E-006', '2026-07-26T08:15:00Z')
        ON CONFLICT (outbound_order_id) DO NOTHING
        "#,
    )
    .bind(candidate_order_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(customer_id)
    .bind(address_id)
    .execute(pool)
    .await?;
    Ok(())
}
