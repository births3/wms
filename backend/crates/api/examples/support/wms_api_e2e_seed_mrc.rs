//! M-RC real-data E2E seed.

use std::error::Error;

use sqlx::PgPool;

pub async fn seed_mrc_data(pool: &PgPool) -> Result<(), Box<dyn Error>> {
    sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification, dosage_form,
            storage_condition, special_drug_category, approval_no, manufacturer, status
        )
        VALUES (
            '00000000-0000-0000-0000-00000000c401',
            '00000000-0000-0000-0000-000000000001',
            'P-RC-E2E-MULTI',
            'M-RC 多库存批次验收商品',
            '1mg*10片',
            '片剂',
            'normal',
            'none',
            '国药准字RC-E2E-001',
            'M-RC 验收药业',
            'active'
        )
        ON CONFLICT (owner_id, product_code) DO NOTHING
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
        VALUES
            (
                '00000000-0000-0000-0000-00000000c411',
                '00000000-0000-0000-0000-000000000001',
                '00000000-0000-0000-0000-000000001301',
                '00000000-0000-0000-0000-000000001302',
                'RC-E2E-01', 90, 1, 1, 1000000, 0, 1, 'storage', NULL, 'available'
            ),
            (
                '00000000-0000-0000-0000-00000000c412',
                '00000000-0000-0000-0000-000000000001',
                '00000000-0000-0000-0000-000000001301',
                '00000000-0000-0000-0000-000000001302',
                'RC-E2E-02', 90, 2, 1, 1000000, 0, 1, 'storage', NULL, 'available'
            )
        ON CONFLICT (owner_id, location_code) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_locked, quality_status, location_id, location_code, container_lpn
        )
        VALUES
            (
                '00000000-0000-0000-0000-00000000c421',
                '00000000-0000-0000-0000-000000000001',
                'P-RC-E2E-MULTI', 'B-RC-E2E-MULTI', '2026-01-01', '2028-01-01',
                10, 0, 'qualified',
                '00000000-0000-0000-0000-00000000c411', 'RC-E2E-01', 'LPN-RC-E2E-01'
            ),
            (
                '00000000-0000-0000-0000-00000000c422',
                '00000000-0000-0000-0000-000000000001',
                'P-RC-E2E-MULTI', 'B-RC-E2E-MULTI', '2026-01-01', '2028-01-01',
                10, 0, 'qualified',
                '00000000-0000-0000-0000-00000000c412', 'RC-E2E-02', 'LPN-RC-E2E-02'
            )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO reconciliation_runs (
            id, owner_id, window_key, request_hash, snapshot_at, matched_count, wms_more_count,
            erp_more_count, created_by, created_at
        )
        VALUES (
            '00000000-0000-0000-0000-00000000c301',
            '00000000-0000-0000-0000-000000000001',
            'e2e-2026-07-23T18',
            'e2e-seed-request-hash',
            now(),
            0,
            3,
            1,
            '00000000-0000-0000-0000-000000000101',
            now()
        )
        ON CONFLICT (owner_id, window_key) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO reconciliation_items (
            id, owner_id, run_id, product_code, batch_no, wms_qty, erp_qty,
            difference_qty, difference_type, resolution_status, created_at, updated_at
        )
        VALUES
            (
                '00000000-0000-0000-0000-00000000c311',
                '00000000-0000-0000-0000-000000000001',
                '00000000-0000-0000-0000-00000000c301',
                'P-M1-E2E-001',
                'B-M4-E2E-001',
                10,
                8,
                2,
                'wms_more',
                'open',
                now(),
                now()
            ),
            (
                '00000000-0000-0000-0000-00000000c312',
                '00000000-0000-0000-0000-000000000001',
                '00000000-0000-0000-0000-00000000c301',
                'P-RC-E2E-ERP',
                'B-RC-E2E-ERP',
                0,
                3,
                -3,
                'erp_more',
                'open',
                now(),
                now()
            ),
            (
                '00000000-0000-0000-0000-00000000c313',
                '00000000-0000-0000-0000-000000000001',
                '00000000-0000-0000-0000-00000000c301',
                'P-RC-E2E-MULTI',
                'B-RC-E2E-MULTI',
                20,
                17,
                3,
                'wms_more',
                'open',
                now(),
                now()
            ),
            (
                '00000000-0000-0000-0000-00000000c314',
                '00000000-0000-0000-0000-000000000001',
                '00000000-0000-0000-0000-00000000c301',
                'P-RC-E2E-EXCEPTION',
                'B-RC-E2E-EXCEPTION',
                5,
                4,
                1,
                'wms_more',
                'exception',
                now(),
                now()
            )
        ON CONFLICT (run_id, product_code, batch_no) DO NOTHING;
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO reconciliation_runs (
            id, owner_id, window_key, request_hash, snapshot_at, matched_count, wms_more_count,
            erp_more_count, created_by, created_at
        )
        VALUES (
            '00000000-0000-0000-0000-00000000c302',
            '00000000-0000-0000-0000-000000000001',
            'e2e-keyset-pagination',
            'e2e-keyset-pagination-hash',
            '2026-07-20T00:00:00Z',
            0,
            51,
            0,
            '00000000-0000-0000-0000-000000000101',
            '2026-07-20T00:00:00Z'
        )
        ON CONFLICT (owner_id, window_key) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO reconciliation_items (
            id, owner_id, run_id, product_code, batch_no, wms_qty, erp_qty,
            difference_qty, difference_type, resolution_status, disposition,
            resolved_by, resolved_at, created_at, updated_at
        )
        SELECT
            (
                '00000000-0000-0000-2000-'
                || lpad(series_no::text, 12, '0')
            )::UUID,
            '00000000-0000-0000-0000-000000000001',
            '00000000-0000-0000-0000-00000000c302',
            'P-RC-PAGE-' || lpad(series_no::text, 3, '0'),
            'B-RC-PAGE',
            2,
            1,
            1,
            'wms_more',
            'resolved',
            'known_difference',
            '00000000-0000-0000-0000-000000000101',
            '2026-07-20T00:00:00Z',
            '2026-07-20T00:00:00Z',
            '2026-07-20T00:00:00Z'
        FROM generate_series(1, 51) AS series_no
        ON CONFLICT (run_id, product_code, batch_no) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}
