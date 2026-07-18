use super::*;
use chrono::{TimeZone, Utc};
use uuid::Uuid;
use wms_domain::ReceivingOrderLine;

use crate::auth::AuthContext;

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "print-data-test".to_string(),
        permissions: vec!["m2.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn line() -> ReceivingOrderLine {
    ReceivingOrderLine {
        line_no: 1,
        product_id: None,
        product_code: "P-001".to_string(),
        expected_qty: 10,
        batch_no: None,
        production_date: None,
        expiry_date: None,
    }
}

#[test]
fn receiving_inspection_rejects_expired_batch() {
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 10, 0, 0)
        .single()
        .expect("valid time");
    let ctx = ctx(Uuid::new_v4());
    let mut store = ReceivingOrderStore::default();
    let created = store
        .create(
            &ctx,
            CreateReceivingOrderRequest {
                receipt_no: "ASN-W3-002".to_string(),
                document_type: "purchase_inbound".to_string(),
                supplier_id: Some(Uuid::new_v4()),
                warehouse_id: Uuid::new_v4(),
                external_ref: None,
                expected_arrival_at: Some(now + chrono::Duration::days(1)),
                lines: vec![line()],
            },
            now,
        )
        .expect("create order");
    store.release(&ctx, created.id, now).expect("release order");
    store
        .receive(
            &ctx,
            created.id,
            ReceiveReceivingOrderRequest {
                actual_qty: 10,
                shortage_qty: 0,
                rejected_qty: 0,
                arrival_temperature_celsius: None,
                exception_note: None,
                details: None,
            },
            now,
        )
        .expect("receive");

    let result = store.inspect(
        &ctx,
        created.id,
        InspectReceivingOrderRequest {
            batch_no: "B-EXPIRED".to_string(),
            accepted_qty: 1,
            rejected_qty: 0,
            production_date: "2025-01-01".to_string(),
            expiry_date: "2026-01-01".to_string(),
            quality_status: "qualified".to_string(),
            trace_codes: vec![],
        },
        chrono::NaiveDate::from_ymd_opt(2026, 6, 4).expect("valid date"),
        now,
    );

    assert!(matches!(result, Err(ReceivingOrderError::BatchExpired)));
}
