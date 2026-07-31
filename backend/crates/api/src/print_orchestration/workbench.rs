use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;
use wms_domain::{
    DeliveryNoteCandidate, DeliveryNoteCandidateListResponse, DeliveryNoteGroupListItem,
    DeliveryNoteGroupListResponse,
};

use crate::operation_context::OperationContext as AuthContext;

use super::{
    repository::{map_db_error, PgPrintOrchestrationRepository},
    PrintOrchestrationError,
};

#[derive(Debug, FromRow)]
struct DeliveryNoteCandidateRow {
    outbound_order_id: Uuid,
    wms_order_no: String,
    erp_order_no: Option<String>,
    warehouse_id: Uuid,
    warehouse_code: String,
    warehouse_name: String,
    customer_id: Uuid,
    customer_code: String,
    customer_name: String,
    delivery_address_id: Uuid,
    delivery_address: String,
    route_code: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct DeliveryNoteGroupListRow {
    id: Uuid,
    delivery_note_no: String,
    warehouse_id: Uuid,
    warehouse_code: String,
    warehouse_name: String,
    customer_id: Uuid,
    customer_code: String,
    customer_name: String,
    delivery_address_id: Uuid,
    delivery_address: String,
    route_code: String,
    cutoff_mode: String,
    cutoff_reason: Option<String>,
    cutoff_plan_id: Option<Uuid>,
    scheduled_cutoff_at: Option<DateTime<Utc>>,
    cutoff_at: DateTime<Utc>,
    order_ids: Vec<Uuid>,
    order_nos: Vec<String>,
}

impl PgPrintOrchestrationRepository {
    pub(super) async fn list_delivery_note_candidates(
        &self,
        ctx: &AuthContext,
        warehouse_id: Option<Uuid>,
    ) -> Result<DeliveryNoteCandidateListResponse, PrintOrchestrationError> {
        let rows = sqlx::query_as::<_, DeliveryNoteCandidateRow>(
            r#"
            SELECT order_row.id AS outbound_order_id,
                   order_row.wms_order_no,
                   order_row.erp_order_no,
                   snapshot.warehouse_id,
                   warehouse.warehouse_code,
                   warehouse.warehouse_name,
                   snapshot.customer_id,
                   customer.customer_code,
                   customer.customer_name,
                   snapshot.delivery_address_id,
                   concat_ws(
                       '',
                       address.province,
                       address.city,
                       address.district,
                       address.detail_address
                   ) AS delivery_address,
                   snapshot.route_code,
                   order_row.created_at
              FROM outbound_orders order_row
              JOIN h9_outbound_route_snapshots snapshot
                ON snapshot.owner_id = order_row.owner_id
               AND snapshot.outbound_order_id = order_row.id
              JOIN warehouses warehouse
                ON warehouse.owner_id = snapshot.owner_id
               AND warehouse.id = snapshot.warehouse_id
              JOIN customers customer
                ON customer.owner_id = snapshot.owner_id
               AND customer.id = snapshot.customer_id
              JOIN customer_addresses address
                ON address.owner_id = snapshot.owner_id
               AND address.id = snapshot.delivery_address_id
             WHERE order_row.owner_id = $1
               AND ($2::uuid IS NULL OR snapshot.warehouse_id = $2)
               AND order_row.status = 'confirmed'
               AND NOT EXISTS (
                    SELECT 1
                      FROM h9_delivery_note_group_orders grouped
                     WHERE grouped.owner_id = order_row.owner_id
                       AND grouped.outbound_order_id = order_row.id
               )
             ORDER BY order_row.created_at, order_row.id
             LIMIT 200
            "#,
        )
        .bind(ctx.owner_id)
        .bind(warehouse_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(DeliveryNoteCandidateListResponse {
            data: rows.into_iter().map(DeliveryNoteCandidate::from).collect(),
        })
    }

    pub(super) async fn list_delivery_note_groups(
        &self,
        ctx: &AuthContext,
        warehouse_id: Option<Uuid>,
    ) -> Result<DeliveryNoteGroupListResponse, PrintOrchestrationError> {
        let rows = sqlx::query_as::<_, DeliveryNoteGroupListRow>(
            r#"
            SELECT group_row.id,
                   group_row.delivery_note_no,
                   group_row.warehouse_id,
                   warehouse.warehouse_code,
                   warehouse.warehouse_name,
                   group_row.customer_id,
                   customer.customer_code,
                   customer.customer_name,
                   group_row.delivery_address_id,
                   concat_ws(
                       '',
                       address.province,
                       address.city,
                       address.district,
                       address.detail_address
                   ) AS delivery_address,
                   group_row.route_code,
                   group_row.cutoff_mode,
                   group_row.cutoff_reason,
                   group_row.cutoff_plan_id,
                   group_row.scheduled_cutoff_at,
                   group_row.cutoff_at,
                   array_agg(
                       order_row.id
                       ORDER BY grouped.created_at, order_row.created_at, order_row.id
                   ) AS order_ids,
                   array_agg(
                       order_row.wms_order_no
                       ORDER BY grouped.created_at, order_row.created_at, order_row.id
                   ) AS order_nos
              FROM h9_delivery_note_groups group_row
              JOIN h9_delivery_note_group_orders grouped
                ON grouped.owner_id = group_row.owner_id
               AND grouped.group_id = group_row.id
              JOIN outbound_orders order_row
                ON order_row.owner_id = grouped.owner_id
               AND order_row.id = grouped.outbound_order_id
              JOIN warehouses warehouse
                ON warehouse.owner_id = group_row.owner_id
               AND warehouse.id = group_row.warehouse_id
              JOIN customers customer
                ON customer.owner_id = group_row.owner_id
               AND customer.id = group_row.customer_id
              JOIN customer_addresses address
                ON address.owner_id = group_row.owner_id
               AND address.id = group_row.delivery_address_id
             WHERE group_row.owner_id = $1
               AND ($2::uuid IS NULL OR group_row.warehouse_id = $2)
             GROUP BY
                   group_row.id,
                   warehouse.warehouse_code,
                   warehouse.warehouse_name,
                   customer.customer_code,
                   customer.customer_name,
                   address.province,
                   address.city,
                   address.district,
                   address.detail_address
             ORDER BY group_row.cutoff_at DESC, group_row.id
             LIMIT 200
            "#,
        )
        .bind(ctx.owner_id)
        .bind(warehouse_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(DeliveryNoteGroupListResponse {
            data: rows
                .into_iter()
                .map(DeliveryNoteGroupListItem::from)
                .collect(),
        })
    }
}

impl From<DeliveryNoteCandidateRow> for DeliveryNoteCandidate {
    fn from(row: DeliveryNoteCandidateRow) -> Self {
        Self {
            outbound_order_id: row.outbound_order_id,
            wms_order_no: row.wms_order_no,
            erp_order_no: row.erp_order_no,
            warehouse_id: row.warehouse_id,
            warehouse_code: row.warehouse_code,
            warehouse_name: row.warehouse_name,
            customer_id: row.customer_id,
            customer_code: row.customer_code,
            customer_name: row.customer_name,
            delivery_address_id: row.delivery_address_id,
            delivery_address: row.delivery_address,
            route_code: row.route_code,
            created_at: row.created_at,
        }
    }
}

impl From<DeliveryNoteGroupListRow> for DeliveryNoteGroupListItem {
    fn from(row: DeliveryNoteGroupListRow) -> Self {
        Self {
            id: row.id,
            delivery_note_no: row.delivery_note_no,
            warehouse_id: row.warehouse_id,
            warehouse_code: row.warehouse_code,
            warehouse_name: row.warehouse_name,
            customer_id: row.customer_id,
            customer_code: row.customer_code,
            customer_name: row.customer_name,
            delivery_address_id: row.delivery_address_id,
            delivery_address: row.delivery_address,
            route_code: row.route_code,
            cutoff_mode: row.cutoff_mode,
            cutoff_reason: row.cutoff_reason,
            cutoff_plan_id: row.cutoff_plan_id,
            scheduled_cutoff_at: row.scheduled_cutoff_at,
            cutoff_at: row.cutoff_at,
            order_ids: row.order_ids,
            order_nos: row.order_nos,
        }
    }
}
