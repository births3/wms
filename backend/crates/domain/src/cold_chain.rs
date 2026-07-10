use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{common::PageMeta, operations::InventoryBatch};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ColdChainDevice {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub device_code: String,
    pub device_type: String,
    pub installed_at_location_code: Option<String>,
    pub calibration_due_at: Option<DateTime<Utc>>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateColdChainDeviceRequest {
    pub device_code: String,
    pub device_type: String,
    pub installed_at_location_code: Option<String>,
    pub calibration_due_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TemperatureReading {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub device_code: String,
    pub temperature_celsius: f64,
    pub humidity_percent: Option<f64>,
    pub captured_at: DateTime<Utc>,
    pub external_report_url: Option<String>,
    pub out_of_range: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct IngestTemperatureReadingRequest {
    pub device_code: String,
    pub temperature_celsius: f64,
    pub humidity_percent: Option<f64>,
    pub captured_at: DateTime<Utc>,
    pub external_report_url: Option<String>,
    pub out_of_range: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TemperatureExcursionEvent {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub external_event_id: String,
    pub device_code: String,
    pub location_code: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub min_temperature_celsius: Option<f64>,
    pub max_temperature_celsius: Option<f64>,
    pub affected_batch_ids: Vec<Uuid>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TemperatureExcursionEventListResponse {
    pub data: Vec<TemperatureExcursionEvent>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct IngestTemperatureExcursionRequest {
    pub external_event_id: String,
    pub device_code: String,
    pub location_code: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub min_temperature_celsius: Option<f64>,
    pub max_temperature_celsius: Option<f64>,
    pub affected_batch_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DisposeTemperatureExcursionRequest {
    pub selected_batch_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TemperatureExcursionDispositionResponse {
    pub event: TemperatureExcursionEvent,
    pub quarantined_batches: Vec<InventoryBatch>,
    pub approval_source: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TransitTemperatureReading {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub dispatch_id: Uuid,
    pub device_code: String,
    pub plate_no: String,
    pub measured_at: DateTime<Utc>,
    pub temperature_celsius: f64,
    pub humidity_percent: Option<f64>,
    pub is_exceeded: bool,
    pub external_trace_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct IngestTransitTemperatureRequest {
    pub dispatch_id: Uuid,
    pub device_code: String,
    pub plate_no: String,
    pub measured_at: DateTime<Utc>,
    pub temperature_celsius: f64,
    pub humidity_percent: Option<f64>,
    pub is_exceeded: bool,
    pub external_trace_url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ContainerRecovery {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub container_lpn: String,
    pub dispatch_id: Option<Uuid>,
    pub customer_id: Uuid,
    pub delivery_provider_type: String,
    pub status: String,
    pub shipped_at: DateTime<Utc>,
    pub recovered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ConfirmContainerRecoveryRequest {
    pub container_lpn: String,
    pub dispatch_id: Option<Uuid>,
    pub customer_id: Uuid,
    pub delivery_provider_type: String,
    pub shipped_at: Option<DateTime<Utc>>,
}
