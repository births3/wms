use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use wms_api::{
    alert_dashboard_handlers::{alert_dashboard_router, AlertDashboardAppState},
    alert_escalation_handlers::{alert_escalation_router, AlertEscalationAppState},
    alert_instance_handlers::{alert_instance_router, AlertInstanceAppState},
    h8_erp_connectors::{h8_erp_connector_router, H8ErpConnectorAppState},
    h8_erp_interface_tables::{h8_erp_interface_table_router, H8ErpInterfaceTableAppState},
    h8_erp_messages::{h8_erp_message_router, H8ErpMessageAppState},
    stock_adjustment_handlers::{stock_adjustment_router, StockAdjustmentAppState},
    wave3_handlers::{wave3_router, Wave3AppState},
};

fn protected_routes() -> Router {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://wms:wms@127.0.0.1:1/wms")
        .expect("lazy test pool");
    let connectors = H8ErpConnectorAppState::with_memory();
    let interface_tables = H8ErpInterfaceTableAppState::with_memory(connectors.repository.clone());

    alert_instance_router(AlertInstanceAppState::with_postgres(pool.clone()))
        .merge(alert_escalation_router(
            AlertEscalationAppState::with_postgres(pool.clone()),
        ))
        .merge(alert_dashboard_router(
            AlertDashboardAppState::with_postgres(pool.clone()),
        ))
        .merge(wave3_router(Wave3AppState::with_postgres(pool.clone())))
        .merge(stock_adjustment_router(
            StockAdjustmentAppState::with_postgres(pool),
        ))
        .merge(h8_erp_connector_router(connectors))
        .merge(h8_erp_interface_table_router(interface_tables))
        .merge(h8_erp_message_router(H8ErpMessageAppState::with_memory()))
}

#[tokio::test]
async fn protected_routes_reject_unauthenticated_http_requests() {
    let app = protected_routes();
    let routes = [
        (Method::GET, "/api/v1/alerts"),
        (Method::GET, "/api/v1/alerts/00000000-0000-0000-0000-000000000001"),
        (Method::POST, "/api/v1/alerts/00000000-0000-0000-0000-000000000001/acknowledge"),
        (Method::POST, "/api/v1/alerts/00000000-0000-0000-0000-000000000001/handling"),
        (Method::POST, "/api/v1/alerts/00000000-0000-0000-0000-000000000001/close"),
        (Method::POST, "/api/v1/alerts/00000000-0000-0000-0000-000000000001/ignore"),
        (Method::GET, "/api/v1/alert-escalation-rules"),
        (Method::PUT, "/api/v1/alert-escalation-rules/default"),
        (Method::GET, "/api/v1/alerts/active"),
        (Method::GET, "/api/v1/alerts/statistics"),
        (Method::GET, "/api/v1/alerts/gsp-report"),
        (Method::GET, "/api/v1/alerts/changes"),
        (Method::POST, "/api/v1/alerts/exports"),
        (Method::GET, "/api/v1/alerts/exports/00000000-0000-0000-0000-000000000001"),
        (Method::GET, "/api/v1/alerts/exports/00000000-0000-0000-0000-000000000001/download"),
        (Method::POST, "/api/v1/inbound/receiving-orders/00000000-0000-0000-0000-000000000001/cancel"),
        (Method::POST, "/api/v1/inbound/receiving-orders/00000000-0000-0000-0000-000000000001/force-close-shortage"),
        (Method::GET, "/api/v1/inbound/putaway-strategy-profiles"),
        (Method::GET, "/api/v1/inventory/relocations"),
        (Method::GET, "/api/v1/inventory/alerts"),
        (Method::POST, "/api/v1/inventory/alerts/00000000-0000-0000-0000-000000000001/handle"),
        (Method::POST, "/api/v1/inventory/alerts/generate-near-expiry"),
        (Method::GET, "/api/v1/inventory/abc"),
        (Method::POST, "/api/v1/inventory/abc/override"),
        (Method::GET, "/api/v1/inventory/batches/00000000-0000-0000-0000-000000000001/shipped-customers"),
        (Method::POST, "/api/v1/inventory/status-erp-outbox/process"),
        (Method::POST, "/api/v1/inventory/maintenance/tasks/generate"),
        (Method::GET, "/api/v1/h8/erp-interface-tables/connectors"),
        (Method::GET, "/api/v1/h8/erp-interface-tables/rows"),
        (Method::GET, "/api/v1/h8/erp-interface-tables/rows/row-1"),
        (Method::GET, "/api/v1/config/erp-connectors"),
        (Method::GET, "/api/v1/config/erp-connectors/00000000-0000-0000-0000-000000000001"),
        (Method::GET, "/api/v1/config/erp-connectors/00000000-0000-0000-0000-000000000001/versions/1"),
        (Method::POST, "/api/v1/config/erp-connectors/00000000-0000-0000-0000-000000000001/test"),
        (Method::POST, "/api/v1/config/erp-connectors/00000000-0000-0000-0000-000000000001/activate"),
        (Method::POST, "/api/v1/config/erp-connectors/00000000-0000-0000-0000-000000000001/disable"),
        (Method::GET, "/api/v1/integration/erp-messages/stats"),
        (Method::GET, "/api/v1/integration/erp-messages/payload-retention"),
        (Method::GET, "/api/v1/integration/erp-messages/00000000-0000-0000-0000-000000000001/payload"),
        (Method::GET, "/api/v1/integration/erp-messages/worker-runtime"),
        (Method::POST, "/api/v1/integration/erp-messages/worker-runtime/heartbeat"),
        (Method::POST, "/api/v1/integration/erp-messages/worker-runtime/control"),
        (Method::GET, "/api/v1/integration/erp-messages/worker-runtime/claim-decision"),
        (Method::POST, "/api/v1/integration/erp-messages/00000000-0000-0000-0000-000000000001/claim"),
        (Method::POST, "/api/v1/stock-adjustments/loss-orders/00000000-0000-0000-0000-000000000001/quality-approval"),
        (Method::POST, "/api/v1/stock-adjustments/loss-orders/00000000-0000-0000-0000-000000000001/start"),
        (Method::POST, "/api/v1/stock-adjustments/loss-orders/00000000-0000-0000-0000-000000000001/execute"),
        (Method::GET, "/api/v1/stock-adjustments/surplus-orders/00000000-0000-0000-0000-000000000001"),
        (Method::POST, "/api/v1/stock-adjustments/surplus-orders/00000000-0000-0000-0000-000000000001/execute"),
    ];

    for (method, uri) in routes {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .expect("request"),
            )
            .await
            .expect("route response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{uri}");
    }
}
