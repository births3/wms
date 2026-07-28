use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    middleware::from_fn_with_state,
    response::{Html, IntoResponse, Response},
    routing::get,
    Extension, Json, Router,
};
use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use std::{env, error::Error, io, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::net::TcpListener;
use utoipa::OpenApi;
use wms_api::ApiDoc;
use wms_api::{
    admin_menu_handlers::{admin_menu_router, AdminMenuAppState},
    alert_dashboard_handlers::{alert_dashboard_router, AlertDashboardAppState},
    alert_definition_handlers::{alert_definition_router, AlertDefinitionAppState},
    alert_escalation_handlers::{alert_escalation_router, AlertEscalationAppState},
    alert_instance_handlers::{alert_instance_router, AlertInstanceAppState},
    api_key_auth::{api_key_auth_middleware, ApiKeyAuthState},
    api_key_handlers::{api_key_router, ApiKeyManagementState},
    auth::{auth_runtime_layer, AuthRuntimePolicy, RedisAuthRevocationStore, JWT_SECRET_ENV},
    auth_handlers::{auth_router, AuthAppState},
    config_center::{config_center_router, ConfigCenterAppState},
    dock_appointment_handlers::{dock_appointment_router, DockAppointmentAppState},
    dock_handlers::{dock_router, DockAppState},
    document_numbering_handlers::{document_numbering_router, DocumentNumberingAppState},
    drug_inspection_handlers::{drug_inspection_router, DrugInspectionAppState},
    dual_person_policy_handlers::{dual_person_policy_router, DualPersonPolicyAppState},
    express::{express_router, ExpressAppState},
    feature_flags::FeatureFlagRegistry,
    h2_lifecycle_handlers::{h2_lifecycle_router, H2LifecycleAppState},
    h8_erp_connectors::{h8_erp_connector_router, H8ErpConnectorAppState},
    h8_erp_interface_tables::{h8_erp_interface_table_router, H8ErpInterfaceTableAppState},
    h8_erp_messages::{h8_erp_message_router, H8ErpMessageAppState},
    h8_inbound::{h8_inbound_router, H8InboundAppState},
    inventory_status_config_handlers::{
        inventory_status_config_router, InventoryStatusConfigAppState,
    },
    master_data_handlers::{master_data_router, MasterDataAppState},
    parameter_mapping::{parameter_mapping_router, ParameterMappingAppState},
    print_device_handlers::{print_device_router, PrintDeviceAppState},
    print_orchestration_handlers::{print_orchestration_router, PrintOrchestrationAppState},
    print_template_handlers::{print_template_router, PrintTemplateAppState},
    quality_liaison_handlers::{quality_liaison_router, QualityLiaisonAppState},
    reconciliation_handlers::{reconciliation_router, ReconciliationAppState},
    reports_handlers::mount_reports,
    resilience::{resilience_middleware, resilience_status, ResilienceState},
    role_management::{role_management_router, RoleManagementState},
    state_machine::state_machine_router,
    stock_adjustment_handlers::{stock_adjustment_router, StockAdjustmentAppState},
    system_dictionary_handlers::{system_dictionary_router, SystemDictionaryAppState},
    task_engine_handlers::{task_engine_router, TaskEngineAppState},
    task_type_handlers::{task_type_router, TaskTypeAppState},
    wave3_handlers::{wave3_router, Wave3AppState},
    wave4_handlers::{wave4_router, Wave4AppState},
    wave5_handlers::{wave5_router, Wave5AppState},
    wechat_notify::{wechat_notify_router, WechatNotifyAppState},
};
use wms_domain::HealthzResponse;

#[path = "wms_api/audit_query.rs"]
mod wms_api_audit_query;
use wms_api_audit_query::{audit_query_router, AuditQueryState};
const BIND_ADDR_ENV: &str = "WMS_BIND_ADDR";
const REDIS_URL_ENV: &str = "WMS_REDIS_URL";
const DATABASE_URL_ENV: &str = "DATABASE_URL";
const WMS_DB_URL_ENV: &str = "WMS_DB_URL";
const DB_MAX_CONNECTIONS_ENV: &str = "WMS_DB_MAX_CONNECTIONS";
const FEATURE_FLAGS_FILE_ENV: &str = "WMS_FEATURE_FLAGS_FILE";
const DEFAULT_BIND_ADDR: &str = "0.0.0.0:8080";
const DEFAULT_DB_MAX_CONNECTIONS: u32 = 32;
const DEFAULT_FEATURE_FLAGS_FILE: &str = "deploy/feature_flags.toml";
const API_DOCS_MODE_ENV: &str = "WMS_API_DOCS_MODE";
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let bind_addr = env::var(BIND_ADDR_ENV)
        .unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_string())
        .parse::<SocketAddr>()?;
    let jwt_secret = env::var(JWT_SECRET_ENV).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{JWT_SECRET_ENV} is required"),
        )
    })?;
    if jwt_secret.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{JWT_SECRET_ENV} must not be empty"),
        )
        .into());
    }
    let redis_url = env::var(REDIS_URL_ENV).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{REDIS_URL_ENV} is required"),
        )
    })?;
    let database_url = database_url()?;
    let feature_flags_file = PathBuf::from(
        env::var(FEATURE_FLAGS_FILE_ENV).unwrap_or_else(|_| DEFAULT_FEATURE_FLAGS_FILE.to_string()),
    );
    let file_registry = FeatureFlagRegistry::from_file(&feature_flags_file).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "failed to load feature flags from {}: {error:?}",
                feature_flags_file.display()
            ),
        )
    })?;
    let redis_auth_store = RedisAuthRevocationStore::from_url(&redis_url)
        .await
        .map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("failed to configure Redis auth revocation store: {error:?}"),
            )
        })?;
    let dual_person_policy_cache = redis_auth_store.multiplexed_connection();
    let revocation_store = Arc::new(redis_auth_store);
    let pool = PgPoolOptions::new()
        .max_connections(database_max_connections()?)
        .after_connect(|connection, _metadata| {
            Box::pin(async move {
                sqlx::query("SET jit = off").execute(connection).await?;
                Ok(())
            })
        })
        .connect(&database_url)
        .await
        .map_err(|error| io::Error::other(format!("failed to connect PostgreSQL: {error:?}")))?;
    wms_api::api_key_expiry::spawn(pool.clone());
    wms_api::alert_engine_job::spawn(pool.clone());
    wms_api::inventory_expiry_job::spawn(pool.clone());
    wms_api::print_orchestration_job::spawn(pool.clone());
    wms_api::h8_erp_messages::spawn_maintenance_job(pool.clone()).await?;
    wms_api::task_release_job::spawn(pool.clone());
    let config_center_state = ConfigCenterAppState::with_postgres(file_registry, pool.clone());
    let auth_state = AuthAppState::new(pool.clone());
    let role_management_state = RoleManagementState::new(pool.clone(), revocation_store.clone());
    let audit_query_state = AuditQueryState { pool: pool.clone() };
    let master_data_state = MasterDataAppState::with_postgres(pool.clone());
    let system_dictionary_state = SystemDictionaryAppState::with_postgres_and_redis(
        pool.clone(),
        dual_person_policy_cache.clone(),
    );
    let wave3_state =
        Wave3AppState::with_postgres(pool.clone()).with_config_center(config_center_state.clone());
    let wave4_state = Wave4AppState::with_postgres(pool.clone());
    let wave5_state = Wave5AppState::with_postgres(pool.clone());
    let express_state = ExpressAppState::with_postgres(pool.clone());
    let app = app(
        config_center_state,
        auth_state,
        wave3_state,
        wave4_state,
        wave5_state,
        express_state,
        audit_query_state,
        master_data_state,
        system_dictionary_state,
        Some(dual_person_policy_cache),
    )
    .merge(role_management_router(role_management_state))
    .layer(auth_runtime_layer(AuthRuntimePolicy::new(revocation_store)));

    let listener = TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn database_url() -> Result<String, io::Error> {
    match env::var(DATABASE_URL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        Some(value) => Ok(value),
        None => env::var(WMS_DB_URL_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{DATABASE_URL_ENV} or {WMS_DB_URL_ENV} is required"),
                )
            }),
    }
}

fn database_max_connections() -> Result<u32, io::Error> {
    match env::var(DB_MAX_CONNECTIONS_ENV) {
        Ok(value) => value.trim().parse::<u32>().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{DB_MAX_CONNECTIONS_ENV} must be a positive integer"),
            )
        }),
        Err(env::VarError::NotPresent) => Ok(DEFAULT_DB_MAX_CONNECTIONS),
        Err(error) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("failed to read {DB_MAX_CONNECTIONS_ENV}: {error}"),
        )),
    }
    .and_then(|value| {
        if value == 0 {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{DB_MAX_CONNECTIONS_ENV} must be greater than 0"),
            ))
        } else {
            Ok(value)
        }
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ApiDocsMode {
    Development,
    Production,
}

impl ApiDocsMode {
    fn from_env() -> Self {
        match env::var(API_DOCS_MODE_ENV)
            .unwrap_or_else(|_| "development".to_string())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "production" => Self::Production,
            _ => Self::Development,
        }
    }

    fn requires_internal_ip(self) -> bool {
        matches!(self, Self::Production)
    }
}
fn app(
    config_center_state: ConfigCenterAppState,
    auth_state: AuthAppState,
    wave3_state: Wave3AppState,
    wave4_state: Wave4AppState,
    wave5_state: Wave5AppState,
    express_state: ExpressAppState,
    audit_query_state: AuditQueryState,
    master_data_state: MasterDataAppState,
    system_dictionary_state: SystemDictionaryAppState,
    dual_person_policy_cache: Option<redis::aio::MultiplexedConnection>,
) -> Router {
    let document_numbering_state =
        DocumentNumberingAppState::with_postgres(audit_query_state.pool.clone());
    let print_template_state = PrintTemplateAppState::with_postgres(audit_query_state.pool.clone());
    let admin_menu_state = AdminMenuAppState::with_postgres(audit_query_state.pool.clone());
    let h2_lifecycle_state = H2LifecycleAppState::with_postgres(audit_query_state.pool.clone());
    let wechat_notify_state = WechatNotifyAppState::with_postgres(audit_query_state.pool.clone());
    let parameter_mapping_state =
        ParameterMappingAppState::with_postgres(audit_query_state.pool.clone());
    let resilience_state =
        ResilienceState::from_env().with_audit_pool(audit_query_state.pool.clone());
    let api_key_auth_state = ApiKeyAuthState::new(audit_query_state.pool.clone());
    let shared_pool = audit_query_state.pool.clone();
    let dual_person_policy_state = match dual_person_policy_cache {
        Some(cache) => {
            DualPersonPolicyAppState::with_postgres_and_redis(shared_pool.clone(), cache)
        }
        None => DualPersonPolicyAppState::with_postgres(shared_pool.clone()),
    };
    let h8_connector_state = H8ErpConnectorAppState::with_postgres(shared_pool.clone());
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(healthz))
        .route("/api/v1/healthz", get(healthz))
        .route("/openapi.json", get(openapi_json))
        .route("/api-docs", get(api_docs))
        .route("/redoc", get(redoc_docs))
        .merge(
            Router::new()
                .route("/api/v1/resilience/status", get(resilience_status))
                .route("/metrics", get(metrics))
                .with_state(resilience_state.clone()),
        )
        .merge(auth_router(auth_state))
        .merge(api_key_router(ApiKeyManagementState::new(
            audit_query_state.pool.clone(),
        )))
        .merge(mount_reports(audit_query_state.pool.clone()))
        .merge(audit_query_router(audit_query_state))
        .merge(h2_lifecycle_router(h2_lifecycle_state))
        .merge(config_center_router(config_center_state))
        .merge(h8_erp_connector_router(h8_connector_state.clone()))
        .merge(h8_erp_interface_table_router(
            H8ErpInterfaceTableAppState::with_postgres(
                shared_pool.clone(),
                h8_connector_state.repository.clone(),
            ),
        ))
        .merge(h8_erp_message_router(H8ErpMessageAppState::with_postgres(
            shared_pool.clone(),
        )))
        .merge(h8_inbound_router(H8InboundAppState::with_postgres(
            shared_pool.clone(),
        )))
        .merge(drug_inspection_router(
            DrugInspectionAppState::with_postgres(shared_pool.clone()),
        ))
        .merge(dock_router(DockAppState::with_postgres(
            shared_pool.clone(),
        )))
        .merge(dock_appointment_router(
            DockAppointmentAppState::with_postgres(shared_pool.clone()),
        ))
        .merge(master_data_router(master_data_state))
        .merge(parameter_mapping_router(parameter_mapping_state))
        .merge(admin_menu_router(admin_menu_state))
        .merge(system_dictionary_router(system_dictionary_state))
        .merge(task_type_router(TaskTypeAppState::with_postgres(
            shared_pool.clone(),
        )))
        .merge(task_engine_router(TaskEngineAppState::with_postgres(
            shared_pool.clone(),
        )))
        .merge(dual_person_policy_router(dual_person_policy_state))
        .merge(inventory_status_config_router(
            InventoryStatusConfigAppState::with_postgres(shared_pool.clone()),
        ))
        .merge(state_machine_router())
        .merge(stock_adjustment_router(
            StockAdjustmentAppState::with_postgres(shared_pool.clone()),
        ))
        .merge(quality_liaison_router(
            QualityLiaisonAppState::with_postgres(shared_pool.clone()),
        ))
        .merge(reconciliation_router(
            ReconciliationAppState::with_postgres(shared_pool.clone()),
        ))
        .merge(alert_definition_router(
            AlertDefinitionAppState::with_postgres(shared_pool.clone()),
        ))
        .merge(alert_dashboard_router(
            AlertDashboardAppState::with_postgres(shared_pool.clone()),
        ))
        .merge(alert_escalation_router(
            AlertEscalationAppState::with_postgres(shared_pool.clone()),
        ))
        .merge(alert_instance_router(AlertInstanceAppState::with_postgres(
            shared_pool.clone(),
        )))
        .merge(document_numbering_router(document_numbering_state))
        .merge(print_template_router(print_template_state))
        .merge(print_orchestration_router(
            PrintOrchestrationAppState::with_postgres(shared_pool.clone()),
        ))
        .merge(print_device_router(PrintDeviceAppState::with_postgres(
            shared_pool.clone(),
        )))
        .merge(wechat_notify_router(wechat_notify_state))
        .merge(express_router(express_state))
        .merge(wave3_router(wave3_state))
        .merge(wave4_router(wave4_state))
        .merge(wave5_router(wave5_state))
        .layer(Extension(ApiDocsMode::from_env()))
        .layer(from_fn_with_state(resilience_state, resilience_middleware))
        .layer(from_fn_with_state(
            api_key_auth_state,
            api_key_auth_middleware,
        ))
}

async fn healthz() -> Json<HealthzResponse> {
    Json(HealthzResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        generated_at: Utc::now(),
    })
}

async fn openapi_json(Extension(mode): Extension<ApiDocsMode>, headers: HeaderMap) -> Response {
    if !docs_internal_access_allowed(&headers, mode) {
        return StatusCode::FORBIDDEN.into_response();
    }
    Json(ApiDoc::openapi()).into_response()
}

async fn api_docs(Extension(mode): Extension<ApiDocsMode>, headers: HeaderMap) -> Response {
    if mode == ApiDocsMode::Production {
        return StatusCode::NOT_FOUND.into_response();
    }
    if !docs_internal_access_allowed(&headers, mode) {
        return StatusCode::FORBIDDEN.into_response();
    }
    Html(
        r##"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <title>WMS API Docs</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>window.ui = SwaggerUIBundle({ url: "/openapi.json", dom_id: "#swagger-ui" });</script>
</body>
</html>"##,
    )
    .into_response()
}

async fn redoc_docs(Extension(mode): Extension<ApiDocsMode>, headers: HeaderMap) -> Response {
    if !docs_internal_access_allowed(&headers, mode) {
        return StatusCode::FORBIDDEN.into_response();
    }
    Html(
        r##"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <title>WMS API ReDoc</title>
</head>
<body>
  <redoc spec-url="/openapi.json"></redoc>
  <script src="https://cdn.jsdelivr.net/npm/redoc@next/bundles/redoc.standalone.js"></script>
</body>
</html>"##,
    )
    .into_response()
}

async fn metrics(
    Extension(mode): Extension<ApiDocsMode>,
    State(state): State<ResilienceState>,
    headers: HeaderMap,
) -> Response {
    if !docs_internal_access_allowed(&headers, mode) {
        return StatusCode::FORBIDDEN.into_response();
    }
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        state.metrics_text(),
    )
        .into_response()
}

fn docs_internal_access_allowed(headers: &HeaderMap, mode: ApiDocsMode) -> bool {
    let Some(ip) = docs_request_ip(headers) else {
        return !mode.requires_internal_ip();
    };
    ip_is_internal(&ip)
}

fn docs_request_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .map(str::to_string)
}

fn ip_is_internal(ip: &str) -> bool {
    ip.starts_with("10.")
        || ip.starts_with("192.168.")
        || ip.starts_with("127.")
        || ip == "::1"
        || ip
            .strip_prefix("172.")
            .and_then(|rest| rest.split('.').next())
            .and_then(|octet| octet.parse::<u8>().ok())
            .is_some_and(|octet| (16..=31).contains(&octet))
}

#[cfg(test)]
mod tests {
    include!("wms_api/wms_api_part1.rs");
    include!("wms_api/wms_api_part2.rs");
    include!("wms_api/wms_api_part3.rs");
}
