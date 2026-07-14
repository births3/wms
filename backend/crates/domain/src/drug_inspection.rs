use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::PageMeta;

pub const DRUG_INSPECTION_AUTH_METHODS: [&str; 2] = ["api_key", "username_password"];
pub const DRUG_INSPECTION_PLATFORM_STATUSES: [&str; 3] = ["connected", "testing", "disabled"];
pub const DRUG_INSPECTION_MIN_TIMEOUT_SECONDS: i32 = 1;
pub const DRUG_INSPECTION_MAX_TIMEOUT_SECONDS: i32 = 300;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DrugInspectionPlatform {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub platform_code: String,
    pub platform_name: String,
    pub api_url: String,
    pub auth_method: String,
    pub username: Option<String>,
    pub api_key_configured: bool,
    pub password_configured: bool,
    pub timeout_seconds: i32,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DrugInspectionPlatformListResponse {
    pub data: Vec<DrugInspectionPlatform>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpsertDrugInspectionPlatformRequest {
    pub platform_code: String,
    pub platform_name: String,
    pub api_url: String,
    pub auth_method: String,
    pub api_key_alias: Option<String>,
    pub username: Option<String>,
    pub password_alias: Option<String>,
    pub timeout_seconds: i32,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ChangeDrugInspectionPlatformStatusRequest {
    pub status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DrugInspectionConfigValidationError {
    FieldRequired(&'static str),
    FieldTooLong(&'static str),
    InvalidApiUrl,
    InvalidAuthMethod,
    InvalidCredentialReference,
    InvalidCredentialCombination,
    InvalidTimeout,
    InvalidStatus,
}

impl UpsertDrugInspectionPlatformRequest {
    pub fn validate(&self) -> Result<(), DrugInspectionConfigValidationError> {
        validate_text(&self.platform_code, "platform_code", 64)?;
        validate_text(&self.platform_name, "platform_name", 128)?;
        validate_api_url(&self.api_url)?;
        if !DRUG_INSPECTION_AUTH_METHODS.contains(&self.auth_method.trim()) {
            return Err(DrugInspectionConfigValidationError::InvalidAuthMethod);
        }
        if !(DRUG_INSPECTION_MIN_TIMEOUT_SECONDS..=DRUG_INSPECTION_MAX_TIMEOUT_SECONDS)
            .contains(&self.timeout_seconds)
        {
            return Err(DrugInspectionConfigValidationError::InvalidTimeout);
        }
        validate_status(&self.status)?;

        match self.auth_method.trim() {
            "api_key" => {
                if self
                    .username
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                    || self.password_alias.is_some()
                {
                    return Err(DrugInspectionConfigValidationError::InvalidCredentialCombination);
                }
                validate_secret_reference(&self.api_key_alias)?;
            }
            "username_password" => {
                if self.api_key_alias.is_some()
                    || self
                        .username
                        .as_deref()
                        .is_none_or(|value| value.trim().is_empty())
                {
                    return Err(DrugInspectionConfigValidationError::InvalidCredentialCombination);
                }
                validate_secret_reference(&self.password_alias)?;
            }
            _ => return Err(DrugInspectionConfigValidationError::InvalidAuthMethod),
        }
        Ok(())
    }
}

impl ChangeDrugInspectionPlatformStatusRequest {
    pub fn validate(&self) -> Result<(), DrugInspectionConfigValidationError> {
        validate_status(&self.status)
    }
}

fn validate_text(
    value: &str,
    field: &'static str,
    max_chars: usize,
) -> Result<(), DrugInspectionConfigValidationError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(DrugInspectionConfigValidationError::FieldRequired(field));
    }
    if value.chars().count() > max_chars {
        return Err(DrugInspectionConfigValidationError::FieldTooLong(field));
    }
    Ok(())
}

fn validate_api_url(value: &str) -> Result<(), DrugInspectionConfigValidationError> {
    let value = value.trim();
    let Some((scheme, authority_and_path)) = value.split_once("://") else {
        return Err(DrugInspectionConfigValidationError::InvalidApiUrl);
    };
    if !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https")
        || authority_and_path.is_empty()
        || value.chars().any(char::is_whitespace)
    {
        return Err(DrugInspectionConfigValidationError::InvalidApiUrl);
    }
    let authority = authority_and_path
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    if authority.is_empty() || authority.contains('@') {
        return Err(DrugInspectionConfigValidationError::InvalidApiUrl);
    }
    Ok(())
}

fn validate_secret_reference(
    value: &Option<String>,
) -> Result<(), DrugInspectionConfigValidationError> {
    let Some(value) = value.as_deref().map(str::trim) else {
        return Err(DrugInspectionConfigValidationError::FieldRequired(
            "credential_ref",
        ));
    };
    if !value.starts_with("vault://")
        || value.len() <= "vault://".len()
        || value.chars().any(char::is_whitespace)
    {
        return Err(DrugInspectionConfigValidationError::InvalidCredentialReference);
    }
    Ok(())
}

fn validate_status(value: &str) -> Result<(), DrugInspectionConfigValidationError> {
    if DRUG_INSPECTION_PLATFORM_STATUSES.contains(&value.trim()) {
        Ok(())
    } else {
        Err(DrugInspectionConfigValidationError::InvalidStatus)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ChangeDrugInspectionPlatformStatusRequest, DrugInspectionConfigValidationError,
        UpsertDrugInspectionPlatformRequest,
    };

    fn api_key_request() -> UpsertDrugInspectionPlatformRequest {
        UpsertDrugInspectionPlatformRequest {
            platform_code: "platform-a".to_string(),
            platform_name: "平台 A".to_string(),
            api_url: "https://inspection.example.test/api".to_string(),
            auth_method: "api_key".to_string(),
            api_key_alias: Some("vault://wms/di/platform-a/api-key".to_string()),
            username: None,
            password_alias: None,
            timeout_seconds: 30,
            status: "testing".to_string(),
        }
    }

    #[test]
    fn validates_api_key_and_username_password_credentials() {
        assert!(api_key_request().validate().is_ok());

        let mut account = api_key_request();
        account.auth_method = "username_password".to_string();
        account.api_key_alias = None;
        account.username = Some("di-user".to_string());
        account.password_alias = Some("vault://wms/di/platform-a/password".to_string());
        assert!(account.validate().is_ok());
    }

    #[test]
    fn rejects_invalid_url_auth_timeout_status_and_inline_secret() {
        let mut request = api_key_request();
        request.api_url = "ftp://inspection.example.test".to_string();
        assert_eq!(
            request.validate(),
            Err(DrugInspectionConfigValidationError::InvalidApiUrl)
        );

        request = api_key_request();
        request.api_key_alias = Some("plain-api-key".to_string());
        assert_eq!(
            request.validate(),
            Err(DrugInspectionConfigValidationError::InvalidCredentialReference)
        );

        request = api_key_request();
        request.timeout_seconds = 0;
        assert_eq!(
            request.validate(),
            Err(DrugInspectionConfigValidationError::InvalidTimeout)
        );

        request = api_key_request();
        request.status = "active".to_string();
        assert_eq!(
            request.validate(),
            Err(DrugInspectionConfigValidationError::InvalidStatus)
        );

        assert_eq!(
            ChangeDrugInspectionPlatformStatusRequest {
                status: "disabled".to_string(),
            }
            .validate(),
            Ok(())
        );
    }
}
