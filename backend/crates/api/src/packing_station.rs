//! Wave 5 M-PK packing station business rules.

use wms_domain::{
    CreatePackJobRequest, CreatePackingStationRequest, PrintWaybillRequest, WeighPackJobRequest,
};

#[derive(Clone, Debug, Default)]
pub struct PackingStationService;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackingStationError {
    InvalidStation,
    InvalidPackJob,
    WeightOutOfTolerance,
    InvalidWaybill,
}

impl PackingStationService {
    pub fn validate_station(
        &self,
        req: &CreatePackingStationRequest,
    ) -> Result<(), PackingStationError> {
        if req.station_code.trim().is_empty()
            || req.station_name.trim().is_empty()
            || req.temperature_zone.trim().is_empty()
        {
            return Err(PackingStationError::InvalidStation);
        }
        Ok(())
    }

    pub fn validate_pack_job(&self, req: &CreatePackJobRequest) -> Result<(), PackingStationError> {
        if req.outbound_order_id.is_nil()
            || req.station_id.is_some_and(|station_id| station_id.is_nil())
            || req.job_no.trim().is_empty()
            || req.pack_mode.trim().is_empty()
            || req.recommended_box_type.trim().is_empty()
            || req.actual_box_type.trim().is_empty()
            || req.outbound_lpn.trim().is_empty()
            || req.trace_codes.is_empty()
        {
            return Err(PackingStationError::InvalidPackJob);
        }
        if req.actual_box_type != req.recommended_box_type
            && req.adjustment_reason.as_deref().is_none_or(str::is_empty)
        {
            return Err(PackingStationError::InvalidPackJob);
        }
        Ok(())
    }

    pub fn validate_weight(&self, req: &WeighPackJobRequest) -> Result<(), PackingStationError> {
        if req.actual_weight_grams <= 0
            || req.theoretical_weight_grams <= 0
            || req.tolerance_percent < 0
        {
            return Err(PackingStationError::InvalidPackJob);
        }
        let delta = (req.actual_weight_grams - req.theoretical_weight_grams).abs() * 100;
        let max_delta = req.theoretical_weight_grams * i64::from(req.tolerance_percent);
        if delta > max_delta && req.override_reason.as_deref().is_none_or(str::is_empty) {
            return Err(PackingStationError::WeightOutOfTolerance);
        }
        Ok(())
    }

    pub fn validate_waybill(&self, req: &PrintWaybillRequest) -> Result<(), PackingStationError> {
        if req.carrier_code.trim().is_empty() {
            return Err(PackingStationError::InvalidWaybill);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;
    use wms_domain::{CreatePackJobRequest, PrintWaybillRequest, WeighPackJobRequest};

    use super::{PackingStationError, PackingStationService};

    fn valid_pack_job_request() -> CreatePackJobRequest {
        CreatePackJobRequest {
            outbound_order_id: Uuid::new_v4(),
            station_id: Some(Uuid::new_v4()),
            job_no: "PK-001".to_string(),
            pack_mode: "station".to_string(),
            recommended_box_type: "M".to_string(),
            actual_box_type: "M".to_string(),
            adjustment_reason: None,
            outbound_lpn: "LPN-001".to_string(),
            trace_codes: vec!["TC-001".to_string()],
        }
    }

    #[test]
    fn pack_job_rejects_nil_outbound_order_id() {
        let service = PackingStationService;
        let mut req = valid_pack_job_request();
        req.outbound_order_id = Uuid::nil();

        assert_eq!(
            service.validate_pack_job(&req),
            Err(PackingStationError::InvalidPackJob)
        );
    }

    #[test]
    fn pack_job_rejects_nil_station_id() {
        let service = PackingStationService;
        let mut req = valid_pack_job_request();
        req.station_id = Some(Uuid::nil());

        assert_eq!(
            service.validate_pack_job(&req),
            Err(PackingStationError::InvalidPackJob)
        );
    }

    #[test]
    fn valid_pack_job_passes_uuid_validation() {
        let service = PackingStationService;

        assert_eq!(service.validate_pack_job(&valid_pack_job_request()), Ok(()));
    }

    #[test]
    fn pack_job_requires_adjustment_reason_when_box_changes() {
        let service = PackingStationService;
        let req = CreatePackJobRequest {
            outbound_order_id: Uuid::new_v4(),
            station_id: None,
            job_no: "PK-001".to_string(),
            pack_mode: "station".to_string(),
            recommended_box_type: "M".to_string(),
            actual_box_type: "L".to_string(),
            adjustment_reason: None,
            outbound_lpn: "LPN-001".to_string(),
            trace_codes: vec!["TC-001".to_string()],
        };

        assert_eq!(
            service.validate_pack_job(&req),
            Err(PackingStationError::InvalidPackJob)
        );
    }

    #[test]
    fn overweight_requires_override_reason() {
        let service = PackingStationService;
        let req = WeighPackJobRequest {
            actual_weight_grams: 1200,
            theoretical_weight_grams: 1000,
            tolerance_percent: 5,
            override_reason: None,
        };

        assert_eq!(
            service.validate_weight(&req),
            Err(PackingStationError::WeightOutOfTolerance)
        );
    }

    #[test]
    fn waybill_requires_carrier() {
        let service = PackingStationService;
        assert_eq!(
            service.validate_waybill(&PrintWaybillRequest {
                carrier_code: "".to_string(),
                waybill_no: None,
            }),
            Err(PackingStationError::InvalidWaybill)
        );
    }
}
