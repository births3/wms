//! LPN 整托上架真实 E2E 种子：待上架 ASN + 两批验收数量。
use std::error::Error;

use sqlx::PgPool;

pub async fn seed_lpn_putaway_order(pool: &PgPool) -> Result<(), Box<dyn Error>> {
    sqlx::query(
        r#"
        INSERT INTO receiving_orders (
            id, owner_id, receipt_no, document_type, warehouse_id,
            erp_bill_id, erp_bill_code, erp_revision, erp_line_no, erp_correlation_id,
            status, expected_arrival_at
        ) VALUES (
            '00000000-0000-0000-0000-000000001901',
            '00000000-0000-0000-0000-000000000001',
            'ASN-LPN-E2E-001',
            'purchase_inbound',
            '00000000-0000-0000-0000-000000001301',
            9101, 'ERP-LPN-E2E-001', 1, 1, 'corr-lpn-e2e-001',
            'putaway', now()
        )
        ON CONFLICT (owner_id, receipt_no) DO UPDATE
        SET status = 'putaway',
            warehouse_id = EXCLUDED.warehouse_id,
            updated_at = now()
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO receiving_order_lines (
            id, receiving_order_id, owner_id, line_no, product_id, product_code,
            expected_qty, batch_no, production_date, expiry_date
        ) VALUES
            (
                '00000000-0000-0000-0000-000000001911',
                '00000000-0000-0000-0000-000000001901',
                '00000000-0000-0000-0000-000000000001',
                1, '00000000-0000-0000-0000-000000001001', 'P-M1-E2E-001',
                10, 'B-LPN-E2E-001', '2026-01-01', '2028-01-01'
            ),
            (
                '00000000-0000-0000-0000-000000001912',
                '00000000-0000-0000-0000-000000001901',
                '00000000-0000-0000-0000-000000000001',
                2, '00000000-0000-0000-0000-000000001001', 'P-M1-E2E-001',
                10, 'B-LPN-E2E-002', '2026-01-01', '2028-01-01'
            )
        ON CONFLICT (receiving_order_id, line_no) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO receiving_inspections (
            id, receiving_order_id, owner_id, batch_no, accepted_qty, rejected_qty,
            production_date, expiry_date, quality_status, occurred_at
        ) VALUES
            (
                '00000000-0000-0000-0000-000000001921',
                '00000000-0000-0000-0000-000000001901',
                '00000000-0000-0000-0000-000000000001',
                'B-LPN-E2E-001', 10, 0, '2026-01-01', '2028-01-01', 'qualified', now()
            ),
            (
                '00000000-0000-0000-0000-000000001922',
                '00000000-0000-0000-0000-000000001901',
                '00000000-0000-0000-0000-000000000001',
                'B-LPN-E2E-002', 10, 0, '2026-01-01', '2028-01-01', 'qualified', now()
            )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}
