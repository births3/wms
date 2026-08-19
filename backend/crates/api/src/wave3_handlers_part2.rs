use super::InventoryBatchListQuery;

async fn seed_receiving_verifiers(pool: &sqlx::PgPool, owner_id: uuid::Uuid, user_ids: &[uuid::Uuid]) {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '收货验收处理器测试货主') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(format!("M2-HANDLER-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed handler verifier owner");
    sqlx::query(
        "INSERT INTO auth_permissions (id, permission_code, permission_name) VALUES ($1, 'm2.write', '收货写入') ON CONFLICT DO NOTHING",
    )
    .bind(uuid::Uuid::new_v4())
    .execute(pool)
    .await
    .expect("seed handler verifier permission");
    sqlx::query(
        "INSERT INTO auth_roles (id, owner_id, role_code, role_name) VALUES ($1, $2, 'receiving_clerk', '收货员（验收岗）') ON CONFLICT DO NOTHING",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("seed handler verifier role");
    let role_id: uuid::Uuid = sqlx::query_scalar(
        "SELECT id FROM auth_roles WHERE owner_id = $1 AND role_code = 'receiving_clerk'",
    )
    .bind(owner_id)
    .fetch_one(pool)
    .await
    .expect("find handler verifier role");
    sqlx::query(
        "INSERT INTO auth_role_permissions (role_id, permission_id) SELECT $1, id FROM auth_permissions WHERE permission_code = 'm2.write' ON CONFLICT DO NOTHING",
    )
    .bind(role_id)
    .execute(pool)
    .await
    .expect("grant handler verifier permission");
    for (index, user_id) in user_ids.iter().enumerate() {
        sqlx::query(
            "INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, $3, 'test-hash', 'active') ON CONFLICT (id) DO NOTHING",
        )
        .bind(*user_id)
        .bind(format!("m2-handler-verifier-{index}-{}", &user_id.to_string()[..8]))
        .bind(format!("收货验收处理器测试员 {index}"))
        .execute(pool)
        .await
        .expect("seed handler verifier user");
        sqlx::query(
            "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, $3) ON CONFLICT (user_id, owner_id) DO UPDATE SET is_active = TRUE",
        )
        .bind(*user_id)
        .bind(owner_id)
        .bind(index == 0)
        .execute(pool)
        .await
        .expect("bind handler verifier");
        sqlx::query(
            "INSERT INTO auth_user_roles (user_id, owner_id, role_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(*user_id)
        .bind(owner_id)
        .bind(role_id)
        .execute(pool)
        .await
        .expect("assign handler verifier role");
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn postgres_inspect_and_sign_handlers_write_idempotency_and_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let authorized = ctx(owner_id, &["m2.write"]);
    let second_signer_id = Uuid::new_v4();
    seed_receiving_verifiers(&pool, owner_id, &[authorized.user_id, second_signer_id]).await;
    let state = Wave3AppState::with_postgres(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 12, 0, 0)
        .single()
        .expect("valid time");
    let repository = state
        .wave3_repository
        .as_ref()
        .expect("postgres repository");
    let supplier_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    seed_active_supplier_and_product(&pool, owner_id, supplier_id, "P-001").await;
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, '收货处理器测试仓', 'normal', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("M2-HANDLER-WH-{}", &warehouse_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed handler warehouse");
    let order = repository
        .create_receiving_order(
            &authorized,
            CreateReceivingOrderRequest {
                receipt_no: "ASN-HANDLER-PG-003".to_string(),
                document_type: "purchase_inbound".to_string(),
                supplier_id: Some(supplier_id),
                warehouse_id,
                external_ref: None,
                expected_arrival_at: Some(now + chrono::Duration::days(1)),
                lines: vec![receiving_line()],
            },
            now,
        )
        .await
        .expect("create order");
    sqlx::query("UPDATE receiving_orders SET status = 'released' WHERE id = $1")
        .bind(order.id)
        .execute(&pool)
        .await
        .expect("prepare released state");
    repository
        .receive_receiving_order_with_audit(
            &authorized,
            order.id,
            ReceiveReceivingOrderRequest {
                actual_qty: 10.into(),
                shortage_qty: wms_domain::Quantity::ZERO,
                rejected_qty: wms_domain::Quantity::ZERO,
                arrival_temperature_celsius: None,
                exception_note: None,
                details: Some(ReceivingReceiptDetails {
                    delivery_qty: 10.into(),
                    second_receiver_id: None,
                    sales_return_batches: vec![],
                    temperature_control_method: Some("普通".to_string()),
                    vehicle_no: Some("沪A00000".to_string()),
                    origin: Some("发运地".to_string()),
                    departure_at: Some(chrono::Utc::now()),
                    arrival_at: Some(chrono::Utc::now()),
                    storage_at: Some(chrono::Utc::now()),
                    transport_mode: Some("公路".to_string()),
                    carrier: Some("承运商".to_string()),
                    contact_name: Some("送货人".to_string()),
                    contact_phone: Some("13800000000".to_string()),
                    contact_id_no: Some("310101199001011234".to_string()),
                    seal_checked: Some("已核对".to_string()),
                    filing_checked: Some("已核对".to_string()),
                }),
            },
            now,
            "handler-receive-before-inspect",
            None,
        )
        .await
        .expect("receive before inspect");

    let inspect_req = InspectReceivingOrderRequest {
        batch_no: "B202606".to_string(),
        accepted_qty: 10.into(),
        rejected_qty: wms_domain::Quantity::ZERO,
        production_date: "2026-01-01".to_string(),
        expiry_date: "2028-01-01".to_string(),
        quality_status: crate::inventory::STATUS_QUALIFIED.to_string(),
        trace_codes: vec!["TRACE-001".to_string()],

                appearance_check: Some("完好".to_string()),
                package_check: Some("完好".to_string()),
                instruction_check: Some("有".to_string()),
                label_check: Some("清晰".to_string()),
                sampling_qty: Some(1.into()),
                approval_no: None,
            };
    let Json(inspection) = inspect_receiving_order_handler(
        authorized.clone(),
        State(state.clone()),
        Path(order.id),
        idempotency_headers("handler-inspect-1"),
        Json(inspect_req.clone()),
    )
    .await
    .expect("postgres inspect should succeed");
    let Json(inspection_replay) = inspect_receiving_order_handler(
        authorized.clone(),
        State(state.clone()),
        Path(order.id),
        idempotency_headers("handler-inspect-1"),
        Json(inspect_req),
    )
    .await
    .expect("same inspect idempotency key should replay");
    assert_eq!(inspection.id, inspection_replay.id);

    let first_sign_req = SignInspectionRequest {
        first_signer_id: authorized.user_id,
        second_signer_id: None,
        dual_required: true,
    };
    let Json(first_signature) = sign_receiving_order_handler(
        authorized.clone(),
        State(state.clone()),
        Path(order.id),
        idempotency_headers("handler-sign-1"),
        Json(first_sign_req.clone()),
    )
    .await
    .expect("first sign should succeed");
    let Json(first_signature_replay) = sign_receiving_order_handler(
        authorized.clone(),
        State(state.clone()),
        Path(order.id),
        idempotency_headers("handler-sign-1"),
        Json(first_sign_req),
    )
    .await
    .expect("same first-sign idempotency key should replay");
    assert_eq!(first_signature.id, first_signature_replay.id);

    let mut second_ctx = authorized.clone();
    second_ctx.user_id = second_signer_id;
    let second_sign_req = SignInspectionRequest {
        first_signer_id: authorized.user_id,
        second_signer_id: Some(second_signer_id),
        dual_required: true,
    };
    let Json(signature) = sign_receiving_order_handler(
        second_ctx.clone(),
        State(state.clone()),
        Path(order.id),
        idempotency_headers("handler-sign-2"),
        Json(second_sign_req.clone()),
    )
    .await
    .expect("second sign should succeed");
    let Json(signature_replay) = sign_receiving_order_handler(
        second_ctx,
        State(state),
        Path(order.id),
        idempotency_headers("handler-sign-2"),
        Json(second_sign_req),
    )
    .await
    .expect("same second-sign idempotency key should replay");
    assert_eq!(signature.id, signature_replay.id);

    let counts: (i64, i64, i64, String) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM receiving_inspections WHERE receiving_order_id = $1),
                (SELECT COUNT(*) FROM receiving_inspection_signatures WHERE receiving_order_id = $1),
                (SELECT COUNT(*) FROM audit_event WHERE owner_id = $2 AND action IN ('inspect', 'sign')),
                (SELECT status FROM receiving_orders WHERE id = $1)
            "#,
        )
        .bind(order.id)
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("counts");
    // inspect + 第一签字 + 第二签字 → 签名 append-only 2 条；审计 inspect+2×sign。
    assert_eq!(counts, (1, 2, 3, "putaway".to_string()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn postgres_inventory_query_and_status_change_are_scoped_idempotent_and_audited(
    pool: PgPool,
) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let authorized = ctx(owner_id, &["m3.read", "m3.write"]);
    let state = Wave3AppState::with_postgres(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 13, 0, 0)
        .single()
        .expect("valid time");
    let batch_id = Uuid::new_v4();
    let other_batch_id = Uuid::new_v4();
    for (id, owner, code) in [
        (batch_id, owner_id, "P-001"),
        (other_batch_id, other_owner_id, "P-002"),
    ] {
        sqlx::query(
            r#"
                INSERT INTO inventory_batches (
                    id, owner_id, product_code, batch_no, production_date, expiry_date,
                    qty_on_hand, qty_frozen, status, location_id, location_code,
                    recall_flag, created_at, updated_at
                )
                VALUES ($1, $2, $3, 'B202606', $4, $5, 10, 0, $6, $7, 'A-01-01', FALSE, $8, $8)
                "#,
        )
        .bind(id)
        .bind(owner)
        .bind(code)
        .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"))
        .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("valid date"))
        .bind(STATUS_QUALIFIED)
        .bind(Uuid::new_v4())
        .bind(now)
        .execute(&pool)
        .await
        .expect("seed inventory batch");
    }

    let Json(list) = list_inventory_batches_handler(
        authorized.clone(),
        State(state.clone()),
        Query(InventoryBatchListQuery {
            filter: InventoryBatchQuery {
            product_code: Some("P-001".to_string()),
            ..InventoryBatchQuery::default()
        },
            page: None,
            page_size: None,
        }),
    )
    .await
    .expect("list should use postgres repository");
    assert_eq!(list.page.count, 1);
    assert_eq!(list.data[0].owner_id, owner_id);
    assert_eq!(list.data[0].id, batch_id);

    let missing_approval = change_inventory_batch_status_handler(
        authorized.clone(),
        State(state.clone()),
        idempotency_headers("m3-status-invalid"),
        Json(ChangeInventoryStatusRequest {
            batch_id,
            target_status: STATUS_QUARANTINED.to_string(),
            reason: "missing approval".to_string(),
            approval_source: "".to_string(),
            approval_id: "".to_string(),
        }),
    )
    .await
    .expect_err("approval source should be required");
    assert!(matches!(
        missing_approval,
        Wave3HandlerError::Repository(
            crate::wave3_repository::Wave3RepositoryError::MissingApprovalSource
        )
    ));

    let req = ChangeInventoryStatusRequest {
        batch_id,
        target_status: STATUS_QUARANTINED.to_string(),
        reason: "temperature exception".to_string(),
        approval_source: "温度超标事件".to_string(),
        approval_id: "TEMP-001".to_string(),
    };
    let Json(quarantined) = change_inventory_batch_status_handler(
        authorized.clone(),
        State(state.clone()),
        idempotency_headers("m3-status-1"),
        Json(req.clone()),
    )
    .await
    .expect("status change should succeed");
    let Json(replay) = change_inventory_batch_status_handler(
        authorized,
        State(state),
        idempotency_headers("m3-status-1"),
        Json(req),
    )
    .await
    .expect("same idempotency key should replay");

    assert_eq!(quarantined.id, batch_id);
    assert_eq!(quarantined.status, STATUS_QUARANTINED);
    assert_eq!(replay.status, STATUS_QUARANTINED);

    let counts: (i64, i64, String) = sqlx::query_as(
        r#"
            SELECT
                (SELECT COUNT(*) FROM inventory_status_changes WHERE batch_id = $1),
                (SELECT COUNT(*) FROM audit_event WHERE owner_id = $2 AND action = 'change_status'),
                (SELECT status FROM inventory_batches WHERE id = $1)
            "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("counts");
    assert_eq!(counts, (1, 1, STATUS_QUARANTINED.to_string()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn postgres_cold_chain_reading_uses_external_api_key_idempotency_and_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key = "test-cold-chain-key";
    let state = Wave3AppState::with_postgres_and_cold_chain_api_key(
        pool.clone(),
        external_api_key_config(owner_id, api_key),
    );
    seed_cold_chain_device(&pool, owner_id, "TEMP-DEVICE-01").await;

    let captured_at = Utc::now() - chrono::Duration::minutes(5);
    let req = IngestTemperatureReadingRequest {
        device_code: "TEMP-DEVICE-01".to_string(),
        temperature_celsius: 5.2,
        humidity_percent: Some(60.0),
        captured_at,
        external_report_url: Some("https://cold-chain.example.test/report/1".to_string()),
        out_of_range: false,
    };

    let missing_key = ingest_temperature_reading_handler(
        State(state.clone()),
        idempotency_headers("m5-reading-missing-key"),
        Json(req.clone()),
    )
    .await
    .expect_err("external API key should be required");
    assert!(matches!(
        missing_key,
        Wave3HandlerError::ExternalAuthMissing
    ));

    let bad_key = ingest_temperature_reading_handler(
        State(state.clone()),
        external_auth_headers("m5-reading-bad-key", "wrong-key"),
        Json(req.clone()),
    )
    .await
    .expect_err("invalid external API key should be rejected");
    assert!(matches!(bad_key, Wave3HandlerError::ExternalAuthInvalid));

    let Json(reading) = ingest_temperature_reading_handler(
        State(state.clone()),
        external_auth_headers("m5-reading-1", api_key),
        Json(req.clone()),
    )
    .await
    .expect("reading should be persisted");
    let Json(replay) = ingest_temperature_reading_handler(
        State(state),
        external_auth_headers("m5-reading-1", api_key),
        Json(req),
    )
    .await
    .expect("same idempotency key should replay");

    assert_eq!(reading.id, replay.id);
    let counts: (i64, i64, String) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM temperature_readings WHERE owner_id = $1),
                (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'ingest_reading'),
                (SELECT actor_name FROM audit_event WHERE owner_id = $1 AND action = 'ingest_reading')
            "#,
        )
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("counts");
    assert_eq!(counts, (1, 1, "external-cold-chain-test".to_string()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn postgres_cold_chain_excursion_is_idempotent_and_audited(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key = "test-cold-chain-key";
    let state = Wave3AppState::with_postgres_and_cold_chain_api_key(
        pool.clone(),
        external_api_key_config(owner_id, api_key),
    );
    seed_cold_chain_device(&pool, owner_id, "TEMP-DEVICE-02").await;

    let started_at = Utc::now() - chrono::Duration::minutes(30);
    let req = IngestTemperatureExcursionRequest {
        external_event_id: "EXT-EVENT-001".to_string(),
        device_code: "TEMP-DEVICE-02".to_string(),
        location_code: Some("CC-01".to_string()),
        started_at,
        ended_at: Some(started_at + chrono::Duration::minutes(15)),
        min_temperature_celsius: Some(1.0),
        max_temperature_celsius: Some(9.1),
        affected_batch_ids: vec![Uuid::new_v4()],
    };

    let Json(event) = ingest_temperature_excursion_handler(
        State(state.clone()),
        external_auth_headers("m5-excursion-1", api_key),
        Json(req.clone()),
    )
    .await
    .expect("excursion should be persisted");
    let Json(replay) = ingest_temperature_excursion_handler(
        State(state),
        external_auth_headers("m5-excursion-1", api_key),
        Json(req),
    )
    .await
    .expect("same idempotency key should replay");

    assert_eq!(event.id, replay.id);
    assert_eq!(event.status, "pending_disposition");
    let counts: (i64, i64, String) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM temperature_excursion_events WHERE owner_id = $1),
                (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'ingest_excursion'),
                (SELECT resource_id FROM audit_event WHERE owner_id = $1 AND action = 'ingest_excursion')
            "#,
        )
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("counts");
    assert_eq!(counts, (1, 1, event.id.to_string()));
}

#[tokio::test]
async fn inventory_handlers_audit_success_and_skip_failed_business_rule() {
    let owner_id = Uuid::new_v4();
    let authorized = ctx(owner_id, &["m3.write"]);
    let state = Wave3AppState::default();

    let Json(batch) = putaway_inventory_batch_handler(
        authorized.clone(),
        State(state.clone()),
        Json(inventory_putaway_req()),
    )
    .await
    .expect("putaway should create batch");
    assert_eq!(batch.status, STATUS_QUALIFIED);
    assert_eq!(state.audit_log.lock().await.events().len(), 1);

    let missing_approval = change_inventory_batch_status_handler(
        authorized.clone(),
        State(state.clone()),
        idempotency_headers("fallback-status-missing-approval"),
        Json(ChangeInventoryStatusRequest {
            batch_id: batch.id,
            target_status: STATUS_QUARANTINED.to_string(),
            reason: "temperature exception".to_string(),
            approval_source: "".to_string(),
            approval_id: "".to_string(),
        }),
    )
    .await
    .expect_err("approval source should be required");
    assert!(matches!(
        missing_approval,
        Wave3HandlerError::Inventory(InventoryError::MissingApprovalSource)
    ));
    assert_eq!(state.audit_log.lock().await.events().len(), 1);

    let missing_idempotency_key = change_inventory_batch_status_handler(
        authorized.clone(),
        State(state.clone()),
        HeaderMap::new(),
        Json(ChangeInventoryStatusRequest {
            batch_id: batch.id,
            target_status: STATUS_QUARANTINED.to_string(),
            reason: "temperature exception".to_string(),
            approval_source: "温度超标事件".to_string(),
            approval_id: "TEMP-001".to_string(),
        }),
    )
    .await
    .expect_err("idempotency key should be required for fallback writes");
    assert!(matches!(
        missing_idempotency_key,
        Wave3HandlerError::MissingIdempotencyKey
    ));
    assert_eq!(state.audit_log.lock().await.events().len(), 1);

    let Json(quarantined) = change_inventory_batch_status_handler(
        authorized,
        State(state.clone()),
        idempotency_headers("fallback-status-1"),
        Json(ChangeInventoryStatusRequest {
            batch_id: batch.id,
            target_status: STATUS_QUARANTINED.to_string(),
            reason: "temperature exception".to_string(),
            approval_source: "温度超标事件".to_string(),
            approval_id: "TEMP-001".to_string(),
        }),
    )
    .await
    .expect("approved transition should succeed");

    assert_eq!(quarantined.status, STATUS_QUARANTINED);
    let audit_log = state.audit_log.lock().await;
    assert_eq!(audit_log.events().len(), 2);
    assert_eq!(audit_log.events()[1].action, "change_status");
    assert_eq!(audit_log.events()[1].resource_id, batch.id.to_string());
    audit_log
        .verify_hash_chain()
        .expect("audit hash chain should verify");
}

#[tokio::test]
async fn expiry_handler_is_owner_scoped_and_returns_isolated_batches() {
    let owner_id = Uuid::new_v4();
    let authorized = ctx(owner_id, &["m3.write"]);
    let state = Wave3AppState::default();
    let Json(batch) = putaway_inventory_batch_handler(
        authorized.clone(),
        State(state.clone()),
        Json(inventory_putaway_req()),
    )
    .await
    .expect("putaway should create a future batch");

    let Json(expired) = isolate_expired_inventory_batches_handler(
        authorized.clone(),
        State(state.clone()),
        idempotency_headers("fallback-expire-1"),
        Json(ExpireInventoryBatchesRequest {
            as_of: Some("2028-01-01".to_string()),
        }),
    )
    .await
    .expect("expiry handler should isolate matching batches");
    assert_eq!(expired.page.count, 1);
    assert_eq!(expired.data[0].id, batch.id);
    assert_eq!(expired.data[0].status, STATUS_UNQUALIFIED);

    let Json(replay) = isolate_expired_inventory_batches_handler(
        authorized,
        State(state),
        idempotency_headers("fallback-expire-2"),
        Json(ExpireInventoryBatchesRequest {
            as_of: Some("2028-01-01".to_string()),
        }),
    )
    .await
    .expect("already isolated batches should be skipped");
    assert_eq!(replay.page.count, 0);
}

#[tokio::test]
async fn inventory_batch_date_query_filters_and_sorts_in_memory() {
    let owner_id = Uuid::new_v4();
    let authorized = ctx(owner_id, &["m3.read", "m3.write"]);
    let state = Wave3AppState::default();

    // 相对日期：绝对日期会随运行日推进而过期（putaway 拒绝 ExpiredBatch），根治脆弱性。
    let today = chrono::Utc::now().date_naive();
    let later_date = today + chrono::Duration::days(120);
    let earlier_date = today + chrono::Duration::days(60);
    let from_date = today + chrono::Duration::days(30);
    let to_date = today + chrono::Duration::days(150);

    let mut later = inventory_putaway_req();
    later.batch_no = "B-LATER".to_string();
    later.expiry_date = later_date.format("%Y-%m-%d").to_string();
    let Json(_) =
        putaway_inventory_batch_handler(authorized.clone(), State(state.clone()), Json(later))
            .await
            .expect("later batch should be stored");

    let mut earlier = inventory_putaway_req();
    earlier.batch_no = "B-EARLIER".to_string();
    earlier.expiry_date = earlier_date.format("%Y-%m-%d").to_string();
    let Json(_) =
        putaway_inventory_batch_handler(authorized.clone(), State(state.clone()), Json(earlier))
            .await
            .expect("earlier batch should be stored");

    let Json(list) = list_inventory_batches_handler(
        authorized,
        State(state),
        Query(InventoryBatchListQuery {
            filter: InventoryBatchQuery {
            expiry_from: Some(from_date.format("%Y-%m-%d").to_string()),
            expiry_to: Some(to_date.format("%Y-%m-%d").to_string()),
            ..Default::default()
        },
            page: None,
            page_size: None,
        }),
    )
    .await
    .expect("date query should filter in-memory batches");

    assert_eq!(list.data.len(), 2);
    assert_eq!(list.data[0].batch_no, "B-EARLIER");
    assert_eq!(list.data[1].batch_no, "B-LATER");
}

#[tokio::test]
async fn inventory_batch_date_query_rejects_reversed_ranges() {
    let owner_id = Uuid::new_v4();
    let authorized = ctx(owner_id, &["m3.read"]);
    let state = Wave3AppState::default();

    let production_error = list_inventory_batches_handler(
        authorized.clone(),
        State(state.clone()),
        Query(InventoryBatchListQuery {
            filter: InventoryBatchQuery {
            production_from: Some("2026-02-01".to_string()),
            production_to: Some("2026-01-01".to_string()),
            ..Default::default()
        },
            page: None,
            page_size: None,
        }),
    )
    .await
    .expect_err("reversed production range should fail");
    assert!(matches!(
        production_error,
        Wave3HandlerError::Repository(crate::wave3_repository::Wave3RepositoryError::InvalidDate(
            ref reason,
        ))
            if reason == "production_from_after_production_to"
    ));

    let created_error = list_inventory_batches_handler(
        authorized,
        State(state),
        Query(InventoryBatchListQuery {
            filter: InventoryBatchQuery {
            created_from: Some("2026-07-01T00:00:00Z".to_string()),
            created_to: Some("2026-06-01T00:00:00Z".to_string()),
            ..Default::default()
        },
            page: None,
            page_size: None,
        }),
    )
    .await
    .expect_err("reversed created range should fail");
    assert!(matches!(
        created_error,
        Wave3HandlerError::Repository(crate::wave3_repository::Wave3RepositoryError::InvalidDate(
            ref reason,
        ))
            if reason == "created_from_after_created_to"
    ));
}

#[tokio::test]
async fn inventory_batches_handler_reads_config_center_smoke_flag_and_fails_closed() {
    let owner_id = Uuid::new_v4();
    let authorized = ctx(owner_id, &["m3.read"]);
    let config_center_state = ConfigCenterAppState::from_registry(config_center_smoke_registry());
    let state = Wave3AppState::default().with_config_center(config_center_state.clone());

    {
        config_center_state
            .switch_feature_flag_source(FeatureFlagSource::ConfigCenter)
            .await;
    }

    let missing_before_migration = list_inventory_batches_handler(
        authorized.clone(),
        State(state.clone()),
        Query(InventoryBatchListQuery { filter: InventoryBatchQuery::default(), page: None, page_size: None }),
    )
    .await
    .expect_err("config-center source should fail closed before migration");
    assert!(matches!(
        missing_before_migration,
        Wave3HandlerError::ConfigCenter(ConfigCenterError::MissingFlag(_))
    ));
    let (status, error) = error_response(missing_before_migration).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error.code, CONFIG_FLAG_MISSING_CODE);

    {
        config_center_state.migrate_feature_flags_from_file().await;
    }

    let Json(list) = list_inventory_batches_handler(
        authorized,
        State(state),
        Query(InventoryBatchListQuery { filter: InventoryBatchQuery::default(), page: None, page_size: None }),
    )
    .await
    .expect("migrated config-center smoke flag should allow inventory list");

    assert_eq!(list.page.count, 0);
}
