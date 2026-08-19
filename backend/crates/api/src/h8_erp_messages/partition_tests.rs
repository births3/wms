//! US-H8-003 AC10/AC12：真实 PostgreSQL 月分区、跨月唯一性与裁剪证据。

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "../../migrations")]
async fn monthly_partitions_preserve_global_identity_and_prune_queries(pool: PgPool) {
    sqlx::query("SET TIME ZONE 'Asia/Shanghai'")
        .execute(&pool)
        .await
        .expect("set non-UTC test timezone");
    let parents: Vec<String> = sqlx::query_scalar(
        r#"SELECT c.relname
           FROM pg_partitioned_table p
           JOIN pg_class c ON c.oid = p.partrelid
           WHERE c.relname IN ('h8_erp_messages', 'h8_erp_message_attempts')
           ORDER BY c.relname"#,
    )
    .fetch_all(&pool)
    .await
    .expect("inspect H8 partition parents");
    assert_eq!(parents, vec!["h8_erp_message_attempts", "h8_erp_messages"]);

    for month in ["2025-01-01", "2025-02-01"] {
        sqlx::query("SELECT h8_erp_messages_ensure_month_partition($1::date)")
            .bind(month)
            .execute(&pool)
            .await
            .expect("create H8 monthly partitions");
    }

    let owner_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$3)")
        .bind(owner_id)
        .bind(format!("OWNER-{owner_id}"))
        .bind("H8 partition test")
        .execute(&pool)
        .await
        .expect("insert owner");

    let january_id = Uuid::new_v4();
    let february_id = Uuid::new_v4();
    let january = timestamp("2025-01-10T08:00:00Z");
    let february = timestamp("2025-02-10T08:00:00Z");
    insert_message(&pool, owner_id, january_id, "ERP-JAN", "idem-jan", january).await;
    insert_message(
        &pool,
        owner_id,
        february_id,
        "ERP-FEB",
        "idem-feb",
        february,
    )
    .await;

    let message_partitions: Vec<String> = sqlx::query_scalar(
        "SELECT tableoid::regclass::text FROM h8_erp_messages ORDER BY created_at",
    )
    .fetch_all(&pool)
    .await
    .expect("read message partitions");
    assert_eq!(
        message_partitions,
        vec!["h8_erp_messages_202501", "h8_erp_messages_202502"]
    );

    let app_can_read_registry: bool = sqlx::query_scalar(
        "SELECT has_table_privilege('wms_app','h8_erp_message_registry','SELECT')",
    )
    .fetch_one(&pool)
    .await
    .expect("inspect runtime role registry grant");
    assert!(app_can_read_registry);

    let duplicate = sqlx::query(
        r#"INSERT INTO h8_erp_messages
           (id, owner_id, direction, message_type, channel, external_ref,
            idempotency_key, correlation_id, payload_digest, created_at, updated_at)
           VALUES ($1,$2,'inbound','asn','interface_table','ERP-JAN','idem-jan',
                   'duplicate','digest',$3,$3)"#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(february)
    .execute(&pool)
    .await
    .expect_err("idempotency identity must stay unique across months");
    assert_eq!(
        duplicate.as_database_error().and_then(|error| error.code()),
        Some("23505".into())
    );

    for (attempt_id, started_at) in [(Uuid::new_v4(), january), (Uuid::new_v4(), february)] {
        sqlx::query(
            r#"INSERT INTO h8_erp_message_attempts
               (id, message_id, owner_id, attempt_no, channel, started_at, finished_at,
                result, actor)
               VALUES ($1,$2,$3,$4,'interface_table',$5,$5,'failed','partition-test')"#,
        )
        .bind(attempt_id)
        .bind(january_id)
        .bind(owner_id)
        .bind(if started_at == january { 1 } else { 2 })
        .bind(started_at)
        .execute(&pool)
        .await
        .expect("insert partitioned attempt");
    }
    let attempt_partitions: Vec<String> = sqlx::query_scalar(
        "SELECT tableoid::regclass::text FROM h8_erp_message_attempts ORDER BY started_at",
    )
    .fetch_all(&pool)
    .await
    .expect("read attempt partitions");
    assert_eq!(
        attempt_partitions,
        vec![
            "h8_erp_message_attempts_202501",
            "h8_erp_message_attempts_202502"
        ]
    );

    let duplicate_attempt = sqlx::query(
        r#"INSERT INTO h8_erp_message_attempts
           (id, message_id, owner_id, attempt_no, channel, started_at, finished_at,
            result, actor)
           VALUES ($1,$2,$3,1,'interface_table',$4,$4,'failed','partition-test')"#,
    )
    .bind(Uuid::new_v4())
    .bind(january_id)
    .bind(owner_id)
    .bind(february)
    .execute(&pool)
    .await
    .expect_err("attempt number must stay unique across months");
    assert_eq!(
        duplicate_attempt
            .as_database_error()
            .and_then(|error| error.code()),
        Some("23505".into())
    );

    let plan = sqlx::query_scalar::<_, String>(
        r#"EXPLAIN (COSTS OFF)
           SELECT id FROM h8_erp_messages
           WHERE created_at >= TIMESTAMPTZ '2025-01-01T00:00:00Z'
             AND created_at < TIMESTAMPTZ '2025-02-01T00:00:00Z'"#,
    )
    .fetch_all(&pool)
    .await
    .expect("explain partition-pruned query")
    .join("\n");
    assert!(plan.contains("h8_erp_messages_202501"), "{plan}");
    assert!(!plan.contains("h8_erp_messages_202502"), "{plan}");
}

async fn insert_message(
    pool: &PgPool,
    owner_id: Uuid,
    id: Uuid,
    external_ref: &str,
    idempotency_key: &str,
    created_at: DateTime<Utc>,
) {
    sqlx::query(
        r#"INSERT INTO h8_erp_messages
           (id, owner_id, direction, message_type, channel, external_ref,
            idempotency_key, correlation_id, payload_digest, created_at, updated_at)
           VALUES ($1,$2,'inbound','asn','interface_table',$3,$4,$4,'digest',$5,$5)"#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(external_ref)
    .bind(idempotency_key)
    .bind(created_at)
    .execute(pool)
    .await
    .expect("insert H8 message");
}

fn timestamp(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .expect("valid test timestamp")
        .with_timezone(&Utc)
}

#[sqlx::test(migrations = "../../migrations")]
async fn daily_stats_snapshot_tracks_status_retry_and_retention_delete(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let message_id = Uuid::new_v4();
    let created_at = timestamp("2025-01-31T20:00:00Z");
    sqlx::query("SET TIME ZONE 'Asia/Shanghai'")
        .execute(&pool)
        .await
        .expect("set non-UTC test timezone");
    sqlx::query("SELECT h8_erp_messages_ensure_month_partition(DATE '2025-01-01')")
        .execute(&pool)
        .await
        .expect("create January partition");
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$3)")
        .bind(owner_id)
        .bind(format!("OWNER-{owner_id}"))
        .bind("H8 stats snapshot test")
        .execute(&pool)
        .await
        .expect("insert owner");
    sqlx::query(
        r#"INSERT INTO h8_erp_messages
           (id, owner_id, connector_code, direction, message_type, channel, external_ref,
            idempotency_key, correlation_id, sync_status, payload_digest, created_at, updated_at)
           VALUES ($1,$2,'SELF-ERP','inbound','asn','interface_table','ERP-SNAPSHOT',
                   'idem-snapshot','corr-snapshot','pending','digest',$3,$3)"#,
    )
    .bind(message_id)
    .bind(owner_id)
    .bind(created_at)
    .execute(&pool)
    .await
    .expect("insert pending message");

    sqlx::query("UPDATE h8_erp_messages SET sync_status='succeeded', retry_count=2 WHERE id=$1")
        .bind(message_id)
        .execute(&pool)
        .await
        .expect("finish message");
    let counts: (i64, i64, i64, i64) = sqlx::query_as(
        r#"SELECT total, succeeded, pending, retry_total
           FROM h8_erp_message_stats_daily
           WHERE owner_id=$1 AND connector_code='SELF-ERP'
             AND channel='interface_table' AND message_type='asn'
             AND stat_date=DATE '2025-01-31'"#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("read daily snapshot");
    assert_eq!(counts, (1, 1, 0, 2));

    sqlx::query("DELETE FROM h8_erp_messages WHERE id=$1")
        .bind(message_id)
        .execute(&pool)
        .await
        .expect("delete retained message");
    let remaining: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total),0)::bigint FROM h8_erp_message_stats_daily WHERE owner_id=$1",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("read snapshot after delete");
    assert_eq!(remaining, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn partition_identity_is_immutable_and_attempts_stay_append_only(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let message_id = Uuid::new_v4();
    let attempt_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$3)")
        .bind(owner_id)
        .bind(format!("OWNER-{owner_id}"))
        .bind("H8 immutable partition test")
        .execute(&pool)
        .await
        .expect("insert owner");
    sqlx::query(
        r#"INSERT INTO h8_erp_messages
           (id, owner_id, direction, message_type, channel, external_ref,
            idempotency_key, correlation_id, payload_digest)
           VALUES ($1,$2,'inbound','asn','interface_table','ERP-IMMUTABLE',
                   'idem-immutable','corr-immutable','digest')"#,
    )
    .bind(message_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("insert message");
    sqlx::query(
        r#"INSERT INTO h8_erp_message_attempts
           (id, message_id, owner_id, attempt_no, channel, started_at, result, actor)
           VALUES ($1,$2,$3,1,'interface_table',now(),'claimed','worker')"#,
    )
    .bind(attempt_id)
    .bind(message_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("insert attempt");

    sqlx::query("UPDATE h8_erp_messages SET idempotency_key='changed' WHERE id=$1")
        .bind(message_id)
        .execute(&pool)
        .await
        .expect_err("message identity must not move away from its registry");
    sqlx::query("UPDATE h8_erp_message_attempts SET result='failed' WHERE id=$1")
        .bind(attempt_id)
        .execute(&pool)
        .await
        .expect_err("attempt rows must remain append-only");

    let other_owner = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$3)")
        .bind(other_owner)
        .bind(format!("OWNER-{other_owner}"))
        .bind("H8 cross-owner attempt test")
        .execute(&pool)
        .await
        .expect("insert other owner");
    sqlx::query(
        r#"INSERT INTO h8_erp_message_attempts
           (id, message_id, owner_id, attempt_no, channel, started_at, result, actor)
           VALUES ($1,$2,$3,2,'interface_table',now(),'claimed','worker')"#,
    )
    .bind(Uuid::new_v4())
    .bind(message_id)
    .bind(other_owner)
    .execute(&pool)
    .await
    .expect_err("attempt owner must match its message owner");
}
