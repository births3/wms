//! Wave 5 M10 TMS+ integration boundary rules.

use chrono::{DateTime, Utc};
use wms_domain::{
    ConfirmContainerRecoveryRequest, IngestTransitTemperatureRequest, ReceiveTmsDispatchRequest,
};

#[derive(Clone, Debug, Default)]
pub struct TmsPlusService;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TmsPlusError {
    InvalidDispatch,
    InvalidTimestamp,
    InvalidContainerRecovery,
}

impl TmsPlusService {
    pub fn validate_dispatch(&self, req: &ReceiveTmsDispatchRequest) -> Result<(), TmsPlusError> {
        if req.dispatch_no.trim().is_empty()
            || req.delivery_provider_type.trim().is_empty()
            || req.version <= 0
        {
            return Err(TmsPlusError::InvalidDispatch);
        }
        match req.delivery_provider_type.as_str() {
            "own_fleet" if option_blank(&req.vehicle_no) || req.driver_user_id.is_none() => {
                Err(TmsPlusError::InvalidDispatch)
            }
            "third_party_express" if option_blank(&req.carrier_code) => {
                Err(TmsPlusError::InvalidDispatch)
            }
            "own_fleet" | "third_party_express" => Ok(()),
            _ => Err(TmsPlusError::InvalidDispatch),
        }
    }

    pub fn validate_temperature(
        &self,
        req: &IngestTransitTemperatureRequest,
        now: DateTime<Utc>,
    ) -> Result<(), TmsPlusError> {
        if req.device_code.trim().is_empty()
            || req.plate_no.trim().is_empty()
            || req.measured_at > now + chrono::Duration::minutes(5)
        {
            return Err(TmsPlusError::InvalidTimestamp);
        }
        Ok(())
    }

    pub fn validate_recovery(
        &self,
        req: &ConfirmContainerRecoveryRequest,
    ) -> Result<(), TmsPlusError> {
        if req.container_lpn.trim().is_empty() || req.delivery_provider_type.trim().is_empty() {
            return Err(TmsPlusError::InvalidContainerRecovery);
        }
        Ok(())
    }
}

fn option_blank(value: &Option<String>) -> bool {
    value.as_deref().is_none_or(|item| item.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;
    use wms_domain::{
        ConfirmContainerRecoveryRequest, IngestTransitTemperatureRequest, ReceiveTmsDispatchRequest,
    };

    use super::{TmsPlusError, TmsPlusService};

    #[test]
    fn own_fleet_dispatch_requires_vehicle_and_driver() {
        let service = TmsPlusService;
        let req = ReceiveTmsDispatchRequest {
            dispatch_no: "DSP-001".to_string(),
            outbound_order_id: Uuid::new_v4(),
            delivery_provider_type: "own_fleet".to_string(),
            vehicle_no: None,
            plate_no: Some("浙A12345".to_string()),
            driver_user_id: None,
            carrier_code: None,
            waybill_no: None,
            version: 1,
            scheduled_load_at: None,
        };

        assert_eq!(
            service.validate_dispatch(&req),
            Err(TmsPlusError::InvalidDispatch)
        );
    }

    #[test]
    fn third_party_dispatch_rejects_blank_carrier() {
        let service = TmsPlusService;
        let req = ReceiveTmsDispatchRequest {
            dispatch_no: "DSP-001".to_string(),
            outbound_order_id: Uuid::new_v4(),
            delivery_provider_type: "third_party_express".to_string(),
            vehicle_no: None,
            plate_no: Some("浙A12345".to_string()),
            driver_user_id: None,
            carrier_code: Some(" ".to_string()),
            waybill_no: None,
            version: 1,
            scheduled_load_at: None,
        };

        assert_eq!(
            service.validate_dispatch(&req),
            Err(TmsPlusError::InvalidDispatch)
        );
    }

    #[test]
    fn future_temperature_reading_is_rejected() {
        let service = TmsPlusService;
        let now = Utc
            .with_ymd_and_hms(2026, 6, 5, 8, 0, 0)
            .single()
            .expect("valid time");
        let req = IngestTransitTemperatureRequest {
            dispatch_id: Uuid::new_v4(),
            device_code: "TEMP-001".to_string(),
            plate_no: "浙A12345".to_string(),
            measured_at: now + chrono::Duration::hours(1),
            temperature_celsius: 4.0,
            humidity_percent: None,
            is_exceeded: false,
            external_trace_url: None,
        };

        assert_eq!(
            service.validate_temperature(&req, now),
            Err(TmsPlusError::InvalidTimestamp)
        );
    }

    #[test]
    fn container_recovery_requires_lpn() {
        let service = TmsPlusService;
        assert_eq!(
            service.validate_recovery(&ConfirmContainerRecoveryRequest {
                container_lpn: "".to_string(),
                dispatch_id: None,
                customer_id: Uuid::new_v4(),
                delivery_provider_type: "own_fleet".to_string(),
                shipped_at: None,
            }),
            Err(TmsPlusError::InvalidContainerRecovery)
        );
    }
}
