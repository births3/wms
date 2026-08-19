use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
};
use wms_domain::InventoryBatch;

fn context(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "expiry-report-test".into(),
        permissions: vec!["m3.read".into()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn keys(data: &[InventoryBatch]) -> Vec<String> {
    data.iter()
        .map(|row| format!("{}|{}|{}", row.expiry_date, row.product_code, row.batch_no))
        .collect()
}

async fn seed_policy(pool: &PgPool, owner_id: Uuid) {
    sqlx::query(r#"WITH category AS (INSERT INTO system_dictionary_categories (dict_code, dict_name, enabled, control_level, param_schema, scope_mode, override_policy, sort_order, remark) VALUES ('inventory_policy', '库存管理参数', TRUE, 'controlled', '{"required":["warning_days"],"properties":{"warning_days":{"type":"integer","minimum":1,"maximum":3650}}}'::jsonb, 'owner_override', '{"allowed_owner_params":["warning_days"]}'::jsonb, 45, 'test') ON CONFLICT (dict_code) DO UPDATE SET enabled = TRUE RETURNING dict_code) INSERT INTO system_dictionary_items (id, dict_code, item_code, item_name, enabled, owner_id, params, source, created_at, updated_at) SELECT v.id, category.dict_code, 'expiry_warning_days', '近效期预警天数', TRUE, v.owner_id, jsonb_build_object('warning_days', v.days), v.source, now(), now() FROM category CROSS JOIN (VALUES ($1::uuid, NULL::uuid, 7, 'global'), ($2::uuid, $3::uuid, 5, 'owner')) AS v(id, owner_id, days, source) ON CONFLICT (dict_code, item_code, COALESCE(owner_id, '00000000-0000-0000-0000-000000000000'::uuid)) DO UPDATE SET enabled = TRUE, params = EXCLUDED.params, updated_at = now()"#)
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .execute(pool)
        .await
        .expect("inventory policy should seed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn near_expiry_report_covers_policy_bounds_scope_order_and_input(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    seed_policy(&pool, owner_id).await;
    sqlx::query(
        r#"INSERT INTO inventory_batches
            (id, owner_id, product_code, batch_no, production_date, expiry_date,
             qty_on_hand, qty_frozen, status, location_id, location_code)
        VALUES
            ('00000000-0000-0000-0000-000000000001', $1, 'P-A', 'B-2', '2026-01-01', '2026-07-01', 3, 0, 'qualified', '00000000-0000-0000-0000-000000000011', 'P-A-B-2'),
            ('00000000-0000-0000-0000-000000000002', $1, 'P-A', 'B-1', '2026-01-01', '2026-07-01', 1, 0, 'qualified', '00000000-0000-0000-0000-000000000012', 'P-A-B-1'),
            ('00000000-0000-0000-0000-000000000003', $1, 'P-B', 'B-1', '2026-01-01', '2026-07-01', 2, 0, 'qualified', '00000000-0000-0000-0000-000000000013', 'P-B-B-1'),
            ('00000000-0000-0000-0000-000000000004', $1, 'P-Z', 'B-9', '2026-01-01', '2026-07-06', 4, 0, 'qualified', '00000000-0000-0000-0000-000000000014', 'P-Z-B-9'),
            ('00000000-0000-0000-0000-000000000005', $1, 'P-E', 'B-E', '2026-01-01', '2026-06-30', 9, 0, 'qualified', '00000000-0000-0000-0000-000000000015', 'P-E-B-E'),
            ('00000000-0000-0000-0000-000000000006', $1, 'P-O', 'B-O', '2026-01-01', '2026-07-07', 9, 0, 'qualified', '00000000-0000-0000-0000-000000000016', 'P-O-B-O'),
            ('00000000-0000-0000-0000-000000000007', $2, 'P-OTHER', 'G-7', '2026-01-01', '2026-07-08', 8, 0, 'qualified', '00000000-0000-0000-0000-000000000017', 'P-OTHER-G-7'),
            ('00000000-0000-0000-0000-000000000008', $2, 'P-OTHER', 'G-8', '2026-01-01', '2026-07-09', 8, 0, 'qualified', '00000000-0000-0000-0000-000000000018', 'P-OTHER-G-8')"#,
    )
    .bind(owner_id)
    .bind(other_owner_id)
    .execute(&pool)
    .await
    .expect("inventory batches should seed");

    let repo = PgWave3Repository::new(pool);
    let as_of = NaiveDate::from_ymd_opt(2026, 7, 1).expect("as_of");
    assert_eq!(
        keys(
            &repo
                .list_near_expiry_batches(&context(owner_id), as_of, None)
                .await
                .expect("owner report")
        ),
        [
            "2026-07-01|P-A|B-1",
            "2026-07-01|P-A|B-2",
            "2026-07-01|P-B|B-1",
            "2026-07-06|P-Z|B-9",
        ]
        .map(str::to_string)
    );
    assert_eq!(
        keys(
            &repo
                .list_near_expiry_batches(&context(other_owner_id), as_of, None)
                .await
                .expect("global fallback report")
        ),
        ["2026-07-08|P-OTHER|G-7"].map(str::to_string)
    );
    assert!(matches!(
        repo.list_near_expiry_batches(&context(owner_id), as_of, Some(0))
            .await,
        Err(Wave3RepositoryError::InvalidQuantity)
    ));
    assert!(matches!(
        repo.list_near_expiry_batches(&context(owner_id), NaiveDate::MAX, Some(1))
            .await,
        Err(Wave3RepositoryError::InvalidDate(_))
    ));
}
