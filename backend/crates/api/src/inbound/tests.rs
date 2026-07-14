use chrono::{TimeZone, Utc};
use uuid::Uuid;
use wms_domain::{
    CreateReceivingOrderRequest, InspectReceivingOrderRequest, PutawayRequest,
    ReceiveReceivingOrderRequest, ReceivingOrderLine, RejectReceivingOrderRequest,
    UpdateReceivingOrderRequest,
};

use super::{ReceivingOrderError, ReceivingOrderStore};
use crate::auth::AuthContext;

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "tester".to_string(),
        permissions: vec!["m2.write".to_string()],
        jti: Uuid::new_v4().to_string(),
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
fn receiving_order_crud_is_owner_scoped() {
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 10, 0, 0)
        .single()
        .expect("valid time");
    let ctx_a = ctx(Uuid::new_v4());
    let ctx_b = ctx(Uuid::new_v4());
    let mut store = ReceivingOrderStore::default();

    let created = store
        .create(
            &ctx_a,
            CreateReceivingOrderRequest {
                receipt_no: "ASN-001".to_string(),
                document_type: "purchase_inbound".to_string(),
                supplier_id: Some(Uuid::new_v4()),
                warehouse_id: Uuid::new_v4(),
                external_ref: Some("ERP-ASN-001".to_string()),
                expected_arrival_at: Some(now + chrono::Duration::days(1)),
                lines: vec![line()],
            },
            now,
        )
        .expect("create receiving order");

    assert_eq!(store.list(&ctx_a).len(), 1);
    assert!(matches!(
        store.get(&ctx_b, created.id),
        Err(ReceivingOrderError::NotFound)
    ));

    store
        .delete(&ctx_a, created.id)
        .expect("delete draft receiving order");
    assert!(store.list(&ctx_a).is_empty());
}

#[test]
fn receiving_order_requires_lines() {
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 10, 0, 0)
        .single()
        .expect("valid time");
    let ctx = ctx(Uuid::new_v4());
    let mut store = ReceivingOrderStore::default();

    let result = store.create(
        &ctx,
        CreateReceivingOrderRequest {
            receipt_no: "ASN-EMPTY".to_string(),
            document_type: "purchase_inbound".to_string(),
            supplier_id: Some(Uuid::new_v4()),
            warehouse_id: Uuid::new_v4(),
            external_ref: None,
            expected_arrival_at: None,
            lines: vec![],
        },
        now,
    );

    assert!(matches!(result, Err(ReceivingOrderError::EmptyLines)));
}

#[test]
fn receiving_order_update_validation_is_atomic() {
    let now = Utc::now();
    let ctx = ctx(Uuid::new_v4());
    let mut store = ReceivingOrderStore::default();
    let created = store
        .create(
            &ctx,
            CreateReceivingOrderRequest {
                receipt_no: "ASN-ATOMIC-001".to_string(),
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

    let result = store.update(
        &ctx,
        created.id,
        UpdateReceivingOrderRequest {
            supplier_id: None,
            warehouse_id: Some(Uuid::new_v4()),
            external_ref: Some(Some("MUST-NOT-PERSIST".to_string())),
            expected_arrival_at: None,
            lines: Some(Vec::new()),
        },
        now,
    );

    assert!(matches!(result, Err(ReceivingOrderError::EmptyLines)));
    assert_eq!(
        store.get(&ctx, created.id).expect("order").external_ref,
        None
    );
}

#[test]
fn receiving_order_update_can_clear_external_reference() {
    let now = Utc::now();
    let ctx = ctx(Uuid::new_v4());
    let mut store = ReceivingOrderStore::default();
    let created = store
        .create(
            &ctx,
            CreateReceivingOrderRequest {
                receipt_no: "ASN-CLEAR-001".to_string(),
                document_type: "purchase_inbound".to_string(),
                supplier_id: Some(Uuid::new_v4()),
                warehouse_id: Uuid::new_v4(),
                external_ref: Some("ERP-CLEAR-001".to_string()),
                expected_arrival_at: Some(now),
                lines: vec![line()],
            },
            now,
        )
        .expect("create order");

    let updated = store
        .update(
            &ctx,
            created.id,
            UpdateReceivingOrderRequest {
                supplier_id: None,
                warehouse_id: None,
                external_ref: Some(None),
                expected_arrival_at: None,
                lines: None,
            },
            now,
        )
        .expect("clear nullable fields");

    assert_eq!(updated.supplier_id, created.supplier_id);
    assert_eq!(updated.external_ref, None);
    assert_eq!(updated.expected_arrival_at, created.expected_arrival_at);
}

#[test]
fn receiving_order_rejects_invalid_document_type() {
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 10, 0, 0)
        .single()
        .expect("valid time");
    let ctx = ctx(Uuid::new_v4());
    let mut store = ReceivingOrderStore::default();

    let result = store.create(
        &ctx,
        CreateReceivingOrderRequest {
            receipt_no: "ASN-BAD-TYPE".to_string(),
            document_type: "purchase_return".to_string(),
            supplier_id: Some(Uuid::new_v4()),
            warehouse_id: Uuid::new_v4(),
            external_ref: None,
            expected_arrival_at: None,
            lines: vec![line()],
        },
        now,
    );

    assert!(matches!(
        result,
        Err(ReceivingOrderError::InvalidDocumentType)
    ));
}

#[test]
fn receiving_workflow_enforces_quantity_closure_and_dual_signature() {
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
                receipt_no: "ASN-W3-001".to_string(),
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

    let mismatch = store.receive(
        &ctx,
        created.id,
        ReceiveReceivingOrderRequest {
            actual_qty: 8,
            shortage_qty: 1,
            rejected_qty: 0,
            arrival_temperature_celsius: None,
            exception_note: None,
            details: None,
        },
        now,
    );
    assert!(matches!(
        mismatch,
        Err(ReceivingOrderError::QuantityClosureMismatch)
    ));

    let receipt = store
        .receive(
            &ctx,
            created.id,
            ReceiveReceivingOrderRequest {
                actual_qty: 8,
                shortage_qty: 2,
                rejected_qty: 0,
                arrival_temperature_celsius: None,
                exception_note: None,
                details: None,
            },
            now,
        )
        .expect("closed receipt");
    assert_eq!(receipt.actual_qty, 8);

    store
        .inspect(
            &ctx,
            created.id,
            InspectReceivingOrderRequest {
                batch_no: "B202606".to_string(),
                accepted_qty: 8,
                rejected_qty: 0,
                production_date: "2026-01-01".to_string(),
                expiry_date: "2028-01-01".to_string(),
                quality_status: "qualified".to_string(),
                trace_codes: vec![],
            },
            chrono::NaiveDate::from_ymd_opt(2026, 6, 4).expect("valid date"),
            now,
        )
        .expect("inspect");

    let unauthorized_signer = store.sign_inspection(
        &ctx,
        created.id,
        wms_domain::SignInspectionRequest {
            first_signer_id: Uuid::new_v4(),
            second_signer_id: Some(Uuid::new_v4()),
            dual_required: true,
        },
        now,
    );
    assert!(matches!(
        unauthorized_signer,
        Err(ReceivingOrderError::UnauthorizedSigner)
    ));

    let same_signer = store.sign_inspection(
        &ctx,
        created.id,
        wms_domain::SignInspectionRequest {
            first_signer_id: ctx.user_id,
            second_signer_id: Some(ctx.user_id),
            dual_required: true,
        },
        now,
    );
    assert!(matches!(same_signer, Err(ReceivingOrderError::SameSigner)));

    let signature = store
        .sign_inspection(
            &ctx,
            created.id,
            wms_domain::SignInspectionRequest {
                first_signer_id: ctx.user_id,
                second_signer_id: Some(Uuid::new_v4()),
                dual_required: true,
            },
            now,
        )
        .expect("sign");
    assert_eq!(signature.owner_id, ctx.owner_id);

    let putaway = store
        .putaway(
            &ctx,
            created.id,
            PutawayRequest {
                batch_no: "B202606".to_string(),
                product_code: "P-001".to_string(),
                qty: 8,
                location_id: Uuid::new_v4(),
                location_code: "A-01-01".to_string(),
                quality_status: "qualified".to_string(),
            },
            now,
        )
        .expect("putaway");
    assert_eq!(putaway.qty, 8);
    assert_eq!(
        store.get(&ctx, created.id).expect("get").status,
        "completed"
    );
}

#[test]
fn receiving_inspection_cannot_exceed_actual_receipt_or_sign_early() {
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
                receipt_no: "ASN-W3-INSPECTION-GATE".to_string(),
                document_type: "purchase_inbound".to_string(),
                supplier_id: Some(Uuid::new_v4()),
                warehouse_id: Uuid::new_v4(),
                external_ref: None,
                expected_arrival_at: Some(now),
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
                actual_qty: 8,
                shortage_qty: 2,
                rejected_qty: 0,
                arrival_temperature_celsius: None,
                exception_note: None,
                details: None,
            },
            now,
        )
        .expect("receive order");

    let over_actual = store.inspect(
        &ctx,
        created.id,
        InspectReceivingOrderRequest {
            batch_no: "B-GATE".to_string(),
            accepted_qty: 9,
            rejected_qty: 0,
            production_date: "2026-01-01".to_string(),
            expiry_date: "2028-01-01".to_string(),
            quality_status: "qualified".to_string(),
            trace_codes: vec![],
        },
        chrono::NaiveDate::from_ymd_opt(2026, 6, 4).expect("valid date"),
        now,
    );
    assert!(matches!(
        over_actual,
        Err(ReceivingOrderError::QuantityClosureMismatch)
    ));

    store
        .inspect(
            &ctx,
            created.id,
            InspectReceivingOrderRequest {
                batch_no: "B-GATE".to_string(),
                accepted_qty: 4,
                rejected_qty: 0,
                production_date: "2026-01-01".to_string(),
                expiry_date: "2028-01-01".to_string(),
                quality_status: "qualified".to_string(),
                trace_codes: vec![],
            },
            chrono::NaiveDate::from_ymd_opt(2026, 6, 4).expect("valid date"),
            now,
        )
        .expect("partial inspection");
    let premature_signature = store.sign_inspection(
        &ctx,
        created.id,
        wms_domain::SignInspectionRequest {
            first_signer_id: ctx.user_id,
            second_signer_id: Some(Uuid::new_v4()),
            dual_required: true,
        },
        now,
    );
    assert!(matches!(
        premature_signature,
        Err(ReceivingOrderError::QuantityClosureMismatch)
    ));
}

#[test]
fn receiving_order_reject_accepts_receiving_status_and_closes_order() {
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
                receipt_no: "ASN-W3-REJECT".to_string(),
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

    let receipt = store
        .reject(
            &ctx,
            created.id,
            RejectReceivingOrderRequest {
                reason: "外包装严重破损".to_string(),
            },
            now,
        )
        .expect("reject receiving order");

    assert_eq!(receipt.rejected_qty, 10);
    assert_eq!(
        store.get(&ctx, created.id).expect("get").status,
        "closed_rejected"
    );
}
