#[cfg(test)]
mod tests {
    use crate::openapi_contract::{
        AUTH_EXEMPT_REASON, BEARER_AUTH_SCHEME, COLD_CHAIN_API_KEY_SCHEME,
        IDEMPOTENCY_EXEMPT_REASON,
    };
    use crate::ApiDoc;
    use utoipa::OpenApi;

    #[test]
    fn openapi_contains_wave1_contract_paths() {
        let json = ApiDoc::openapi()
            .to_pretty_json()
            .expect("openapi json should serialize");

        for required_path in [
            "/api/v1/healthz",
            "/openapi.json",
            "/api-docs",
            "/redoc",
            "/api/v1/resilience/status",
            "/metrics",
            "/api/v1/auth/login",
            "/api/v1/auth/me",
            "/api/v1/auth/sessions/revoke-others",
            "/api/v1/admin/menus/published",
            "/api/v1/admin/menus/draft",
            "/api/v1/admin/menus/draft/nodes",
            "/api/v1/admin/menus/draft/nodes/{id}",
            "/api/v1/admin/menus/draft/batch-enable",
            "/api/v1/admin/menus/publish",
            "/api/v1/admin/menus/rollback",
            "/api/v1/audit/events",
            "/api/v1/audit/events/export",
            "/api/v1/audit/archive/partitions",
            "/api/v1/audit/archive/runs",
            "/api/v1/event-bus/deliveries/pending",
            "/api/v1/event-bus/deliveries/{delivery_id}/ack",
            "/api/v1/event-bus/deliveries/{delivery_id}/nack",
            "/api/v1/business-retention/policies",
            "/api/v1/business-retention/jobs",
            "/api/v1/master-data/products",
            "/api/v1/master-data/products/{id}",
            "/api/v1/master-data/suppliers",
            "/api/v1/master-data/suppliers/{id}",
            "/api/v1/master-data/customers",
            "/api/v1/master-data/customers/{id}",
            "/api/v1/master-data/customers/{customer_id}/addresses",
            "/api/v1/master-data/customers/{customer_id}/addresses/{address_id}",
            "/api/v1/master-data/customers/{customer_id}/profile",
            "/api/v1/master-data/warehouses",
            "/api/v1/master-data/warehouses/{id}",
            "/api/v1/master-data/locations",
            "/api/v1/master-data/locations/batch-create",
            "/api/v1/master-data/locations/{id}",
            "/api/v1/master-data/special-drug-categories",
            "/api/v1/master-data/special-drug-categories/{id}",
            "/api/v1/system-dictionaries/{dict_code}/items",
            "/api/v1/system-dictionaries/{dict_code}/items/{item_code}",
            "/api/v1/system-dictionaries/{dict_code}/items/{item_code}/impact-preview",
            "/api/v1/system-dictionaries/{dict_code}/items/{item_code}/disable",
            "/api/v1/code-generator/document-number-rules",
            "/api/v1/code-generator/document-number-rules/{rule_code}",
            "/api/v1/code-generator/document-number-rules/{rule_code}/enabled",
            "/api/v1/code-generator/document-number-allocations",
            "/api/v1/docks",
            "/api/v1/docks/import",
            "/api/v1/docks/{id}",
            "/api/v1/dock-appointments/{id}",
            "/api/v1/dock-appointments/{id}/cancel",
            "/api/v1/state-machines",
            "/api/v1/state-machines/{machine_code}",
            "/api/v1/state-machines/{machine_code}/transition-validation",
            "/api/v1/inbound/receiving-orders",
            "/api/v1/inbound/receiving-dashboard",
            "/api/v1/inbound/receiving-orders/{id}",
            "/api/v1/inbound/receiving-orders/{id}/print-data",
            "/api/v1/inbound/receiving-orders/{id}/release",
            "/api/v1/inbound/receiving-orders/{id}/receive",
            "/api/v1/inbound/receiving-orders/{id}/reject",
            "/api/v1/inbound/receiving-orders/{id}/inspect",
            "/api/v1/inbound/receiving-orders/{id}/sign",
            "/api/v1/inbound/receiving-orders/{id}/putaway",
            "/api/v1/inventory/batches",
            "/api/v1/inventory/batches/{id}/trace",
            "/api/v1/inventory/batches/putaway",
            "/api/v1/inventory/batches/status",
            "/api/v1/inventory/status-transitions",
            "/api/v1/inventory/status-transitions/{from_status}/{to_status}",
            "/api/v1/inventory/batches/recall",
            "/api/v1/inventory/batches/expire",
            "/api/v1/outbound/orders",
            "/api/v1/outbound/orders/{id}",
            "/api/v1/outbound/waves",
            "/api/v1/outbound/waves/{wave_id}",
            "/api/v1/outbound/waves/{wave_id}/cancel",
            "/api/v1/outbound/pick-tasks/{id}/complete",
            "/api/v1/outbound/orders/{id}/review",
            "/api/v1/outbound/orders/{id}/ship",
            "/api/v1/reports/query",
            "/api/v1/reports/gsp/inbound-ledger",
            "/api/v1/reports/gsp/outbound-ledger",
            "/api/v1/reports/gsp/inventory-ledger",
            "/api/v1/traceability/outbound-reports",
            "/api/v1/driver/tasks/today",
            "/api/v1/store/dashboard",
            "/api/v1/parameter-mapping/map",
            "/api/v1/config-center/feature-flags/migrate",
            "/api/v1/config-center/feature-flags/reconcile",
            "/api/v1/config-center/feature-flags/export",
            "/api/v1/config-center/feature-flags/import",
            "/api/v1/config-center/feature-flags/source",
            "/api/v1/config-center/feature-flags/archive-file-source",
            "/api/v1/cold-chain/devices",
            "/api/v1/cold-chain/readings",
            "/api/v1/cold-chain/excursions",
            "/api/v1/cold-chain/excursions/pending-disposition",
            "/api/v1/cold-chain/excursions/{external_event_id}/dispose",
            "/api/v1/billing/accounts",
            "/api/v1/billing/contracts",
            "/api/v1/billing/rules",
            "/api/v1/packing/stations",
            "/api/v1/packing/jobs",
            "/api/v1/packing/jobs/{id}/weigh",
            "/api/v1/packing/jobs/{id}/waybill",
            "/api/v1/express/carriers",
            "/api/v1/express/routing-rules",
            "/api/v1/express/waybills",
            "/api/v1/express/waybills/{waybill_no}/cancel",
            "/api/v1/express/waybills/{waybill_no}/tracking",
            "/api/v1/wechat-notify/configs",
            "/api/v1/wechat-notify/settings",
            "/api/v1/wechat-notify/settings/test",
            "/api/v1/wechat-notify/send",
            "/api/v1/wechat-notify/approvals",
            "/api/v1/wechat-notify/records",
            "/api/v1/wechat-notify/records/{record_id}/resend",
            "/api/v1/print-templates/field-libraries",
            "/api/v1/print-templates/field-libraries/{version_id}/fields",
            "/api/v1/print-templates/templates",
            "/api/v1/print-templates/templates/{template_id}/versions",
            "/api/v1/print-templates/resolve",
            "/api/v1/print-templates/preview",
            "/api/v1/print-templates/print",
            "/api/v1/retail/replenishment-suggestions",
            "/api/v1/retail/crossdock-plans",
            "/api/v1/billing/charges/calculate",
            "/api/v1/billing/statements",
            "/api/v1/billing/statements/{id}/confirm",
            "/api/v1/tms/dispatches",
            "/api/v1/tms/route-plans",
            "/api/v1/tms/transit-temperature-readings",
            "/api/v1/tms/container-recoveries",
            "/api/v1/inventory/counts",
            "/api/v1/inventory/counts/{id}",
            "/api/v1/inventory/counts/{id}/lines/{line_id}/submit",
            "/api/v1/inventory/counts/{id}/approve",
            "/api/v1/inventory/maintenance/tasks",
            "/api/v1/inventory/maintenance/records",
            "/api/v1/task-engine/task-types/{task_type_code}/enabled",
            "/api/v1/integration/erp-messages/inbound/asn",
            "/api/v1/integration/erp-messages/inbound/outbound_order",
            "/api/v1/integration/erp-messages/inbound/product_change",
            "/api/v1/integration/erp-messages/inbound/product_master",
            "/api/v1/integration/erp-messages/inbound/return_order",
            "/api/v1/integration/erp-messages/{id}/receipt",
        ] {
            assert!(
                json.contains(required_path),
                "missing required path: {required_path}"
            );
        }

        for required_schema in [
            "\"ErrorResponse\"",
            "\"ResilienceStatus\"",
            "\"LoginRequest\"",
            "\"LoginResponse\"",
            "\"CurrentUser\"",
            "\"AuditEvent\"",
            "\"AuditArchivePartitionState\"",
            "\"AuditArchivePartitionStateListResponse\"",
            "\"AuditArchiveRunRequest\"",
            "\"AuditArchiveRunResponse\"",
            "\"EventDelivery\"",
            "\"EventDeliveryListResponse\"",
            "\"EventDeliveryNackRequest\"",
            "\"BusinessRetentionPolicy\"",
            "\"BusinessRetentionPolicyListResponse\"",
            "\"PlanBusinessArchiveJobRequest\"",
            "\"BusinessArchiveJob\"",
            "\"BatchCreateLocationsRequest\"",
            "\"Product\"",
            "\"Supplier\"",
            "\"ReceivingOrder\"",
            "\"ReceiveReceivingOrderRequest\"",
            "\"InventoryBatch\"",
            "\"InventoryBatchTrace\"",
            "\"MarkInventoryRecallRequest\"",
            "\"ExpireInventoryBatchesRequest\"",
            "\"ColdChainDevice\"",
            "\"BillingContract\"",
            "\"MapParameterRequest\"",
            "\"FeatureFlagBatchImportRequest\"",
            "\"FeatureFlagReconcileReport\"",
            "\"FeatureFlagArchiveResult\"",
            "\"ExpressCarrier\"",
            "\"ExpressCarrierListResponse\"",
            "\"ExpressRoutingRule\"",
            "\"ExpressRoutingRuleListResponse\"",
            "\"ExpressWaybill\"",
            "\"ExpressTrackingResponse\"",
            "\"CreateOutboundOrderRequest\"",
            "\"OutboundOrder\"",
            "\"CreateOutboundWaveRequest\"",
            "\"OutboundWave\"",
            "\"CompletePickTaskRequest\"",
            "\"ReviewOutboundOrderRequest\"",
            "\"ShipOutboundOrderRequest\"",
            "\"DisposeTemperatureExcursionRequest\"",
            "\"TemperatureExcursionDispositionResponse\"",
            "\"TemperatureExcursionEventListResponse\"",
            "\"SystemDictionaryItem\"",
            "\"SystemDictionaryItemListResponse\"",
            "\"SystemDictionaryImpactPreview\"",
            "\"SystemDictionaryImpactReference\"",
            "\"UpsertSystemDictionaryItemRequest\"",
            "\"DisableSystemDictionaryItemRequest\"",
            "\"DocumentNumberRule\"",
            "\"DocumentNumberRuleListResponse\"",
            "\"UpsertDocumentNumberRuleRequest\"",
            "\"SetDocumentNumberRuleEnabledRequest\"",
            "\"DocumentNumberAllocation\"",
            "\"DocumentNumberAllocationListResponse\"",
            "\"StateMachineDefinition\"",
            "\"StateMachineDefinitionListResponse\"",
            "\"StateTransitionValidationResponse\"",
            "\"GspLedgerReport\"",
            "\"GspLedgerRow\"",
            "\"TraceabilityOutboundReport\"",
            "\"TraceabilityOutboundReportRequest\"",
            "\"TraceabilityStatusChangeEvent\"",
            "\"DriverTask\"",
            "\"DriverTaskListResponse\"",
            "\"StoreDashboardResponse\"",
            "\"PackingStation\"",
            "\"CreatePackingStationRequest\"",
            "\"PackJob\"",
            "\"CreatePackJobRequest\"",
            "\"WeighPackJobRequest\"",
            "\"PrintWaybillRequest\"",
            "\"RetailReplenishmentSuggestion\"",
            "\"CreateRetailReplenishmentSuggestionRequest\"",
            "\"CrossdockPlan\"",
            "\"CreateCrossdockPlanRequest\"",
            "\"BillingChargeCalculation\"",
            "\"CalculateBillingChargesRequest\"",
            "\"BillingStatement\"",
            "\"GenerateBillingStatementRequest\"",
            "\"ConfirmBillingStatementRequest\"",
            "\"TmsDispatch\"",
            "\"ReceiveTmsDispatchRequest\"",
            "\"TransitTemperatureReading\"",
            "\"IngestTransitTemperatureRequest\"",
            "\"ContainerRecovery\"",
            "\"ConfirmContainerRecoveryRequest\"",
            "\"H4WechatSettingsTestResponse\"",
            "\"H8ErpBusinessReceiptRequest\"",
        ] {
            assert!(
                json.contains(required_schema),
                "missing required schema: {required_schema}"
            );
        }
    }

    #[test]
    fn h6_transition_validation_contract_matches_runtime_query() {
        let doc: serde_json::Value = serde_json::from_str(
            &ApiDoc::openapi()
                .to_pretty_json()
                .expect("openapi json should serialize"),
        )
        .expect("openapi json should parse as value");
        let parameters = doc
            .pointer(
                "/paths/~1api~1v1~1state-machines~1{machine_code}~1transition-validation/get/parameters",
            )
            .and_then(serde_json::Value::as_array)
            .expect("H6 transition validation should declare parameters");
        let names = parameters
            .iter()
            .filter_map(|parameter| parameter.get("name").and_then(serde_json::Value::as_str))
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec!["machine_code", "from_state", "to_state", "event_code"]
        );
        assert_eq!(
            parameters[3].get("required"),
            Some(&serde_json::json!(false)),
            "event_code should remain optional",
        );
    }

    #[test]
    fn m1_product_specification_is_required_and_non_nullable() {
        let doc: serde_json::Value = serde_json::from_str(
            &ApiDoc::openapi()
                .to_pretty_json()
                .expect("openapi json should serialize"),
        )
        .expect("openapi json should parse as value");

        for schema in [
            "CreateProductRequest",
            "Product",
            "H8ProductMasterInboundRequest",
        ] {
            let required = doc
                .pointer(&format!("/components/schemas/{schema}/required"))
                .and_then(serde_json::Value::as_array)
                .expect("product schema should declare required fields");
            assert!(
                required.iter().any(|field| field == "spec"),
                "{schema}.spec must be required"
            );
            assert_eq!(
                doc.pointer(&format!(
                    "/components/schemas/{schema}/properties/spec/type"
                )),
                Some(&serde_json::json!("string")),
                "{schema}.spec must be a non-null string"
            );
        }
    }

    #[test]
    fn mrc_service_writes_require_claim_contract_and_authentication_responses() {
        let doc: serde_json::Value = serde_json::from_str(
            &ApiDoc::openapi()
                .to_pretty_json()
                .expect("openapi json should serialize"),
        )
        .expect("openapi json should parse as value");
        let required = doc
            .pointer("/components/schemas/SubmitReconciliationRunRequest/required")
            .and_then(serde_json::Value::as_array)
            .expect("M-RC submit schema should declare required fields");
        for field in [
            "claim_id",
            "claim_token",
            "window_key",
            "snapshot_at",
            "items",
        ] {
            assert!(
                required.iter().any(|value| value == field),
                "SubmitReconciliationRunRequest.{field} must be required"
            );
        }
        assert_eq!(
            doc.pointer(
                "/paths/~1api~1v1~1reconciliation~1runs/post/requestBody/content/application~1json/schema/$ref",
            ),
            Some(&serde_json::json!(
                "#/components/schemas/SubmitReconciliationRunRequest"
            ))
        );
        for operation in [
            "/paths/~1api~1v1~1reconciliation~1rule/put",
            "/paths/~1api~1v1~1reconciliation~1claims/post",
            "/paths/~1api~1v1~1reconciliation~1claims~1{id}~1renew/post",
            "/paths/~1api~1v1~1reconciliation~1claims~1{id}~1failed/post",
            "/paths/~1api~1v1~1reconciliation~1runs/post",
            "/paths/~1api~1v1~1reconciliation~1items~1isolation/post",
            "/paths/~1api~1v1~1reconciliation~1items~1{id}~1resolve/post",
        ] {
            assert!(
                doc.pointer(&format!("{operation}/responses/401")).is_some(),
                "protected M-RC write should declare 401: {operation}"
            );
        }
    }

    #[test]
    fn h6_openapi_declares_permission_denied_responses() {
        let doc: serde_json::Value = serde_json::from_str(
            &ApiDoc::openapi()
                .to_pretty_json()
                .expect("openapi json should serialize"),
        )
        .expect("openapi json should parse as value");

        for path in [
            "~1api~1v1~1state-machines",
            "~1api~1v1~1state-machines~1{machine_code}",
            "~1api~1v1~1state-machines~1{machine_code}~1transition-validation",
        ] {
            assert_eq!(
                doc.pointer(&format!("/paths/{path}/get/responses/403/description")),
                Some(&serde_json::json!("权限不足")),
                "H6 endpoint should declare 403: {path}",
            );
        }
    }

    #[test]
    fn openapi_declares_h3_security_and_idempotency_contract() {
        let doc: serde_json::Value = serde_json::from_str(
            &ApiDoc::openapi()
                .to_pretty_json()
                .expect("openapi json should serialize"),
        )
        .expect("openapi json should parse as value");

        assert_eq!(
            doc.pointer("/components/securitySchemes/BearerAuth/type"),
            Some(&serde_json::json!("http")),
        );
        assert_eq!(
            doc.pointer("/components/securitySchemes/BearerAuth/scheme"),
            Some(&serde_json::json!("bearer")),
        );
        assert_eq!(
            doc.pointer("/components/securitySchemes/BearerAuth/bearerFormat"),
            Some(&serde_json::json!("JWT")),
        );
        assert_eq!(
            doc.pointer("/components/securitySchemes/ColdChainApiKeyAuth/type"),
            Some(&serde_json::json!("apiKey")),
        );
        assert_eq!(
            doc.pointer("/components/securitySchemes/ColdChainApiKeyAuth/name"),
            Some(&serde_json::json!("X-WMS-API-Key")),
        );

        let global_security = doc
            .get("security")
            .and_then(serde_json::Value::as_array)
            .expect("openapi should declare global security");
        assert!(
            global_security
                .iter()
                .any(|requirement| requirement.get(BEARER_AUTH_SCHEME).is_some()),
            "global security should require BearerAuth",
        );

        for public_operation in [
            "/paths/~1api~1v1~1healthz/get",
            "/paths/~1api~1v1~1auth~1login/post",
        ] {
            let security = doc
                .pointer(&format!("{public_operation}/security"))
                .and_then(serde_json::Value::as_array)
                .expect("public operation should override security");
            assert!(
                security.is_empty(),
                "public operation should be unauthenticated"
            );
            assert!(
                doc.pointer(&format!("{public_operation}/{AUTH_EXEMPT_REASON}"))
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|reason| !reason.is_empty()),
                "public operation should declare auth exemption reason",
            );
        }

        for external_api_key_operation in [
            "/paths/~1api~1v1~1cold-chain~1readings/post",
            "/paths/~1api~1v1~1cold-chain~1excursions/post",
            "/paths/~1api~1v1~1integration~1erp-messages~1inbound~1asn/post",
            "/paths/~1api~1v1~1integration~1erp-messages~1inbound~1outbound_order/post",
            "/paths/~1api~1v1~1integration~1erp-messages~1inbound~1return_order/post",
            "/paths/~1api~1v1~1integration~1erp-messages~1{id}~1receipt/post",
        ] {
            let security = doc
                .pointer(&format!("{external_api_key_operation}/security"))
                .and_then(serde_json::Value::as_array)
                .expect("external operation should declare security");
            assert!(
                security
                    .iter()
                    .any(|requirement| requirement.get(COLD_CHAIN_API_KEY_SCHEME).is_some()),
                "external operation should require API key",
            );
        }
        for operation in [
            "/paths/~1api~1v1~1integration~1erp-messages~1inbound~1asn/post",
            "/paths/~1api~1v1~1integration~1erp-messages~1inbound~1outbound_order/post",
            "/paths/~1api~1v1~1integration~1erp-messages~1inbound~1return_order/post",
        ] {
            let parameters = doc
                .pointer(&format!("{operation}/parameters"))
                .and_then(serde_json::Value::as_array)
                .expect("H8 inbound operation should declare headers");
            for name in ["Idempotency-Key", "X-WMS-Warehouse-ID"] {
                assert!(
                    parameters.iter().any(|parameter| {
                        parameter.get("name").and_then(serde_json::Value::as_str) == Some(name)
                            && parameter.get("required") == Some(&serde_json::json!(true))
                    }),
                    "H8 inbound operation should require {name}",
                );
            }
        }

        let login_idempotency_pointer =
            format!("/paths/~1api~1v1~1auth~1login/post/{IDEMPOTENCY_EXEMPT_REASON}");
        assert!(
            doc.pointer(&login_idempotency_pointer)
                .and_then(serde_json::Value::as_str)
                .is_some_and(|reason| !reason.is_empty()),
            "login should document idempotency exemption",
        );
        for operation in [
            "/paths/~1api~1v1~1master-data~1products/post",
            "/paths/~1api~1v1~1master-data~1suppliers/post",
            "/paths/~1api~1v1~1master-data~1customers/post",
            "/paths/~1api~1v1~1master-data~1warehouses/post",
        ] {
            let parameters = doc
                .pointer(&format!("{operation}/parameters"))
                .and_then(serde_json::Value::as_array)
                .expect("M1 master-data create should declare parameters");
            assert!(
                parameters.iter().any(|parameter| {
                    parameter.get("name").and_then(serde_json::Value::as_str)
                        == Some("Idempotency-Key")
                }),
                "M1 master-data create should declare Idempotency-Key",
            );
        }
        let h4_settings_test_idempotency_pointer = format!(
            "/paths/~1api~1v1~1wechat-notify~1settings~1test/post/{IDEMPOTENCY_EXEMPT_REASON}"
        );
        assert!(
            doc.pointer(&h4_settings_test_idempotency_pointer)
                .and_then(serde_json::Value::as_str)
                .is_some_and(|reason| !reason.is_empty()),
            "H4 settings test should document its read-only POST idempotency exemption",
        );
        assert!(
            doc.pointer(
                "/paths/~1api~1v1~1state-machines~1{machine_code}~1transition-validation/get/responses/400",
            )
            .is_some(),
            "H6 transition validation should declare malformed query response",
        );
        let outbound_parameters = doc
            .pointer("/paths/~1api~1v1~1outbound~1orders/post/parameters")
            .and_then(serde_json::Value::as_array)
            .expect("outbound order creation should declare parameters");
        assert!(
            outbound_parameters.iter().any(|parameter| {
                parameter.get("name") == Some(&serde_json::json!("Idempotency-Key"))
                    && parameter.get("in") == Some(&serde_json::json!("header"))
            }),
            "newer write contracts should keep Idempotency-Key header",
        );
    }
}
