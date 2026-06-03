//! Wave 3 M5 cold-chain external data ingestion service.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;
use wms_domain::{
    ColdChainDevice, CreateColdChainDeviceRequest, IngestTemperatureExcursionRequest,
    IngestTemperatureReadingRequest, TemperatureExcursionEvent, TemperatureReading,
};

use crate::auth::AuthContext;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ColdChainError {
    DuplicateDevice(String),
    DeviceNotFound(String),
    FutureTimestamp,
}

#[derive(Clone, Debug, Default)]
pub struct ColdChainService {
    devices: BTreeMap<String, ColdChainDevice>,
    readings: BTreeMap<(String, DateTime<Utc>), TemperatureReading>,
    excursions: BTreeMap<String, TemperatureExcursionEvent>,
}

impl ColdChainService {
    pub fn create_device(
        &mut self,
        ctx: &AuthContext,
        req: CreateColdChainDeviceRequest,
        now: DateTime<Utc>,
    ) -> Result<ColdChainDevice, ColdChainError> {
        let key = device_key(ctx.owner_id, &req.device_code);
        if self.devices.contains_key(&key) {
            return Err(ColdChainError::DuplicateDevice(req.device_code));
        }
        let device = ColdChainDevice {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            device_code: req.device_code,
            device_type: req.device_type,
            installed_at_location_code: req.installed_at_location_code,
            calibration_due_at: req.calibration_due_at,
            status: "active".to_string(),
            created_at: now,
        };
        self.devices.insert(key, device.clone());
        Ok(device)
    }

    pub fn ingest_reading(
        &mut self,
        ctx: &AuthContext,
        req: IngestTemperatureReadingRequest,
        now: DateTime<Utc>,
    ) -> Result<TemperatureReading, ColdChainError> {
        if req.captured_at > now {
            return Err(ColdChainError::FutureTimestamp);
        }
        let key = device_key(ctx.owner_id, &req.device_code);
        if !self.devices.contains_key(&key) {
            return Err(ColdChainError::DeviceNotFound(req.device_code));
        }
        let reading_key = (key, req.captured_at);
        if let Some(existing) = self.readings.get(&reading_key) {
            return Ok(existing.clone());
        }
        let reading = TemperatureReading {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            device_code: req.device_code,
            temperature_celsius: req.temperature_celsius,
            humidity_percent: req.humidity_percent,
            captured_at: req.captured_at,
            external_report_url: req.external_report_url,
            out_of_range: req.out_of_range,
        };
        self.readings.insert(reading_key, reading.clone());
        Ok(reading)
    }

    pub fn ingest_excursion(
        &mut self,
        ctx: &AuthContext,
        req: IngestTemperatureExcursionRequest,
        now: DateTime<Utc>,
    ) -> Result<TemperatureExcursionEvent, ColdChainError> {
        if req.started_at > now {
            return Err(ColdChainError::FutureTimestamp);
        }
        let key = device_key(ctx.owner_id, &req.device_code);
        if !self.devices.contains_key(&key) {
            return Err(ColdChainError::DeviceNotFound(req.device_code));
        }
        let event_key = format!("{}:{}", ctx.owner_id, req.external_event_id);
        if let Some(existing) = self.excursions.get(&event_key) {
            return Ok(existing.clone());
        }

        let event = TemperatureExcursionEvent {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            external_event_id: req.external_event_id,
            device_code: req.device_code,
            location_code: req.location_code,
            started_at: req.started_at,
            ended_at: req.ended_at,
            min_temperature_celsius: req.min_temperature_celsius,
            max_temperature_celsius: req.max_temperature_celsius,
            affected_batch_ids: req.affected_batch_ids,
            status: "pending_disposition".to_string(),
            created_at: now,
        };
        self.excursions.insert(event_key, event.clone());
        Ok(event)
    }
}

fn device_key(owner_id: Uuid, device_code: &str) -> String {
    format!("{owner_id}:{device_code}")
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};
    use uuid::Uuid;
    use wms_domain::{
        CreateColdChainDeviceRequest, IngestTemperatureExcursionRequest,
        IngestTemperatureReadingRequest,
    };

    use super::{ColdChainError, ColdChainService};
    use crate::auth::AuthContext;

    fn ctx(owner_id: Uuid) -> AuthContext {
        AuthContext {
            user_id: Uuid::new_v4(),
            owner_id,
            actor_name: "tester".to_string(),
            permissions: vec!["m5.write".to_string()],
            jti: Uuid::new_v4().to_string(),
        }
    }

    #[test]
    fn external_temperature_readings_are_idempotent_and_owner_scoped() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 14, 0, 0)
            .single()
            .expect("valid time");
        let ctx = ctx(Uuid::new_v4());
        let mut service = ColdChainService::default();
        service
            .create_device(
                &ctx,
                CreateColdChainDeviceRequest {
                    device_code: "TEMP-001".to_string(),
                    device_type: "temperature_recorder".to_string(),
                    installed_at_location_code: Some("COLD-A".to_string()),
                    calibration_due_at: None,
                },
                now,
            )
            .expect("device");

        let req = IngestTemperatureReadingRequest {
            device_code: "TEMP-001".to_string(),
            temperature_celsius: 4.2,
            humidity_percent: Some(55.0),
            captured_at: now - Duration::minutes(1),
            external_report_url: Some("https://cold-chain.internal/report/1".to_string()),
            out_of_range: false,
        };
        let first = service
            .ingest_reading(&ctx, req.clone(), now)
            .expect("reading");
        let duplicate = service
            .ingest_reading(&ctx, req, now)
            .expect("same reading");

        assert_eq!(first.id, duplicate.id);
        assert_eq!(first.temperature_celsius, 4.2);
    }

    #[test]
    fn excursion_event_is_pending_disposition_and_not_auto_isolated() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 14, 0, 0)
            .single()
            .expect("valid time");
        let ctx = ctx(Uuid::new_v4());
        let mut service = ColdChainService::default();
        service
            .create_device(
                &ctx,
                CreateColdChainDeviceRequest {
                    device_code: "TEMP-002".to_string(),
                    device_type: "temperature_recorder".to_string(),
                    installed_at_location_code: Some("COLD-B".to_string()),
                    calibration_due_at: None,
                },
                now,
            )
            .expect("device");

        let event = service
            .ingest_excursion(
                &ctx,
                IngestTemperatureExcursionRequest {
                    external_event_id: "EXT-001".to_string(),
                    device_code: "TEMP-002".to_string(),
                    location_code: Some("COLD-B".to_string()),
                    started_at: now - Duration::minutes(20),
                    ended_at: Some(now - Duration::minutes(10)),
                    min_temperature_celsius: Some(1.0),
                    max_temperature_celsius: Some(12.0),
                    affected_batch_ids: vec![Uuid::new_v4()],
                },
                now,
            )
            .expect("event");

        assert_eq!(event.status, "pending_disposition");
    }

    #[test]
    fn future_temperature_reading_is_rejected() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 14, 0, 0)
            .single()
            .expect("valid time");
        let ctx = ctx(Uuid::new_v4());
        let mut service = ColdChainService::default();
        service
            .create_device(
                &ctx,
                CreateColdChainDeviceRequest {
                    device_code: "TEMP-003".to_string(),
                    device_type: "temperature_recorder".to_string(),
                    installed_at_location_code: None,
                    calibration_due_at: None,
                },
                now,
            )
            .expect("device");

        let result = service.ingest_reading(
            &ctx,
            IngestTemperatureReadingRequest {
                device_code: "TEMP-003".to_string(),
                temperature_celsius: 4.2,
                humidity_percent: None,
                captured_at: now + Duration::minutes(1),
                external_report_url: None,
                out_of_range: false,
            },
            now,
        );

        assert!(matches!(result, Err(ColdChainError::FutureTimestamp)));
    }
}
