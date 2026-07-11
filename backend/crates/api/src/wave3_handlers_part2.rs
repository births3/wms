    #[sqlx::test(migrations = "../../migrations")]
    async fn postgres_inspect_and_sign_handlers_write_idempotency_and_audit(pool: PgPool) {
        let owner_id = Uuid::new_v4();
        let authorized = ctx(owner_id, &["m2.write"]);
        let state = Wave3AppState::with_postgres(pool.clone());
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 12, 0, 0)
            .single()
            .expect("valid time");
        let repository = state
            .wave3_repository
            .as_ref()
            .expect("postgres repository");
        let order = repository
            .create_receiving_order(
                &authorized,
                CreateReceivingOrderRequest {
                    receipt_no: "ASN-HANDLER-PG-003".to_string(),
                    document_type: "purchase_inbound".to_string(),
                    supplier_id: None,
                    warehouse_id: Uuid::new_v4(),
                    external_ref: None,
                    expected_arrival_at: None,
                    lines: vec![receiving_line()],
                },
                now,
            )
            .await
            .expect("create order");
        sqlx::query("UPDATE receiving_orders SET status = 'inspecting' WHERE id = $1")
            .bind(order.id)
            .execute(&pool)
            .await
            .expect("prepare inspecting state");

        let inspect_req = InspectReceivingOrderRequest {
            batch_no: "B202606".to_string(),
            accepted_qty: 10,
            rejected_qty: 0,
            production_date: "2026-01-01".to_string(),
            expiry_date: "2028-01-01".to_string(),
            quality_status: crate::inventory::STATUS_QUALIFIED.to_string(),
            trace_codes: vec!["TRACE-001".to_string()],
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

        let second_signer_id = Uuid::new_v4();
        let sign_req = SignInspectionRequest {
            first_signer_id: authorized.user_id,
            second_signer_id: Some(second_signer_id),
            dual_required: true,
        };
        let Json(signature) = sign_receiving_order_handler(
            authorized.clone(),
            State(state.clone()),
            Path(order.id),
            idempotency_headers("handler-sign-1"),
            Json(sign_req.clone()),
        )
        .await
        .expect("postgres sign should succeed");
        let Json(signature_replay) = sign_receiving_order_handler(
            authorized,
            State(state),
            Path(order.id),
            idempotency_headers("handler-sign-1"),
            Json(sign_req),
        )
        .await
        .expect("same sign idempotency key should replay");
        assert_eq!(signature.id, signature_replay.id);

        let counts: (i64, i64, i64, i64, String) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM receiving_inspections WHERE receiving_order_id = $1),
                (SELECT COUNT(*) FROM receiving_inspection_signatures WHERE receiving_order_id = $1),
                (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $2),
                (SELECT COUNT(*) FROM audit_event WHERE owner_id = $2 AND action IN ('inspect', 'sign')),
                (SELECT status FROM receiving_orders WHERE id = $1)
            "#,
        )
        .bind(order.id)
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("counts");
        assert_eq!(counts, (1, 1, 2, 2, "putaway".to_string()));
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
                    qty_on_hand, qty_locked, quality_status, location_id, location_code,
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

        let Json(list) = list_inventory_batches_handler(authorized.clone(), State(state.clone()))
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
        assert_eq!(quarantined.quality_status, STATUS_QUARANTINED);
        assert_eq!(replay.quality_status, STATUS_QUARANTINED);

        let counts: (i64, i64, i64, String) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM inventory_status_changes WHERE batch_id = $1),
                (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $2),
                (SELECT COUNT(*) FROM audit_event WHERE owner_id = $2 AND action = 'change_status'),
                (SELECT quality_status FROM inventory_batches WHERE id = $1)
            "#,
        )
        .bind(batch_id)
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("counts");
        assert_eq!(counts, (1, 1, 1, STATUS_QUARANTINED.to_string()));
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
        let counts: (i64, i64, i64, String) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM temperature_readings WHERE owner_id = $1),
                (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1),
                (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'ingest_reading'),
                (SELECT actor_name FROM audit_event WHERE owner_id = $1 AND action = 'ingest_reading')
            "#,
        )
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("counts");
        assert_eq!(counts, (1, 1, 1, "external-cold-chain-test".to_string()));
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
        let counts: (i64, i64, i64, String) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM temperature_excursion_events WHERE owner_id = $1),
                (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1),
                (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'ingest_excursion'),
                (SELECT resource_id FROM audit_event WHERE owner_id = $1 AND action = 'ingest_excursion')
            "#,
        )
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("counts");
        assert_eq!(counts, (1, 1, 1, event.id.to_string()));
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
        assert_eq!(batch.quality_status, STATUS_QUALIFIED);
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

        assert_eq!(quarantined.quality_status, STATUS_QUARANTINED);
        let audit_log = state.audit_log.lock().await;
        assert_eq!(audit_log.events().len(), 2);
        assert_eq!(audit_log.events()[1].action, "change_status");
        assert_eq!(audit_log.events()[1].resource_id, batch.id.to_string());
        audit_log
            .verify_hash_chain()
            .expect("audit hash chain should verify");
    }

    #[tokio::test]
    async fn inventory_batches_handler_reads_config_center_smoke_flag_and_fails_closed() {
        let owner_id = Uuid::new_v4();
        let authorized = ctx(owner_id, &["m3.read"]);
        let config_center_state =
            ConfigCenterAppState::from_registry(config_center_smoke_registry());
        let state = Wave3AppState::default().with_config_center(config_center_state.clone());

        {
            config_center_state
                .switch_feature_flag_source(FeatureFlagSource::ConfigCenter)
                .await;
        }

        let missing_before_migration =
            list_inventory_batches_handler(authorized.clone(), State(state.clone()))
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

        let Json(list) = list_inventory_batches_handler(authorized, State(state))
            .await
            .expect("migrated config-center smoke flag should allow inventory list");

        assert_eq!(list.page.count, 0);
    }
