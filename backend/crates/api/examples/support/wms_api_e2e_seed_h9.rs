//! H9 business print-template seeds used by real web-admin E2E entrypoints.

use std::error::Error;

use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

struct BusinessPrintSeed {
    library_code: &'static str,
    library_name: &'static str,
    business_module: &'static str,
    source_schema: &'static str,
    template_code: &'static str,
    template_name: &'static str,
    template_type_code: &'static str,
    field_path: &'static str,
    field_name: &'static str,
    paper_type: &'static str,
}

const BUSINESS_PRINT_SEEDS: [BusinessPrintSeed; 6] = [
    BusinessPrintSeed {
        library_code: "m2_asn",
        library_name: "M2 ASN 字段库",
        business_module: "M2",
        source_schema: "ReceivingOrderPrintData",
        template_code: "m2_asn_e2e",
        template_name: "M2 ASN E2E 模板",
        template_type_code: "asn",
        field_path: "order.receipt_no",
        field_name: "ASN 号",
        paper_type: "A4",
    },
    BusinessPrintSeed {
        library_code: "m2_acceptance_record",
        library_name: "M2 验收记录字段库",
        business_module: "M2",
        source_schema: "ReceivingOrderPrintData",
        template_code: "m2_acceptance_e2e",
        template_name: "M2 验收记录 E2E 模板",
        template_type_code: "acceptance_record",
        field_path: "order.receipt_no",
        field_name: "ASN 号",
        paper_type: "A4",
    },
    BusinessPrintSeed {
        library_code: "m4_delivery_note",
        library_name: "M4 随货同行单字段库",
        business_module: "M4",
        source_schema: "OutboundOrder",
        template_code: "m4_delivery_note_e2e",
        template_name: "M4 随货同行单 E2E 模板",
        template_type_code: "delivery_note",
        field_path: "wms_order_no",
        field_name: "出库单号",
        paper_type: "A4",
    },
    BusinessPrintSeed {
        library_code: "m1_location_label",
        library_name: "M1 库位标签字段库",
        business_module: "M1",
        source_schema: "Location",
        template_code: "m1_location_label_e2e",
        template_name: "M1 库位标签 E2E 模板",
        template_type_code: "location_label",
        field_path: "location_code",
        field_name: "库位编码",
        paper_type: "CUSTOM",
    },
    BusinessPrintSeed {
        library_code: "m3_lpn_label",
        library_name: "M3 LPN 标签字段库",
        business_module: "M3",
        source_schema: "InventoryBatch",
        template_code: "m3_lpn_label_e2e",
        template_name: "M3 LPN 标签 E2E 模板",
        template_type_code: "lpn_label",
        field_path: "container_lpn",
        field_name: "LPN",
        paper_type: "CUSTOM",
    },
    BusinessPrintSeed {
        library_code: "m1_product_label",
        library_name: "M1 商品标签字段库",
        business_module: "M1",
        source_schema: "Product",
        template_code: "m1_product_label_e2e",
        template_name: "M1 商品标签 E2E 模板",
        template_type_code: "product_label",
        field_path: "product_code",
        field_name: "商品编码",
        paper_type: "CUSTOM",
    },
];

pub async fn seed_h9_business_print_templates(pool: &PgPool) -> Result<(), Box<dyn Error>> {
    let actor_id = Uuid::parse_str("00000000-0000-0000-0000-000000000101")?;
    for seed in BUSINESS_PRINT_SEEDS {
        seed_business_print_template(pool, actor_id, &seed).await?;
    }
    Ok(())
}

pub async fn seed_h9_delivery_note_aggregation(pool: &PgPool) -> Result<(), Box<dyn Error>> {
    let owner_id = Uuid::from_u128(1);
    let actor_id = Uuid::parse_str("00000000-0000-0000-0000-000000000101")?;
    let customer_id = Uuid::parse_str("00000000-0000-0000-0000-000000001201")?;
    let warehouse_id = Uuid::parse_str("00000000-0000-0000-0000-000000001301")?;
    let address_id = Uuid::parse_str("00000000-0000-0000-0000-000000001211")?;
    let order_id = Uuid::parse_str("00000000-0000-0000-0000-000000009603")?;

    sqlx::query(
        r#"
        INSERT INTO customer_addresses (
            id, owner_id, customer_id, province, city, district,
            detail_address, contact_name, contact_phone, is_default
        )
        VALUES ($1, $2, $3, '上海市', '上海市', '浦东新区', '真实数据路 006 号',
                'E2E 收货人', '13800000006', TRUE)
        ON CONFLICT (id) DO UPDATE
        SET detail_address = EXCLUDED.detail_address,
            contact_name = EXCLUDED.contact_name,
            contact_phone = EXCLUDED.contact_phone
        "#,
    )
    .bind(address_id)
    .bind(owner_id)
    .bind(customer_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO h9_route_bindings (
            id, owner_id, warehouse_id, customer_id, delivery_address_id,
            route_code, effective_from, effective_to, created_by
        )
        VALUES ('00000000-0000-0000-0000-000000009601', $1, $2, $3, $4,
                'LINE-H9-E2E-006', '2026-01-01T00:00:00Z', '2100-01-01T00:00:00Z', $5)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(customer_id)
    .bind(address_id)
    .bind(actor_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO h9_cutoff_plans (
            id, owner_id, name, warehouse_id, scope_type, customer_id,
            utc_offset_minutes, weekly_schedule, exceptions, effective_from,
            status, created_by, published_by, published_at
        )
        VALUES (
            '00000000-0000-0000-0000-000000009602', $1, 'E2E 客户截单计划', $2,
            'customer', $3, 480,
            '[{"weekday":1,"cutoff_time":"17:00"},{"weekday":2,"cutoff_time":"17:00"}]'::jsonb,
            '[{"date":"2026-08-01","cutoff_time":"12:00"}]'::jsonb,
            '2026-01-01T00:00:00Z', 'published', $4, $4, now()
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(customer_id)
    .bind(actor_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO document_number_rules (
            id, owner_id, document_type, rule_code, rule_name, template,
            reset_policy, sequence_width, enabled, effective_from
        )
        VALUES (
            '00000000-0000-0000-0000-000000009604', $1,
            'print_document_category:delivery_note', 'h9-delivery-note-e2e',
            '随货同行单号', 'SHTX-{OWNER}-{YYYY}{MM}{DD}-{SEQ}',
            'daily', 4, TRUE, '2026-01-01T00:00:00Z'
        )
        ON CONFLICT ((COALESCE(owner_id, '00000000-0000-0000-0000-000000000000'::uuid)), rule_code)
        DO NOTHING
        "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO outbound_orders (
            id, owner_id, document_type, wms_order_no, erp_order_no,
            customer_id, warehouse_id, status, created_at
        )
        VALUES ($1, $2, 'sales_outbound', 'OUT-H9-E2E-006', 'ERP-H9-E2E-006',
                $3, $4, 'confirmed', '2026-07-26T08:00:00Z')
        ON CONFLICT (id) DO UPDATE
        SET status = CASE
            WHEN EXISTS (
                SELECT 1 FROM h9_delivery_note_group_orders
                 WHERE owner_id = EXCLUDED.owner_id AND outbound_order_id = EXCLUDED.id
            ) THEN outbound_orders.status
            ELSE 'confirmed'
        END
        "#,
    )
    .bind(order_id)
    .bind(owner_id)
    .bind(customer_id)
    .bind(warehouse_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO h9_outbound_route_snapshots (
            outbound_order_id, owner_id, warehouse_id, customer_id,
            delivery_address_id, route_code, frozen_at
        )
        VALUES ($1, $2, $3, $4, $5, 'LINE-H9-E2E-006', '2026-07-26T08:00:00Z')
        ON CONFLICT (outbound_order_id) DO NOTHING
        "#,
    )
    .bind(order_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(customer_id)
    .bind(address_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn seed_business_print_template(
    pool: &PgPool,
    actor_id: Uuid,
    seed: &BusinessPrintSeed,
) -> Result<(), Box<dyn Error>> {
    sqlx::query(
        r#"
        INSERT INTO print_field_libraries (
            id, library_code, library_name, business_module, source_schema
        )
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (library_code) DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(seed.library_code)
    .bind(seed.library_name)
    .bind(seed.business_module)
    .bind(seed.source_schema)
    .execute(pool)
    .await?;

    let library_id: Uuid =
        sqlx::query_scalar("SELECT id FROM print_field_libraries WHERE library_code = $1")
            .bind(seed.library_code)
            .fetch_one(pool)
            .await?;
    sqlx::query(
        r#"
        INSERT INTO print_field_library_versions (
            id, library_id, version_no, status, source_schema, business_module,
            request_hash, created_by
        )
        VALUES ($1, $2, 1, 'draft', $3, $4, $5, $6)
        ON CONFLICT (library_id, version_no) DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(library_id)
    .bind(seed.source_schema)
    .bind(seed.business_module)
    .bind(format!("e2e-{}-fields-v1", seed.template_type_code))
    .bind(actor_id)
    .execute(pool)
    .await?;

    let (library_version_id, status): (Uuid, String) = sqlx::query_as(
        r#"
        SELECT id, status
          FROM print_field_library_versions
         WHERE library_id = $1 AND version_no = 1
        "#,
    )
    .bind(library_id)
    .fetch_one(pool)
    .await?;
    if status == "draft" {
        sqlx::query(
            r#"
            INSERT INTO print_field_definitions (
                id, library_version_id, field_path, field_type, source_schema,
                display_name, group_code, group_name, printable, sensitive, sort_order
            )
            VALUES ($1, $2, $3, 'string', $4, $5, 'business', '业务信息', TRUE, FALSE, 10)
            ON CONFLICT (library_version_id, field_path) DO NOTHING
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(library_version_id)
        .bind(seed.field_path)
        .bind(seed.source_schema)
        .bind(seed.field_name)
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            UPDATE print_field_library_versions
               SET status = 'published', published_at = now(), published_by = $2
             WHERE id = $1 AND status = 'draft'
            "#,
        )
        .bind(library_version_id)
        .bind(actor_id)
        .execute(pool)
        .await?;
    }

    sqlx::query(
        r#"
        INSERT INTO print_templates (
            id, owner_id, template_code, template_name, template_type_code, scope,
            enabled, is_default, remark, created_by, updated_by
        )
        VALUES ($1, $2, $3, $4, $5, 'global', TRUE, TRUE, '真实数据 E2E 模板', $6, $6)
        ON CONFLICT (owner_id, template_code) DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(Uuid::nil())
    .bind(seed.template_code)
    .bind(seed.template_name)
    .bind(seed.template_type_code)
    .bind(actor_id)
    .execute(pool)
    .await?;

    let template_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM print_templates WHERE owner_id = $1 AND template_code = $2",
    )
    .bind(Uuid::nil())
    .bind(seed.template_code)
    .fetch_one(pool)
    .await?;
    let label_width = if seed.paper_type == "A4" { 210 } else { 100 };
    let label_height = if seed.paper_type == "A4" { 297 } else { 60 };
    let hiprint_json = json!({
        "panels": [{
            "index": 0,
            "paperType": seed.paper_type,
            "width": label_width,
            "height": label_height,
            "printElements": [{
                "options": {
                    "field": seed.field_path,
                    "title": seed.field_name,
                    "left": 20,
                    "top": 20,
                    "width": 260,
                    "height": 20
                },
                "printElementType": { "type": "text" }
            }]
        }]
    });
    let field_bindings = json!([{ "field_path": seed.field_path, "required": true }]);
    let paper = json!({
        "paperType": seed.paper_type,
        "width": label_width,
        "height": label_height,
        "direction": "portrait"
    });
    sqlx::query(
        r#"
        INSERT INTO print_template_versions (
            id, template_id, field_library_version_id,
            template_name, template_type_code, scope, is_default, remark,
            version_no, status, hiprint_json, field_bindings, paper, designer_version,
            request_hash, created_by, published_at, published_by
        )
        VALUES (
            $1, $2, $3, $4, $5, 'global', TRUE, '真实数据 E2E 模板',
            1, 'published', $6, $7, $8, 'hiprint@0.4.0', $9, $10, now(), $10
        )
        ON CONFLICT (template_id, version_no) DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(template_id)
    .bind(library_version_id)
    .bind(seed.template_name)
    .bind(seed.template_type_code)
    .bind(hiprint_json)
    .bind(field_bindings)
    .bind(paper)
    .bind(format!("e2e-{}-template-v1", seed.template_type_code))
    .bind(actor_id)
    .execute(pool)
    .await?;
    Ok(())
}
