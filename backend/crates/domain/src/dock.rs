use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DockAppointment {
    pub id: Uuid,
    pub dock_id: Uuid,
    pub owner_id: Uuid,
    pub warehouse_id: Uuid,
    pub status: String,
    pub appointment_no: String,
    pub document_type: String,
    pub document_no: String,
    pub window_start_at: DateTime<Utc>,
    pub window_end_at: DateTime<Utc>,
    pub vehicle_plate_no: Option<String>,
    pub vehicle_type: String,
    pub driver_name: String,
    pub driver_phone: String,
    pub supersedes_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
    pub arrived_at: Option<DateTime<Utc>>,
    pub arrival_deviation_minutes: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateDockAppointmentRequest {
    pub dock_id: Uuid,
    pub warehouse_id: Uuid,
    pub appointment_no: String,
    pub document_type: String,
    pub document_no: String,
    pub window_start_at: DateTime<Utc>,
    pub window_end_at: DateTime<Utc>,
    pub vehicle_plate_no: Option<String>,
    pub vehicle_type: String,
    pub driver_name: String,
    pub driver_phone: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateDockAppointmentRequest {
    pub dock_id: Uuid,
    pub window_start_at: DateTime<Utc>,
    pub window_end_at: DateTime<Utc>,
    pub vehicle_plate_no: Option<String>,
    pub vehicle_type: String,
    pub driver_name: String,
    pub driver_phone: String,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CancelDockAppointmentRequest {
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ArriveDockAppointmentRequest {
    pub appointment_no: String,
    pub vehicle_plate_no: String,
    pub driver_name: String,
    pub vehicle_type: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct Dock {
    pub id: Uuid,
    pub warehouse_id: Uuid,
    pub dock_code: String,
    pub dock_type: String,
    pub temperature_zone: String,
    pub status: String,
    pub maintenance_recovery_at: Option<DateTime<Utc>>,
    pub location_description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateDockRequest {
    pub warehouse_id: Uuid,
    pub dock_code: String,
    pub dock_type: String,
    pub temperature_zone: String,
    pub location_description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateDockImportRequest {
    pub warehouse_id: Uuid,
    pub docks: Vec<CreateDockRequest>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateDockRequest {
    pub status: String,
    pub maintenance_recovery_at: Option<DateTime<Utc>>,
}
