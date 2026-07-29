use super::*;
use wms_api::reconciliation_query::ReconciliationItemQuery;

#[sqlx::test(migrations = "../../migrations")]
async fn reconciliation_item_query_uses_stable_keyset_pages(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();
    let actor = ctx(owner_id);
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name)
         VALUES ($1,$2,'RC keyset 货主')",
    )
    .bind(owner_id)
    .bind(format!("RC-PAGE-{}", &owner_id.simple().to_string()[..8]))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO reconciliation_runs
         (id, owner_id, window_key, request_hash, snapshot_at, created_by)
         VALUES ($1,$2,'keyset','hash',now(),$3)",
    )
    .bind(run_id)
    .bind(owner_id)
    .bind(actor.user_id)
    .execute(&pool)
    .await
    .unwrap();
    let created_at = Utc::now();
    let ids = [
        Uuid::parse_str("30000000-0000-0000-0000-000000000003").unwrap(),
        Uuid::parse_str("30000000-0000-0000-0000-000000000002").unwrap(),
        Uuid::parse_str("30000000-0000-0000-0000-000000000001").unwrap(),
    ];
    for (index, id) in ids.iter().enumerate() {
        sqlx::query(
            "INSERT INTO reconciliation_items
             (id, owner_id, run_id, product_code, batch_no, wms_qty, erp_qty,
              difference_qty, difference_type, resolution_status, created_at, updated_at)
             VALUES ($1,$2,$3,$4,'B1',2,1,1,'wms_more','open',$5,$5)",
        )
        .bind(id)
        .bind(owner_id)
        .bind(run_id)
        .bind(format!("P-{index}"))
        .bind(created_at)
        .execute(&pool)
        .await
        .unwrap();
    }
    let repository = PgReconciliationRepository::new(pool);
    let first = repository
        .list_items(
            &actor,
            ReconciliationItemQuery {
                limit: Some(2),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(
        first.data.iter().map(|item| item.id).collect::<Vec<_>>(),
        ids[..2]
    );
    let second = repository
        .list_items(
            &actor,
            ReconciliationItemQuery {
                cursor: first.page.next_cursor,
                limit: Some(2),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(
        second.data.iter().map(|item| item.id).collect::<Vec<_>>(),
        ids[2..]
    );
    assert!(second.page.next_cursor.is_none());
    assert_eq!(
        repository
            .list_items(
                &actor,
                ReconciliationItemQuery {
                    cursor: Some("not-a-cursor".to_string()),
                    ..Default::default()
                },
            )
            .await
            .unwrap_err(),
        ReconciliationError::InvalidRequest
    );
}
