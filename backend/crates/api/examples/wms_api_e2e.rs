//! Test-only HTTP entrypoint for real-data web-admin E2E.

use std::{env, error::Error, io, net::SocketAddr, path::PathBuf, sync::Arc};

use axum::middleware::from_fn_with_state;
use axum::{routing::get, Json, Router};
use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use uuid::Uuid;
use wms_api::{
    admin_menu_handlers::{admin_menu_router, AdminMenuAppState},
    alert_dashboard_handlers::{alert_dashboard_router, AlertDashboardAppState},
    alert_definition_handlers::{alert_definition_router, AlertDefinitionAppState},
    alert_escalation_handlers::{alert_escalation_router, AlertEscalationAppState},
    alert_instance_handlers::{alert_instance_router, AlertInstanceAppState},
    api_key_auth::{api_key_auth_middleware, ApiKeyAuthState},
    api_key_handlers::{api_key_router, ApiKeyManagementState},
    auth::{
        auth_runtime_layer, AuthRevocationStore, AuthRevocationStoreError, AuthRuntimePolicy,
        RedisAuthRevocationStore, JWT_SECRET_ENV,
    },
    auth_handlers::{auth_router, AuthAppState},
    config_center::{config_center_router, ConfigCenterAppState},
    dock_appointment_handlers::{dock_appointment_router, DockAppointmentAppState},
    dock_handlers::{dock_router, DockAppState},
    document_numbering_handlers::{document_numbering_router, DocumentNumberingAppState},
    drug_inspection_copy_service::spawn_drug_inspection_copy_worker,
    drug_inspection_document_handlers::{
        drug_inspection_document_router, DrugInspectionDocumentAppState,
    },
    drug_inspection_handlers::{drug_inspection_router, DrugInspectionAppState},
    drug_inspection_portal_bridge::spawn_drug_inspection_portal_bridge,
    drug_inspection_stamp_handlers::{drug_inspection_stamp_router, DrugInspectionStampAppState},
    dual_person_policy_handlers::{dual_person_policy_router, DualPersonPolicyAppState},
    express::{express_router, ExpressAppState},
    feature_flags::FeatureFlagRegistry,
    file_attachment::FileAttachmentService,
    file_attachment_handlers::{file_attachment_router, FileAttachmentAppState},
    h8_erp_connectors::{h8_erp_connector_router, H8ErpConnectorAppState},
    h8_erp_interface_tables::{h8_erp_interface_table_router, H8ErpInterfaceTableAppState},
    h8_erp_messages::{h8_erp_message_router, H8ErpMessageAppState},
    inventory_status_config_handlers::{
        inventory_status_config_router, InventoryStatusConfigAppState,
    },
    master_data_handlers::{master_data_router, MasterDataAppState},
    print_device_handlers::{print_device_router, PrintDeviceAppState},
    print_orchestration::CategoryPdfRenderer,
    print_orchestration_handlers::{print_orchestration_router, PrintOrchestrationAppState},
    print_template_handlers::{print_template_router, PrintTemplateAppState},
    quality_liaison_handlers::{quality_liaison_router, QualityLiaisonAppState},
    reconciliation_handlers::{reconciliation_router, ReconciliationAppState},
    role_management::{role_management_router, RoleManagementState},
    system_dictionary_handlers::{system_dictionary_router, SystemDictionaryAppState},
    task_engine_handlers::{task_engine_router, TaskEngineAppState},
    task_type_handlers::{task_type_router, TaskTypeAppState},
    wave3_handlers::{wave3_router, Wave3AppState},
    wave4_handlers::postgres_outbound,
    wave5_handlers::{wave5_router, Wave5AppState},
};
use wms_domain::HealthzResponse;

#[path = "support/wms_api_e2e_seed.rs"]
mod wms_api_e2e_seed;
#[path = "support/wms_api_e2e_seed_data.rs"]
mod wms_api_e2e_seed_data;
#[path = "support/wms_api_e2e_seed_h9.rs"]
mod wms_api_e2e_seed_h9;
#[path = "support/wms_api_e2e_seed_mrc.rs"]
mod wms_api_e2e_seed_mrc;

const BIND_ADDR_ENV: &str = "WMS_BIND_ADDR";
const DATABASE_URL_ENV: &str = "DATABASE_URL";
const WMS_DB_URL_ENV: &str = "WMS_DB_URL";
const E2E_SEED_ENV: &str = "WMS_E2E_SEED";
const REDIS_URL_ENV: &str = "WMS_REDIS_URL";
const DEFAULT_BIND_ADDR: &str = "127.0.0.1:19080";
const E2E_ATTACHMENT_ROOT_ENV: &str = "WMS_E2E_ATTACHMENT_ROOT";

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

    let revocation_store: Arc<dyn AuthRevocationStore> = match env::var(REDIS_URL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        Some(redis_url) => Arc::new(
            RedisAuthRevocationStore::from_url(&redis_url)
                .await
                .map_err(|error| io::Error::other(format!("Redis auth store: {error:?}")))?,
        ),
        None => Arc::new(AllowAllRevocationStore),
    };

    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&database_url()?)
        .await?;
    let seed_enabled = env::var(E2E_SEED_ENV).ok().as_deref() == Some("1");
    let h_file = if seed_enabled {
        FileAttachmentService::with_memory(pool.clone())
    } else {
        FileAttachmentService::from_env(pool.clone())
            .unwrap_or_else(|_| FileAttachmentService::disabled(pool.clone()))
    };
    if seed_enabled {
        sqlx::migrate!("../../migrations").run(&pool).await?;
        wms_api_e2e_seed_data::seed_e2e_data(&pool).await?;
        wms_api_e2e_seed_h9::seed_h9_file_attachments(&pool, &h_file).await?;
    }

    let h8_connector_state = H8ErpConnectorAppState::with_postgres(pool.clone());
    let attachment_root = env::var(E2E_ATTACHMENT_ROOT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::temp_dir().join("wms-drug-inspection-e2e-attachments"));
    spawn_drug_inspection_copy_worker(pool.clone(), attachment_root.clone());
    if let (Ok(portal_url), Ok(projection_key)) = (
        env::var("WMS_MDI_PORTAL_URL"),
        env::var("WMS_MDI_PORTAL_PROJECTION_KEY"),
    ) {
        spawn_drug_inspection_portal_bridge(pool.clone(), portal_url, projection_key);
    }
    let app = Router::new()
        .route("/api/v1/healthz", get(healthz))
        .merge(auth_router(AuthAppState::new(pool.clone())))
        .merge(api_key_router(ApiKeyManagementState::new(pool.clone())))
        .merge(admin_menu_router(AdminMenuAppState::with_postgres(
            pool.clone(),
        )))
        .merge(quality_liaison_router(
            QualityLiaisonAppState::with_postgres(pool.clone()),
        ))
        .merge(reconciliation_router(
            ReconciliationAppState::with_postgres(pool.clone()),
        ))
        .merge(alert_definition_router(
            AlertDefinitionAppState::with_postgres(pool.clone()),
        ))
        .merge(alert_dashboard_router(
            AlertDashboardAppState::with_postgres(pool.clone()),
        ))
        .merge(alert_escalation_router(
            AlertEscalationAppState::with_postgres(pool.clone()),
        ))
        .merge(alert_instance_router(AlertInstanceAppState::with_postgres(
            pool.clone(),
        )))
        .merge(config_center_router(ConfigCenterAppState::with_postgres(
            FeatureFlagRegistry::from_file(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../deploy/feature_flags.toml"),
            )
            .map_err(|error| io::Error::other(format!("feature flags: {error:?}")))?,
            pool.clone(),
        )))
        .merge(h8_erp_connector_router(h8_connector_state.clone()))
        .merge(h8_erp_interface_table_router(
            H8ErpInterfaceTableAppState::with_postgres(
                pool.clone(),
                h8_connector_state.repository.clone(),
            ),
        ))
        .merge(h8_erp_message_router(H8ErpMessageAppState::with_postgres(
            pool.clone(),
        )))
        .merge(drug_inspection_router(
            DrugInspectionAppState::with_postgres(pool.clone()),
        ))
        .merge(drug_inspection_document_router(
            DrugInspectionDocumentAppState::with_postgres(pool.clone()),
        ))
        .merge(drug_inspection_stamp_router(
            DrugInspectionStampAppState::with_local_storage(pool.clone(), attachment_root.clone()),
        ))
        .merge(file_attachment_router(
            FileAttachmentAppState::with_local_storage(pool.clone(), attachment_root),
        ))
        .merge(document_numbering_router(
            DocumentNumberingAppState::with_postgres(pool.clone()),
        ))
        .merge(dock_router(DockAppState::with_postgres(pool.clone())))
        .merge(dock_appointment_router(
            DockAppointmentAppState::with_postgres(pool.clone()),
        ))
        .merge(master_data_router(MasterDataAppState::with_postgres(
            pool.clone(),
        )))
        .merge(system_dictionary_router(
            SystemDictionaryAppState::with_postgres(pool.clone()),
        ))
        .merge(task_type_router(TaskTypeAppState::with_postgres(
            pool.clone(),
        )))
        .merge(task_engine_router(TaskEngineAppState::with_postgres(
            pool.clone(),
        )))
        .merge(dual_person_policy_router(
            DualPersonPolicyAppState::with_postgres(pool.clone()),
        ))
        // Real browser tests must mount the same inventory configuration route as production.
        .merge(inventory_status_config_router(
            InventoryStatusConfigAppState::with_postgres(pool.clone()),
        ))
        .merge(print_template_router(PrintTemplateAppState::with_postgres(
            pool.clone(),
        )))
        .merge(print_orchestration_router(
            PrintOrchestrationAppState::with_pdf_dependencies(
                pool.clone(),
                h_file,
                CategoryPdfRenderer::from_env(),
            ),
        ))
        .merge(print_device_router(PrintDeviceAppState::with_postgres(
            pool.clone(),
        )))
        .merge(role_management_router(RoleManagementState::new(
            pool.clone(),
            revocation_store.clone(),
        )))
        .merge(wave3_router(Wave3AppState::with_postgres(pool.clone())))
        .merge(express_router(ExpressAppState::with_postgres(pool.clone())))
        .merge(postgres_outbound(pool.clone()))
        .merge(wave5_router(Wave5AppState::with_postgres(pool.clone())))
        .layer(from_fn_with_state(
            ApiKeyAuthState::new(pool.clone()),
            api_key_auth_middleware,
        ))
        .layer(auth_runtime_layer(AuthRuntimePolicy::new(revocation_store)));

    let listener = TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn database_url() -> Result<String, io::Error> {
    env::var(DATABASE_URL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            env::var(WMS_DB_URL_ENV)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{DATABASE_URL_ENV} or {WMS_DB_URL_ENV} is required"),
            )
        })
}

async fn healthz() -> Json<HealthzResponse> {
    Json(HealthzResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        generated_at: Utc::now(),
    })
}

struct AllowAllRevocationStore;

#[axum::async_trait]
impl AuthRevocationStore for AllowAllRevocationStore {
    async fn jti_is_blacklisted(&self, _jti: &str) -> Result<bool, AuthRevocationStoreError> {
        Ok(false)
    }

    async fn permissions_changed_at(
        &self,
        _user_id: Uuid,
    ) -> Result<Option<i64>, AuthRevocationStoreError> {
        Ok(None)
    }

    async fn blacklist_jti(
        &self,
        _jti: &str,
        _ttl_seconds: u64,
    ) -> Result<(), AuthRevocationStoreError> {
        Ok(())
    }

    async fn set_permissions_changed_at(
        &self,
        _user_id: Uuid,
        _changed_at_unix: i64,
    ) -> Result<(), AuthRevocationStoreError> {
        Ok(())
    }
}
