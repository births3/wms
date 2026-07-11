    use axum::{
        body::to_bytes,
        extract::{Path, State},
        http::{HeaderMap, StatusCode},
        response::IntoResponse,
        Json,
    };
    use chrono::{NaiveDate, TimeZone, Utc};
    use sqlx::PgPool;
    use uuid::Uuid;
    use wms_domain::{
        ChangeInventoryStatusRequest, CreateReceivingOrderRequest,
        IngestTemperatureExcursionRequest, IngestTemperatureReadingRequest,
        InspectReceivingOrderRequest, PutawayInventoryRequest, PutawayRequest,
        ReceiveReceivingOrderRequest, ReceivingOrderLine, SignInspectionRequest,
        UpdateReceivingOrderRequest,
    };

    use super::{
        change_inventory_batch_status_handler, ingest_temperature_excursion_handler,
        ingest_temperature_reading_handler, inspect_receiving_order_handler,
        list_inventory_batches_handler, putaway_inventory_batch_handler,
        putaway_receiving_order_handler, receive_receiving_order_handler, sha256_hex,
        sign_receiving_order_handler, update_receiving_order_handler, wave3_router,
        ExternalApiKeyConfig, Wave3AppState, Wave3HandlerError, EXTERNAL_API_KEY_HEADER,
        IDEMPOTENCY_KEY_HEADER, INVENTORY_BATCHES_SMOKE_FLAG,
    };
    use crate::{
        auth::{AuthContext, AuthError},
        config_center::{
            ConfigCenterAppState, ConfigCenterError, FeatureFlagSource, CONFIG_FLAG_MISSING_CODE,
        },
        feature_flags::FeatureFlagRegistry,
        inventory::{InventoryError, STATUS_QUALIFIED, STATUS_QUARANTINED},
    };

    fn ctx(owner_id: Uuid, permissions: &[&str]) -> AuthContext {
        AuthContext {
            user_id: Uuid::new_v4(),
            owner_id,
            actor_name: "tester".to_string(),
            permissions: permissions
                .iter()
                .map(|permission| permission.to_string())
                .collect(),
            jti: Uuid::new_v4().to_string(),
        }
    }
    fn receiving_line() -> ReceivingOrderLine {
        ReceivingOrderLine {
            line_no: 1,
            product_id: None,
            product_code: "P-001".to_string(),
            expected_qty: 10,
            batch_no: Some("B202606".to_string()),
            production_date: Some("2026-01-01".to_string()),
            expiry_date: Some("2028-01-01".to_string()),
        }
    }

    fn inventory_putaway_req() -> PutawayInventoryRequest {
        PutawayInventoryRequest {
            product_code: "P-001".to_string(),
            batch_no: "B202606".to_string(),
            production_date: "2026-01-01".to_string(),
            expiry_date: "2028-01-01".to_string(),
            qty: 10,
            quality_status: STATUS_QUALIFIED.to_string(),
            location_id: Uuid::new_v4(),
            location_code: "A-01-01".to_string(),
            source_receiving_order_id: Uuid::new_v4(),
        }
    }

    fn config_center_smoke_registry() -> FeatureFlagRegistry {
        FeatureFlagRegistry::from_toml_str(&format!(
            r#"
            [[flags]]
            key = "{INVENTORY_BATCHES_SMOKE_FLAG}"
            owner = "platform"
            created_at = 2026-06-07
            cleanup_by = 2026-09-05
            enabled = true
            "#
        ))
        .expect("valid smoke flag registry")
    }

    fn idempotency_headers(key: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(IDEMPOTENCY_KEY_HEADER, key.parse().expect("valid header"));
        headers
    }

    fn external_auth_headers(idempotency_key: &'static str, api_key: &'static str) -> HeaderMap {
        let mut headers = idempotency_headers(idempotency_key);
        headers.insert(
            EXTERNAL_API_KEY_HEADER,
            api_key.parse().expect("valid header"),
        );
        headers
    }

    fn external_api_key_config(owner_id: Uuid, api_key: &str) -> ExternalApiKeyConfig {
        ExternalApiKeyConfig {
            key_sha256: sha256_hex(api_key.as_bytes()),
            owner_id,
            actor_name: "external-cold-chain-test".to_string(),
        }
    }

    async fn error_response(error: Wave3HandlerError) -> (StatusCode, wms_domain::ErrorResponse) {
        let response = error.into_response();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let payload = serde_json::from_slice(&body).expect("error response should be json");
        (status, payload)
    }

    async fn seed_cold_chain_device(pool: &PgPool, owner_id: Uuid, device_code: &str) {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 14, 0, 0)
            .single()
            .expect("valid time");
        sqlx::query(
            r#"
	            INSERT INTO cold_chain_devices (
	                id, owner_id, device_code, device_type,
	                installed_at_location_code, calibration_due_at, status, created_at, updated_at
	            )
	            VALUES ($1, $2, $3, 'thermometer', 'CC-01', NULL, 'active', $4, $4)
	            "#,
        )
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .bind(device_code)
        .bind(now)
        .execute(pool)
        .await
        .expect("seed cold-chain device");
    }

    #[test]
    fn wave3_router_registers_first_batch_handlers() {
        let _router = wave3_router(Wave3AppState::default());
    }

    #[tokio::test]
    async fn update_receiving_order_requires_permission_and_appends_audit() {
        let owner_id = Uuid::new_v4();
        let state = Wave3AppState::default();
        let authorized = ctx(owner_id, &["m2.write"]);
        let created = {
            let mut store = state.inbound_store.lock().await;
            store
                .create(
                    &authorized,
                    CreateReceivingOrderRequest {
                        receipt_no: "ASN-UPDATE-001".to_string(),
                        document_type: "purchase_inbound".to_string(),
                        supplier_id: None,
                        warehouse_id: Uuid::new_v4(),
                        external_ref: None,
                        expected_arrival_at: None,
                        lines: vec![receiving_line()],
                    },
                    Utc::now(),
                )
                .expect("receiving order should be created")
        };

        let denied = update_receiving_order_handler(
            ctx(owner_id, &[]),
            State(state.clone()),
            Path(created.id),
            idempotency_headers("update-denied"),
            Json(UpdateReceivingOrderRequest {
                supplier_id: None,
                warehouse_id: None,
                external_ref: Some(Some("ERP-UPDATED".to_string())),
                expected_arrival_at: None,
                lines: None,
            }),
        )
        .await;
        assert!(matches!(
            denied,
            Err(Wave3HandlerError::Auth(AuthError::PermissionDenied(permission)))
                if permission == "m2.write"
        ));

        let missing_key = update_receiving_order_handler(
            authorized.clone(),
            State(state.clone()),
            Path(created.id),
            HeaderMap::new(),
            Json(UpdateReceivingOrderRequest {
                supplier_id: None,
                warehouse_id: None,
                external_ref: Some(Some("ERP-UPDATED".to_string())),
                expected_arrival_at: None,
                lines: None,
            }),
        )
        .await;
        assert!(matches!(
            missing_key,
            Err(Wave3HandlerError::MissingIdempotencyKey)
        ));

        let updated = update_receiving_order_handler(
            authorized,
            State(state.clone()),
            Path(created.id),
            idempotency_headers("update-authorized"),
            Json(UpdateReceivingOrderRequest {
                supplier_id: None,
                warehouse_id: None,
                external_ref: Some(Some("ERP-UPDATED".to_string())),
                expected_arrival_at: None,
                lines: None,
            }),
        )
        .await
        .expect("authorized update should succeed")
        .0;

        assert_eq!(updated.external_ref.as_deref(), Some("ERP-UPDATED"));
        let audit = state.audit_log.lock().await;
        assert_eq!(audit.events().len(), 1);
        assert_eq!(audit.events()[0].action, "update");
        let diff = audit.events()[0]
            .diff
            .as_ref()
            .expect("update audit should record before and after values");
        assert!(diff.changed_keys.contains(&"external_ref".to_string()));
        assert_eq!(diff.before["external_ref"], serde_json::Value::Null);
        assert_eq!(diff.after["external_ref"], "ERP-UPDATED");
        drop(audit);
    }

    #[tokio::test]
    async fn inbound_receive_handler_requires_permission_and_appends_audit() {
        let owner_id = Uuid::new_v4();
        let authorized = ctx(owner_id, &["m2.write"]);
        let denied_ctx = ctx(owner_id, &[]);
        let state = Wave3AppState::default();
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 10, 0, 0)
            .single()
            .expect("valid time");
        let order = {
            let mut store = state.inbound_store.lock().await;
            let created = store
                .create(
                    &authorized,
                    CreateReceivingOrderRequest {
                        receipt_no: "ASN-HANDLER-001".to_string(),
                        document_type: "purchase_inbound".to_string(),
                        supplier_id: Some(Uuid::new_v4()),
                        warehouse_id: Uuid::new_v4(),
                        external_ref: None,
                        expected_arrival_at: None,
                        lines: vec![receiving_line()],
                    },
                    now,
                )
                .expect("create order");
            store
                .release(&authorized, created.id, now)
                .expect("release order")
        };

        let req = ReceiveReceivingOrderRequest {
            actual_qty: 8,
            shortage_qty: 2,
            rejected_qty: 0,
            arrival_temperature_celsius: None,
            exception_note: None,
        };
        let denied = receive_receiving_order_handler(
            denied_ctx,
            State(state.clone()),
            Path(order.id),
            HeaderMap::new(),
            Json(req.clone()),
        )
        .await
        .expect_err("permission should be required");
        assert!(matches!(
            denied,
            Wave3HandlerError::Auth(AuthError::PermissionDenied(permission))
                if permission == "m2.write"
        ));
        assert!(state.audit_log.lock().await.events().is_empty());

        let missing_idempotency_key = receive_receiving_order_handler(
            authorized.clone(),
            State(state.clone()),
            Path(order.id),
            HeaderMap::new(),
            Json(req.clone()),
        )
        .await
        .expect_err("idempotency key should be required for fallback writes");
        assert!(matches!(
            missing_idempotency_key,
            Wave3HandlerError::MissingIdempotencyKey
        ));
        assert!(state.audit_log.lock().await.events().is_empty());

        let Json(receipt) = receive_receiving_order_handler(
            authorized.clone(),
            State(state.clone()),
            Path(order.id),
            idempotency_headers("fallback-receive-1"),
            Json(req),
        )
        .await
        .expect("authorized receive should succeed");

        assert_eq!(receipt.actual_qty, 8);
        let audit_log = state.audit_log.lock().await;
        assert_eq!(audit_log.events().len(), 1);
        assert_eq!(audit_log.events()[0].action, "receive");
        assert_eq!(audit_log.events()[0].module, "M2");
        assert_eq!(audit_log.events()[0].resource_id, order.id.to_string());
        audit_log
            .verify_hash_chain()
            .expect("audit hash chain should verify");
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn postgres_receive_handler_writes_business_idempotency_and_audit(pool: PgPool) {
        let owner_id = Uuid::new_v4();
        let authorized = ctx(owner_id, &["m2.write"]);
        let state = Wave3AppState::with_postgres(pool.clone());
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 10, 0, 0)
            .single()
            .expect("valid time");
        let repository = state
            .wave3_repository
            .as_ref()
            .expect("postgres repository");
        let (supplier_id, warehouse_id) = (Uuid::new_v4(), Uuid::new_v4());
        sqlx::query("INSERT INTO suppliers (id, owner_id, supplier_code, supplier_name, uscc, status) VALUES ($1,$2,'SUP-H1','S','USCC-H1','active')").bind(supplier_id).bind(owner_id).execute(&pool).await.expect("seed supplier");
        sqlx::query("INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1,$2,'WH-H1','W','normal','active')").bind(warehouse_id).bind(owner_id).execute(&pool).await.expect("seed warehouse");
        let order = repository
            .create_receiving_order(
                &authorized,
                CreateReceivingOrderRequest {
                    receipt_no: "ASN-HANDLER-PG-001".to_string(),
                    document_type: "purchase_inbound".to_string(),
                    supplier_id: Some(supplier_id),
                    warehouse_id,
                    external_ref: None,
                    expected_arrival_at: None,
                    lines: vec![receiving_line()],
                },
                now,
            )
            .await
            .expect("create order");
        repository
            .release_receiving_order(&authorized, order.id, now)
            .await
            .expect("release order");

        let req = ReceiveReceivingOrderRequest {
            actual_qty: 8,
            shortage_qty: 2,
            rejected_qty: 0,
            arrival_temperature_celsius: None,
            exception_note: None,
        };
        let Json(receipt) = receive_receiving_order_handler(
            authorized.clone(),
            State(state.clone()),
            Path(order.id),
            idempotency_headers("handler-receive-1"),
            Json(req.clone()),
        )
        .await
        .expect("postgres receive should succeed");
        let Json(replay) = receive_receiving_order_handler(
            authorized,
            State(state.clone()),
            Path(order.id),
            idempotency_headers("handler-receive-1"),
            Json(req),
        )
        .await
        .expect("same idempotency key should replay");

        assert_eq!(receipt.id, replay.id);
        let counts: (i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM receiving_order_receipts WHERE receiving_order_id = $1),
                (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $2),
                (SELECT COUNT(*) FROM audit_event WHERE owner_id = $2 AND action = 'receive')
            "#,
        )
        .bind(order.id)
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("counts");
        assert_eq!(counts, (1, 1, 1));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn postgres_putaway_handler_commits_inventory_and_audit(pool: PgPool) {
        let owner_id = Uuid::new_v4();
        let authorized = ctx(owner_id, &["m2.write"]);
        let state = Wave3AppState::with_postgres(pool.clone());
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 11, 0, 0)
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
                    receipt_no: "ASN-HANDLER-PG-002".to_string(),
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
        sqlx::query("UPDATE receiving_orders SET status = 'putaway' WHERE id = $1")
            .bind(order.id)
            .execute(&pool)
            .await
            .expect("prepare putaway state");

        let Json(putaway) = putaway_receiving_order_handler(
            authorized,
            State(state),
            Path(order.id),
            idempotency_headers("handler-putaway-1"),
            Json(PutawayRequest {
                batch_no: "B202606".to_string(),
                product_code: "P-001".to_string(),
                qty: 10,
                location_id: Uuid::new_v4(),
                location_code: "A-01-01".to_string(),
                quality_status: crate::inventory::STATUS_QUALIFIED.to_string(),
            }),
        )
        .await
        .expect("postgres putaway should succeed");

        assert_eq!(putaway.receiving_order_id, order.id);
        let counts: (i64, i64, i64, String) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM receiving_putaways WHERE receiving_order_id = $1),
                (SELECT COUNT(*) FROM inventory_batches WHERE owner_id = $2),
                (SELECT COUNT(*) FROM audit_event WHERE owner_id = $2 AND action = 'putaway'),
                (SELECT status FROM receiving_orders WHERE id = $1)
            "#,
        )
        .bind(order.id)
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("counts");
        assert_eq!(counts, (1, 1, 1, "completed".to_string()));
    }
