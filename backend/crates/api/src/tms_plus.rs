//! Wave 5 M10 TMS+ integration boundary rules.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use wms_domain::{
    ConfirmContainerRecoveryRequest, IngestTransitTemperatureRequest, ReceiveTmsDispatchRequest,
};

#[derive(Clone, Debug, Default)]
pub struct TmsPlusService;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TmsPlusError {
    InvalidDispatch,
    InvalidRoutePlan,
    InvalidTimestamp,
    InvalidContainerRecovery,
}

/// M10-001 的 TMS 路径规划结果。路径算法和车辆排班仍由外部 TMS 负责。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReceiveTmsRoutePlanRequest {
    pub dispatch_result_id: String,
    pub delivery_date: NaiveDate,
    pub vehicle_no: String,
    pub plate_no: String,
    pub driver_user_id: Uuid,
    pub version: i32,
    pub outbound_order_ids: Vec<Uuid>,
    pub stops: Vec<TmsRouteStopRequest>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TmsRouteStopRequest {
    pub store_id: Uuid,
    pub sequence: i32,
    pub estimated_arrival_at: DateTime<Utc>,
    pub outbound_order_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TmsRoutePlan {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub dispatch_result_id: String,
    pub delivery_date: NaiveDate,
    pub vehicle_no: String,
    pub plate_no: String,
    pub driver_user_id: Uuid,
    pub status: String,
    pub version: i32,
    pub outbound_order_ids: Vec<Uuid>,
    pub stops: Vec<TmsRouteStop>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TmsRouteStop {
    pub id: Uuid,
    pub store_id: Uuid,
    pub sequence: i32,
    pub estimated_arrival_at: DateTime<Utc>,
    pub outbound_order_ids: Vec<Uuid>,
}

impl TmsPlusService {
    pub fn validate_route_plan(
        &self,
        req: &ReceiveTmsRoutePlanRequest,
    ) -> Result<(), TmsPlusError> {
        use std::collections::HashSet;

        if req.dispatch_result_id.trim().is_empty()
            || req.dispatch_result_id.len() > 128
            || req.vehicle_no.trim().is_empty()
            || req.plate_no.trim().is_empty()
            || req.driver_user_id.is_nil()
            || req.version <= 0
            || req.outbound_order_ids.is_empty()
            || req.stops.is_empty()
        {
            return Err(TmsPlusError::InvalidRoutePlan);
        }

        let order_ids: HashSet<Uuid> = req.outbound_order_ids.iter().copied().collect();
        if order_ids.len() != req.outbound_order_ids.len() {
            return Err(TmsPlusError::InvalidRoutePlan);
        }

        let mut routed_order_ids = HashSet::new();
        let mut previous_arrival_at = None;
        for (index, stop) in req.stops.iter().enumerate() {
            if stop.store_id.is_nil()
                || stop.sequence != i32::try_from(index + 1).unwrap_or_default()
                || stop.outbound_order_ids.is_empty()
                || previous_arrival_at.is_some_and(|value| stop.estimated_arrival_at <= value)
            {
                return Err(TmsPlusError::InvalidRoutePlan);
            }
            previous_arrival_at = Some(stop.estimated_arrival_at);
            for order_id in &stop.outbound_order_ids {
                if !order_ids.contains(order_id) || !routed_order_ids.insert(*order_id) {
                    return Err(TmsPlusError::InvalidRoutePlan);
                }
            }
        }
        if routed_order_ids != order_ids {
            return Err(TmsPlusError::InvalidRoutePlan);
        }
        Ok(())
    }

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
    use chrono::{NaiveDate, TimeZone, Utc};
    use uuid::Uuid;
    use wms_domain::{
        ConfirmContainerRecoveryRequest, IngestTransitTemperatureRequest, ReceiveTmsDispatchRequest,
    };

    use super::{ReceiveTmsRoutePlanRequest, TmsPlusError, TmsPlusService, TmsRouteStopRequest};

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
    fn route_plan_requires_contiguous_stops_and_exact_order_coverage() {
        let service = TmsPlusService;
        let order_id = Uuid::new_v4();
        let request = ReceiveTmsRoutePlanRequest {
            dispatch_result_id: "TMS-RESULT-001".to_string(),
            delivery_date: NaiveDate::from_ymd_opt(2026, 7, 14).expect("valid date"),
            vehicle_no: "V-001".to_string(),
            plate_no: "沪A12345".to_string(),
            driver_user_id: Uuid::new_v4(),
            version: 1,
            outbound_order_ids: vec![order_id],
            stops: vec![TmsRouteStopRequest {
                store_id: Uuid::new_v4(),
                sequence: 2,
                estimated_arrival_at: Utc::now(),
                outbound_order_ids: vec![order_id],
            }],
        };

        assert_eq!(
            service.validate_route_plan(&request),
            Err(TmsPlusError::InvalidRoutePlan)
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
