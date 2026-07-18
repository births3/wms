use chrono::{Duration, NaiveDate, Utc};
use uuid::Uuid;
use wms_api::auth::AuthContext;
use wms_api::wave3_repository::PgWave3Repository;
use wms_domain::LocationHistoryQuery;

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "location-history-test".to_string(),
        permissions: vec!["m3.read".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn location_history_is_owner_scoped_and_filters_by_location_code(pool: sqlx::PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let location_code = "A01-01-02-03";
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_locked, quality_status, location_id, location_code
        ) VALUES ($1, $2, 'P-LOC-001', 'B-LOC-001', $3, $4, 10, 0, 'qualified', $5, $6)
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("production"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("expiry"))
    .bind(Uuid::new_v4())
    .bind(location_code)
    .execute(&pool)
    .await
    .expect("batch");

    sqlx::query(
        r#"
        INSERT INTO inventory_movements (
            id, owner_id, batch_id, movement_type, qty_delta,
            source_document_type, source_document_id, occurred_at,
            location_code, to_location_code, operator_name
        ) VALUES ($1, $2, $3, 'inbound_putaway', 10, 'receiving_order', $4, $5, $6, $6, 'keeper-a')
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(batch_id)
    .bind(Uuid::new_v4())
    .bind(now - Duration::hours(1))
    .bind(location_code)
    .execute(&pool)
    .await
    .expect("movement");

    sqlx::query(
        r#"
        INSERT INTO inventory_movements (
            id, owner_id, batch_id, movement_type, qty_delta,
            source_document_type, source_document_id, occurred_at,
            location_code, to_location_code
        ) VALUES ($1, $2, $3, 'inbound_putaway', 5, 'receiving_order', $4, $5, 'B02-01-01-01', 'B02-01-01-01')
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(batch_id)
    .bind(Uuid::new_v4())
    .bind(now)
    .execute(&pool)
    .await
    .expect("other location movement");

    let repository = PgWave3Repository::new(pool);
    let history = repository
        .list_location_history(
            &ctx(owner_id),
            &LocationHistoryQuery {
                location_code: Some(location_code.to_string()),
                days: Some(30),
                ..LocationHistoryQuery::default()
            },
        )
        .await
        .expect("history");
    assert_eq!(history.location_code, location_code);
    assert_eq!(history.data.len(), 1);
    assert_eq!(history.data[0].movement_type, "inbound_putaway");
    assert_eq!(history.data[0].operator_name.as_deref(), Some("keeper-a"));
    assert_eq!(history.data[0].product_code.as_deref(), Some("P-LOC-001"));
    assert!(!history.product_shares.is_empty());

    let missing = repository
        .list_location_history(
            &ctx(owner_id),
            &LocationHistoryQuery {
                location_code: Some("Z99-99-99-99".to_string()),
                ..LocationHistoryQuery::default()
            },
        )
        .await;
    assert!(missing.is_err());

    let cross_owner = repository
        .list_location_history(
            &ctx(other_owner_id),
            &LocationHistoryQuery {
                location_code: Some(location_code.to_string()),
                ..LocationHistoryQuery::default()
            },
        )
        .await;
    assert!(cross_owner.is_err());
}

#[sqlx::test(migrations = "../../migrations")]
async fn location_history_requires_location_code(pool: sqlx::PgPool) {
    let repository = PgWave3Repository::new(pool);
    let error = repository
        .list_location_history(&ctx(Uuid::new_v4()), &LocationHistoryQuery::default())
        .await
        .expect_err("missing location");
    assert!(matches!(
        error,
        wms_api::wave3_repository::Wave3RepositoryError::InvalidLocation
    ));
}
