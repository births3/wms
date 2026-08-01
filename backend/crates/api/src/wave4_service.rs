//! M4 发运 application service boundary.
//!
//! The service owns request/context/audit intent. The focused repository port
//! remains the single PostgreSQL transaction until AR-06 settles shared
//! idempotency semantics.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;
use wms_domain::{OutboundOrder, ShipOutboundOrderRequest};

use crate::{
    audit::AuditWriteRequest,
    operation_context::OperationContext,
    wave4_repository::{IdempotentMutation, ShipOutboundOrderPort, Wave4RepositoryError},
};

#[derive(Clone, Debug)]
pub struct Wave4ShippingService<R> {
    port: Arc<R>,
}

impl<R> Wave4ShippingService<R>
where
    R: ShipOutboundOrderPort,
{
    pub fn new(port: Arc<R>) -> Self {
        Self { port }
    }

    pub async fn ship_outbound_order(
        &self,
        ctx: &OperationContext,
        order_id: Uuid,
        request: ShipOutboundOrderRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<OutboundOrder>, Wave4RepositoryError> {
        crate::outbound_state_rules::validate_outbound_transition(
            "reviewed",
            "shipped",
            "handover_confirmed",
        )
        .map_err(|_| Wave4RepositoryError::InvalidStateTransition {
            from: "reviewed".to_string(),
            to: "shipped".to_string(),
            approval_source: "handover_confirmed".to_string(),
        })?;
        let audit = AuditWriteRequest::from_auth_context(
            ctx,
            "ship_outbound_order",
            "M4",
            "outbound_order",
            order_id.to_string(),
            None,
        );
        self.port
            .persist_ship_outbound_order(ctx, order_id, request, now, idempotency_key, Some(audit))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Clone, Default)]
    struct RecordingPort {
        audit: Arc<Mutex<Option<AuditWriteRequest>>>,
    }

    impl ShipOutboundOrderPort for RecordingPort {
        fn persist_ship_outbound_order<'a>(
            &'a self,
            _ctx: &'a OperationContext,
            _order_id: Uuid,
            _request: ShipOutboundOrderRequest,
            _now: DateTime<Utc>,
            _idempotency_key: &'a str,
            audit: Option<AuditWriteRequest>,
        ) -> crate::wave4_repository::ShipOutboundOrderFuture<'a> {
            *self.audit.lock().expect("audit lock") = audit;
            Box::pin(async { Err(Wave4RepositoryError::NotFound) })
        }
    }

    #[tokio::test]
    async fn wave4_shipping_service_builds_audit_and_delegates_to_port() {
        let port = RecordingPort::default();
        let audit = Arc::clone(&port.audit);
        let service = Wave4ShippingService::new(Arc::new(port));
        let ctx = OperationContext {
            user_id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            actor_name: "alice".to_string(),
            permissions: vec!["m4.write".to_string()],
            jti: "jti-1".to_string(),
            warehouse_scope: None,
        };
        let request = ShipOutboundOrderRequest {
            delivery_provider_type: "own_fleet".to_string(),
            vehicle_no: Some("沪A12345".to_string()),
            plate_no: "沪A12345".to_string(),
            driver_user_id: Some(Uuid::new_v4()),
            courier_name: None,
            courier_phone: None,
            signature_attachment_id: None,
            loading_temperature_celsius: None,
            cold_chain_packages: Vec::new(),
            package_count: 1,
        };

        let result = service
            .ship_outbound_order(&ctx, Uuid::new_v4(), request, Utc::now(), "ship-1")
            .await;

        assert!(matches!(result, Err(Wave4RepositoryError::NotFound)));
        let recorded = audit
            .lock()
            .expect("audit lock")
            .clone()
            .expect("audit request");
        assert_eq!(recorded.action, "ship_outbound_order");
        assert_eq!(recorded.module, "M4");
    }
}
