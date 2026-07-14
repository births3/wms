use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::PageMeta;

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct MaintenanceTaskQuery {
    pub task_id: Option<Uuid>,
    pub batch_id: Option<Uuid>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MaintenanceTask {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub batch_id: Uuid,
    pub product_code: String,
    pub batch_no: String,
    pub expiry_date: NaiveDate,
    pub quality_status: String,
    pub location_id: Uuid,
    pub location_code: String,
    pub planned_at: DateTime<Utc>,
    pub status: String,
    pub assigned_user_id: Option<Uuid>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MaintenanceTaskListResponse {
    pub data: Vec<MaintenanceTask>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct MaintenanceRecordQuery {
    pub task_id: Option<Uuid>,
    pub batch_id: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateMaintenanceRecordRequest {
    pub task_id: Uuid,
    pub temperature_celsius: f64,
    pub humidity_percent: f64,
    pub appearance: String,
    pub packaging: String,
    pub pest: String,
    pub rodent: String,
    pub mildew: String,
    pub conclusion: String,
    pub exception_type: Option<String>,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MaintenanceRecord {
    pub id: Uuid,
    pub task_id: Uuid,
    pub owner_id: Uuid,
    pub batch_id: Uuid,
    pub product_code: String,
    pub batch_no: String,
    pub expiry_date: NaiveDate,
    pub inventory_status: String,
    pub temperature_celsius: f64,
    pub humidity_percent: f64,
    pub appearance: String,
    pub packaging: String,
    pub pest: String,
    pub rodent: String,
    pub mildew: String,
    pub conclusion: String,
    pub exception_type: Option<String>,
    pub notes: Option<String>,
    pub performed_by: Uuid,
    pub performed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MaintenanceRecordListResponse {
    pub data: Vec<MaintenanceRecord>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MaintenanceRecordValidationError {
    InvalidTemperature,
    InvalidHumidity,
    InvalidAppearance,
    InvalidPackaging,
    InvalidPest,
    InvalidRodent,
    InvalidMildew,
    AbnormalRequiresQualityWorkflow,
}

pub fn validate_create_maintenance_record_request(
    request: &CreateMaintenanceRecordRequest,
) -> Result<(), MaintenanceRecordValidationError> {
    if !request.temperature_celsius.is_finite()
        || !(-100.0..=100.0).contains(&request.temperature_celsius)
    {
        return Err(MaintenanceRecordValidationError::InvalidTemperature);
    }
    if !request.humidity_percent.is_finite() || !(0.0..=100.0).contains(&request.humidity_percent) {
        return Err(MaintenanceRecordValidationError::InvalidHumidity);
    }
    if !matches!(
        request.appearance.as_str(),
        "intact" | "damaged" | "discolored" | "damp"
    ) {
        return Err(MaintenanceRecordValidationError::InvalidAppearance);
    }
    if !matches!(
        request.packaging.as_str(),
        "intact" | "damaged" | "leaking" | "label_unclear"
    ) {
        return Err(MaintenanceRecordValidationError::InvalidPackaging);
    }
    if !matches!(request.pest.as_str(), "none" | "present") {
        return Err(MaintenanceRecordValidationError::InvalidPest);
    }
    if !matches!(request.rodent.as_str(), "none" | "present") {
        return Err(MaintenanceRecordValidationError::InvalidRodent);
    }
    if !matches!(request.mildew.as_str(), "none" | "present") {
        return Err(MaintenanceRecordValidationError::InvalidMildew);
    }
    if request.conclusion != "normal" || request.exception_type.is_some() {
        return Err(MaintenanceRecordValidationError::AbnormalRequiresQualityWorkflow);
    }
    Ok(())
}
