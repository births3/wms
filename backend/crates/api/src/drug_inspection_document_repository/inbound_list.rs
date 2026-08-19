use chrono::{NaiveDate, Utc};
use sqlx::FromRow;
use uuid::Uuid;
use wms_domain::InboundDocumentEntry;

use super::{
    map_db_error, DrugInspectionDocumentRepositoryError, PgDrugInspectionDocumentRepository,
};

#[derive(Clone, Debug, Default)]
pub struct InboundDocumentQuery {
    pub received_from: Option<NaiveDate>,
    pub received_to: Option<NaiveDate>,
    pub missing_drug_inspection: bool,
    pub missing_upstream_delivery: bool,
}

impl PgDrugInspectionDocumentRepository {
    pub async fn list_inbound_documents(
        &self,
        owner_id: Uuid,
        query: InboundDocumentQuery,
        page: u32,
        page_size: u32,
    ) -> Result<(Vec<InboundDocumentEntry>, i64), DrugInspectionDocumentRepositoryError> {
        let page = page.max(1);
        let page_size = page_size.clamp(1, 200);
        let offset = ((page - 1) as i64) * (page_size as i64);
        // ASN × SKU 行放大列表：count 与分页查询共用同一 CTE 与过滤条件，
        // 保证 total 与过滤结果一致。
        let source_cte = r#"
WITH source AS (
    SELECT receiving.id AS asn_id,
           receiving.receipt_no,
           COALESCE(receiving.external_ref, 'PO-' || receiving.receipt_no) AS purchase_order_no,
           receiving.owner_id,
           receiving.supplier_id,
           supplier.supplier_name,
           (array_agg(DISTINCT line.product_id))[1] AS product_id,
           MIN(line.product_code) AS product_code,
           MIN(product.product_name) AS product_name,
           COALESCE(
               array_agg(DISTINCT line.batch_no ORDER BY line.batch_no)
                   FILTER (WHERE line.batch_no IS NOT NULL AND btrim(line.batch_no) <> ''),
               ARRAY[]::TEXT[]
           ) AS batch_nos,
           MAX(receipt.occurred_at) AS actual_received_at,
           receiving.created_at,
           COUNT(DISTINCT line.batch_no)
               FILTER (WHERE line.batch_no IS NOT NULL AND btrim(line.batch_no) <> '')::INT
               AS batch_count,
           COUNT(DISTINCT link.batch_no)
               FILTER (WHERE version.status = 'confirmed')::INT AS linked_confirmed_count,
           COALESCE(MAX(version.version_number), 0)::INT AS drug_inspection_version,
           COALESCE(MAX(upstream_version.version_number), 0)::INT AS upstream_version
           ,(array_agg(DISTINCT upstream_version.document_id)
               FILTER (WHERE upstream_version.document_id IS NOT NULL))[1]
               AS upstream_document_id
      FROM receiving_orders AS receiving
      JOIN receiving_order_lines AS line
        ON line.receiving_order_id = receiving.id
       AND line.owner_id = receiving.owner_id
      JOIN suppliers AS supplier
        ON supplier.id = receiving.supplier_id
       AND supplier.owner_id = receiving.owner_id
      JOIN products AS product
        ON product.id = line.product_id
       AND product.owner_id = receiving.owner_id
 LEFT JOIN receiving_order_receipts AS receipt
        ON receipt.receiving_order_id = receiving.id
       AND receipt.owner_id = receiving.owner_id
 LEFT JOIN drug_inspection_asn_links AS link
        ON link.owner_id = receiving.owner_id
       AND link.asn_id = receiving.id
       AND link.batch_no = line.batch_no
 LEFT JOIN drug_inspection_reports AS report
        ON report.id = link.report_id
       AND report.owner_id = receiving.owner_id
 LEFT JOIN drug_inspection_report_versions AS version
        ON version.id = report.current_version_id
       AND version.owner_id = receiving.owner_id
 LEFT JOIN upstream_delivery_asn_current AS upstream
        ON upstream.owner_id = receiving.owner_id
       AND upstream.asn_id = receiving.id
 LEFT JOIN upstream_delivery_document_versions AS upstream_version
        ON upstream_version.id = upstream.version_id
       AND upstream_version.owner_id = receiving.owner_id
     WHERE receiving.owner_id = $1
       AND receiving.document_type = 'purchase_inbound'
  GROUP BY receiving.id, supplier.supplier_name
)
"#;
        let filters = r#"
 WHERE ($2::DATE IS NULL OR actual_received_at::DATE >= $2)
   AND ($3::DATE IS NULL OR actual_received_at::DATE <= $3)
   AND (
        (NOT $4 AND NOT $5)
        OR ($4 AND actual_received_at IS NOT NULL AND batch_count > 0
            AND linked_confirmed_count < batch_count)
        OR ($5 AND upstream_version = 0)
   )
"#;
        let total_sql = format!(
            "{source_cte}\
SELECT count(*) FROM source {filters}"
        );
        let rows_sql = format!(
            "{source_cte}\
SELECT asn_id, receipt_no, purchase_order_no, owner_id, supplier_id,
       supplier_name, product_id, product_code, product_name, batch_nos,
       actual_received_at,
       CASE
           WHEN actual_received_at IS NULL THEN 'pending_receipt'
           WHEN batch_count = 0 THEN 'pending_batch'
           WHEN linked_confirmed_count = 0 THEN 'missing'
           WHEN linked_confirmed_count < batch_count THEN 'partial'
           ELSE 'complete'
       END AS drug_inspection_status,
       drug_inspection_version,
       CASE WHEN upstream_version > 0 THEN 'uploaded' ELSE 'missing' END
           AS upstream_delivery_status,
       upstream_version,
       upstream_document_id,
       created_at
  FROM source {filters}
 ORDER BY actual_received_at DESC NULLS LAST, receipt_no
 LIMIT $6 OFFSET $7"
        );
        let total: i64 = sqlx::query_scalar(&total_sql)
            .bind(owner_id)
            .bind(query.received_from)
            .bind(query.received_to)
            .bind(query.missing_drug_inspection)
            .bind(query.missing_upstream_delivery)
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_error)?;
        let rows = sqlx::query_as::<_, InboundDocumentRow>(&rows_sql)
            .bind(owner_id)
            .bind(query.received_from)
            .bind(query.received_to)
            .bind(query.missing_drug_inspection)
            .bind(query.missing_upstream_delivery)
            .bind(page_size as i64)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_db_error)?;
        Ok((rows.into_iter().map(Into::into).collect(), total))
    }
}

#[derive(FromRow)]
struct InboundDocumentRow {
    asn_id: Uuid,
    receipt_no: String,
    purchase_order_no: String,
    owner_id: Uuid,
    supplier_id: Uuid,
    supplier_name: String,
    product_id: Uuid,
    product_code: String,
    product_name: String,
    batch_nos: Vec<String>,
    actual_received_at: Option<chrono::DateTime<Utc>>,
    drug_inspection_status: String,
    drug_inspection_version: i32,
    upstream_delivery_status: String,
    upstream_version: i32,
    upstream_document_id: Option<Uuid>,
    created_at: chrono::DateTime<Utc>,
}

impl From<InboundDocumentRow> for InboundDocumentEntry {
    fn from(row: InboundDocumentRow) -> Self {
        Self {
            asn_id: row.asn_id,
            receipt_no: row.receipt_no,
            purchase_order_no: row.purchase_order_no,
            owner_id: row.owner_id,
            supplier_id: row.supplier_id,
            supplier_name: row.supplier_name,
            product_id: row.product_id,
            product_code: row.product_code,
            product_name: row.product_name,
            batch_nos: row.batch_nos,
            actual_received_at: row.actual_received_at,
            drug_inspection_status: row.drug_inspection_status,
            drug_inspection_version: row.drug_inspection_version,
            upstream_delivery_status: row.upstream_delivery_status,
            upstream_version: row.upstream_version,
            upstream_document_id: row.upstream_document_id,
            created_at: row.created_at,
        }
    }
}
