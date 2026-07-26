//! H9 print-template seed used by the real web-admin E2E entrypoint.

use std::error::Error;

use sqlx::PgPool;

pub async fn seed_h9_asn_print_template(pool: &PgPool) -> Result<(), Box<dyn Error>> {
    sqlx::query(
        r#"
        INSERT INTO print_field_libraries (
            id, library_code, library_name, business_module, source_schema
        )
        VALUES (
            '00000000-0000-0000-0000-000000002801',
            'm2_asn',
            'M2 ASN 字段库',
            'M2',
            'ReceivingOrder'
        )
        ON CONFLICT (library_code) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO print_field_library_versions (
            id, library_id, version_no, status, source_schema, business_module,
            request_hash, created_by
        )
        SELECT
            '00000000-0000-0000-0000-000000002901',
            libraries.id,
            1,
            'draft',
            'ReceivingOrder',
            'M2',
            'wms-api-e2e-m2-asn-v1',
            '00000000-0000-0000-0000-000000000101'
        FROM print_field_libraries libraries
        WHERE libraries.library_code = 'm2_asn'
        ON CONFLICT (library_id, version_no) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO print_field_definitions (
            id, library_version_id, field_path, field_type, source_schema,
            display_name, group_code, group_name, printable, sensitive, sort_order
        )
        SELECT
            seed.id,
            versions.id,
            seed.field_path,
            seed.field_type,
            seed.source_schema,
            seed.display_name,
            seed.group_code,
            seed.group_name,
            seed.printable,
            seed.sensitive,
            seed.sort_order
        FROM print_field_library_versions versions
        JOIN print_field_libraries libraries ON libraries.id = versions.library_id
        CROSS JOIN (VALUES
            (
                '00000000-0000-0000-0000-000000004801'::uuid,
                'asn.code', 'string', 'ReceivingOrder', 'ASN 号', 'order', '订单信息',
                TRUE, FALSE, 10
            ),
            (
                '00000000-0000-0000-0000-000000004802'::uuid,
                'product.code', 'string', 'ReceivingOrderLine', '商品编码', 'product', '商品信息',
                TRUE, FALSE, 20
            )
        ) AS seed(
            id, field_path, field_type, source_schema, display_name,
            group_code, group_name, printable, sensitive, sort_order
        )
        WHERE libraries.library_code = 'm2_asn'
          AND versions.version_no = 1
        ON CONFLICT (library_version_id, field_path) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        UPDATE print_field_library_versions versions
           SET status = 'published',
               published_at = now(),
               published_by = '00000000-0000-0000-0000-000000000101'
          FROM print_field_libraries libraries
         WHERE libraries.id = versions.library_id
           AND libraries.library_code = 'm2_asn'
           AND versions.version_no = 1
           AND versions.status = 'draft'
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO print_templates (
            id, owner_id, template_code, template_name, template_type_code, scope,
            enabled, is_default, remark, created_by, updated_by
        )
        VALUES (
            '00000000-0000-0000-0000-000000003801',
            '00000000-0000-0000-0000-000000000001',
            'm2_asn_e2e', 'M2 ASN E2E 模板', 'asn', 'global',
            TRUE, TRUE, '真实数据 E2E 模板',
            '00000000-0000-0000-0000-000000000101',
            '00000000-0000-0000-0000-000000000101'
        )
        ON CONFLICT (owner_id, template_code) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO print_template_versions (
            id, template_id, field_library_version_id,
            template_name, template_type_code, scope, is_default, remark,
            version_no, status,
            hiprint_json, field_bindings, paper, designer_version, request_hash,
            created_by, published_at, published_by
        )
        SELECT
            '00000000-0000-0000-0000-000000003901',
            templates.id,
            versions.id,
            templates.template_name,
            templates.template_type_code,
            templates.scope,
            templates.is_default,
            templates.remark,
            1,
            'published',
            '{"panels":[{"index":0,"paperType":"A4","width":210,"height":297,"printElements":[{"options":{"field":"asn.code","title":"ASN 号","left":20,"top":20,"width":120,"height":20},"printElementType":{"type":"text"}}]}]}'::jsonb,
            '[{"field_path":"asn.code","required":true},{"field_path":"product.code","required":false}]'::jsonb,
            '{"paperType":"A4","width":210,"height":297,"direction":"portrait"}'::jsonb,
            'hiprint@0.4.0',
            'wms-api-e2e-m2-asn-template-v1',
            '00000000-0000-0000-0000-000000000101',
            now(),
            '00000000-0000-0000-0000-000000000101'
        FROM print_templates templates
        JOIN print_field_libraries libraries ON libraries.library_code = 'm2_asn'
        JOIN print_field_library_versions versions
          ON versions.library_id = libraries.id AND versions.version_no = 1
        WHERE templates.owner_id = '00000000-0000-0000-0000-000000000001'
          AND templates.template_code = 'm2_asn_e2e'
        ON CONFLICT (template_id, version_no) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}
