use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    audit::AuditWriteRequest,
    auth::AuthContext,
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
};
use wms_domain::{CreateColdChainDeviceRequest, UpdateColdChainDeviceRequest};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m5-device-test".to_string(),
        permissions: vec!["m5.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn audit(ctx: &AuthContext) -> AuditWriteRequest {
    AuditWriteRequest::from_auth_context(ctx, "create_device", "M5", "cold_chain_device", "", None)
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_cold_chain_device_is_validated_idempotent_and_audited(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let req = CreateColdChainDeviceRequest {
        device_code: "CC-LEDGER-001".to_string(),
        device_type: "temperature_recorder".to_string(),
        installed_at_location_code: Some("COLD-A".to_string()),
        calibration_due_at: None,
    };

    let first = repo
        .create_cold_chain_device_with_audit(
            &ctx,
            req.clone(),
            Utc::now(),
            "m5-device-create-1",
            audit(&ctx),
        )
        .await
        .expect("create device")
        .value;
    let replay = repo
        .create_cold_chain_device_with_audit(
            &ctx,
            req.clone(),
            Utc::now(),
            "m5-device-create-1",
            audit(&ctx),
        )
        .await
        .expect("replay device")
        .value;
    assert_eq!(first.id, replay.id);

    let duplicate = repo
        .create_cold_chain_device_with_audit(
            &ctx,
            req,
            Utc::now(),
            "m5-device-create-2",
            audit(&ctx),
        )
        .await;
    assert!(matches!(
        duplicate,
        Err(Wave3RepositoryError::DuplicateCode)
    ));

    let invalid = repo
        .create_cold_chain_device_with_audit(
            &ctx,
            CreateColdChainDeviceRequest {
                device_code: "CC-LEDGER-002".to_string(),
                device_type: "probe".to_string(),
                installed_at_location_code: None,
                calibration_due_at: None,
            },
            Utc::now(),
            "m5-device-create-3",
            audit(&ctx),
        )
        .await;
    assert!(matches!(
        invalid,
        Err(Wave3RepositoryError::InvalidDeviceType)
    ));

    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM cold_chain_devices WHERE owner_id = $1), (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'create_device')",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("query device and audit counts");
    assert_eq!(counts, (1, 1));

    let updated = repo
        .update_cold_chain_device_with_audit(
            &ctx,
            "CC-LEDGER-001",
            UpdateColdChainDeviceRequest {
                device_type: Some("thermometer".to_string()),
                installed_at_location_code: Some("COLD-B".to_string()),
                calibration_due_at: None,
            },
            Utc::now(),
            "m5-device-update-1",
            audit(&ctx),
        )
        .await
        .expect("update device")
        .value;
    assert_eq!(updated.device_type, "thermometer");

    let listed = repo
        .list_cold_chain_devices(&ctx)
        .await
        .expect("list devices");
    assert_eq!(listed.len(), 1);
    assert_eq!(
        listed[0].installed_at_location_code.as_deref(),
        Some("COLD-B")
    );

    sqlx::query("UPDATE cold_chain_devices SET status = 'monitoring' WHERE id = $1")
        .bind(first.id)
        .execute(&pool)
        .await
        .expect("mark device monitoring");
    let blocked = repo
        .disable_cold_chain_device_with_audit(
            &ctx,
            "CC-LEDGER-001",
            Utc::now(),
            "m5-device-disable-1",
            audit(&ctx),
        )
        .await;
    assert!(matches!(
        blocked,
        Err(Wave3RepositoryError::ActiveMonitoring)
    ));

    sqlx::query("UPDATE cold_chain_devices SET status = 'active' WHERE id = $1")
        .bind(first.id)
        .execute(&pool)
        .await
        .expect("restore active status");
    let disabled = repo
        .disable_cold_chain_device_with_audit(
            &ctx,
            "CC-LEDGER-001",
            Utc::now(),
            "m5-device-disable-2",
            audit(&ctx),
        )
        .await
        .expect("disable device")
        .value;
    assert_eq!(disabled.status, "inactive");
}
