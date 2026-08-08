use super::*;
use chrono::{TimeZone, Utc};
use uuid::Uuid;
use wms_domain::{ReceivingOrderLine, ReceivingReceiptDetails};

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
        expected_qty: 10.into(),
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
                actual_qty: 10.into(),
                shortage_qty: wms_domain::Quantity::ZERO,
                rejected_qty: wms_domain::Quantity::ZERO,
                arrival_temperature_celsius: None,
                exception_note: None,
                details: Some(ReceivingReceiptDetails {
                    temperature_control_method: Some("普通".to_string()),
                    vehicle_no: Some("沪A00000".to_string()),
                    origin: Some("发运地".to_string()),
                    departure_at: Some(chrono::Utc::now()),
                    arrival_at: Some(chrono::Utc::now()),
                    storage_at: Some(chrono::Utc::now()),
                    transport_mode: Some("公路".to_string()),
                    carrier: Some("承运商".to_string()),
                    contact_name: Some("送货人".to_string()),
                    contact_phone: Some("13800000000".to_string()),
                    contact_id_no: Some("310101199001011234".to_string()),
                    seal_checked: Some("已核对".to_string()),
                    filing_checked: Some("已核对".to_string()),
                }),
            },
            now,
        )
        .expect("receive");

    let result = store.inspect(
        &ctx,
        created.id,
        InspectReceivingOrderRequest {
            batch_no: "B-EXPIRED".to_string(),
            accepted_qty: 1.into(),
            rejected_qty: wms_domain::Quantity::ZERO,
            production_date: "2025-01-01".to_string(),
            expiry_date: "2026-01-01".to_string(),
            quality_status: "qualified".to_string(),
            trace_codes: vec![],

            appearance_check: Some("完好".to_string()),
            package_check: Some("完好".to_string()),
            instruction_check: Some("有".to_string()),
            label_check: Some("清晰".to_string()),
            sampling_qty: Some(1.into()),
            approval_no: None,
        },
        chrono::NaiveDate::from_ymd_opt(2026, 6, 4).expect("valid date"),
        now,
    );

    assert!(matches!(result, Err(ReceivingOrderError::BatchExpired)));
}
