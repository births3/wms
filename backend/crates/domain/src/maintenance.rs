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
    InvalidConclusion,
    /// 异常结论必须带 exception_type，才能进入质量联系/隔离闭环。
    AbnormalRequiresExceptionType,
    /// 正常结论不得附带 exception_type。
    NormalMustNotHaveExceptionType,
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
    match request.conclusion.as_str() {
        "normal" => {
            if request
                .exception_type
                .as_deref()
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .is_some()
            {
                return Err(MaintenanceRecordValidationError::NormalMustNotHaveExceptionType);
            }
        }
        "abnormal" => {
            let exception = request
                .exception_type
                .as_deref()
                .map(str::trim)
                .filter(|item| !item.is_empty());
            let Some(exception) = exception else {
                return Err(MaintenanceRecordValidationError::AbnormalRequiresExceptionType);
            };
            if !matches!(
                exception,
                "quality_change"
                    | "package_damage"
                    | "temperature_excursion"
                    | "pest_rodent_mildew"
                    | "other"
            ) {
                return Err(MaintenanceRecordValidationError::AbnormalRequiresExceptionType);
            }
        }
        _ => return Err(MaintenanceRecordValidationError::InvalidConclusion),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn base_request() -> CreateMaintenanceRecordRequest {
        CreateMaintenanceRecordRequest {
            task_id: Uuid::new_v4(),
            temperature_celsius: 5.0,
            humidity_percent: 40.0,
            appearance: "intact".to_string(),
            packaging: "intact".to_string(),
            pest: "none".to_string(),
            rodent: "none".to_string(),
            mildew: "none".to_string(),
            conclusion: "normal".to_string(),
            exception_type: None,
            notes: None,
        }
    }

    #[test]
    fn accepts_normal_and_abnormal_with_exception_type() {
        assert!(validate_create_maintenance_record_request(&base_request()).is_ok());
        let mut abnormal = base_request();
        abnormal.conclusion = "abnormal".to_string();
        abnormal.exception_type = Some("package_damage".to_string());
        assert!(validate_create_maintenance_record_request(&abnormal).is_ok());
    }

    #[test]
    fn rejects_abnormal_without_exception_type() {
        let mut abnormal = base_request();
        abnormal.conclusion = "abnormal".to_string();
        assert_eq!(
            validate_create_maintenance_record_request(&abnormal),
            Err(MaintenanceRecordValidationError::AbnormalRequiresExceptionType)
        );
    }
}
