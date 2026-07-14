//! Wave 5 M8 retail chain business rules.

use wms_domain::{CreateCrossdockPlanRequest, CreateRetailReplenishmentSuggestionRequest};

#[derive(Clone, Debug, Default)]
pub struct RetailChainService;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RetailChainError {
    InvalidWatermark,
    InvalidCrossdockQty,
}

impl RetailChainService {
    pub fn suggested_qty(
        &self,
        req: &CreateRetailReplenishmentSuggestionRequest,
    ) -> Result<i64, RetailChainError> {
        if req.min_qty < 0
            || req.max_qty <= 0
            || req.min_qty > req.max_qty
            || req.current_qty < 0
            || req.in_transit_qty < 0
            || req.daily_sales_avg < 0
        {
            return Err(RetailChainError::InvalidWatermark);
        }
        let available = req.current_qty + req.in_transit_qty;
        Ok((req.max_qty - available).max(0))
    }

    pub fn validate_crossdock(
        &self,
        req: &CreateCrossdockPlanRequest,
    ) -> Result<(), RetailChainError> {
        if req.asn_id.is_nil()
            || req.outbound_order_id.is_nil()
            || req.store_id.is_nil()
            || req.qty <= 0
            || req.product_code.trim().is_empty()
        {
            return Err(RetailChainError::InvalidCrossdockQty);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;
    use wms_domain::{CreateCrossdockPlanRequest, CreateRetailReplenishmentSuggestionRequest};

    use super::{RetailChainError, RetailChainService};

    #[test]
    fn replenishment_suggestion_uses_max_minus_available() {
        let service = RetailChainService;
        let qty = service
            .suggested_qty(&CreateRetailReplenishmentSuggestionRequest {
                store_id: Uuid::new_v4(),
                product_code: "P-001".to_string(),
                period_key: "2026-W23".to_string(),
                min_qty: 10,
                max_qty: 50,
                current_qty: 12,
                in_transit_qty: 8,
                daily_sales_avg: 3,
            })
            .expect("valid");

        assert_eq!(qty, 30);
    }

    #[test]
    fn invalid_crossdock_qty_is_rejected() {
        let service = RetailChainService;
        assert_eq!(
            service.validate_crossdock(&crossdock_request_with(|request| request.qty = 0)),
            Err(RetailChainError::InvalidCrossdockQty)
        );
    }

    #[test]
    fn nil_asn_id_is_rejected() {
        let service = RetailChainService;
        assert_eq!(
            service.validate_crossdock(&crossdock_request_with(
                |request| request.asn_id = Uuid::nil()
            )),
            Err(RetailChainError::InvalidCrossdockQty)
        );
    }

    #[test]
    fn nil_outbound_order_id_is_rejected() {
        let service = RetailChainService;
        assert_eq!(
            service.validate_crossdock(&crossdock_request_with(|request| {
                request.outbound_order_id = Uuid::nil()
            })),
            Err(RetailChainError::InvalidCrossdockQty)
        );
    }

    #[test]
    fn nil_store_id_is_rejected() {
        let service = RetailChainService;
        assert_eq!(
            service.validate_crossdock(&crossdock_request_with(
                |request| request.store_id = Uuid::nil()
            )),
            Err(RetailChainError::InvalidCrossdockQty)
        );
    }

    #[test]
    fn valid_crossdock_request_is_accepted() {
        assert_eq!(
            RetailChainService.validate_crossdock(&crossdock_request()),
            Ok(())
        );
    }

    fn crossdock_request() -> CreateCrossdockPlanRequest {
        CreateCrossdockPlanRequest {
            asn_id: Uuid::new_v4(),
            outbound_order_id: Uuid::new_v4(),
            store_id: Uuid::new_v4(),
            product_code: "P-001".to_string(),
            qty: 1,
        }
    }

    fn crossdock_request_with(
        customize: impl FnOnce(&mut CreateCrossdockPlanRequest),
    ) -> CreateCrossdockPlanRequest {
        let mut request = crossdock_request();
        customize(&mut request);
        request
    }
}
