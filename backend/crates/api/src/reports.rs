//! Wave 2/Wave 4 M6 report query skeletons.

use chrono::Utc;
use serde_json::json;
use wms_domain::{
    GspLedgerReport, GspLedgerRow, PageMeta, ReportQueryRequest, ReportQueryResponse, ReportRow,
};

#[derive(Clone, Debug, Default)]
pub struct ReportService;

impl ReportService {
    pub fn query(&self, req: ReportQueryRequest) -> ReportQueryResponse {
        let limit = req.limit.unwrap_or(50).min(200);
        let rows = if req.report_code == "m6_inbound_summary" {
            vec![ReportRow {
                values: json!({
                    "metric": "receiving_orders",
                    "count": 0,
                    "filters": req.filters,
                }),
            }]
        } else {
            Vec::new()
        };
        ReportQueryResponse {
            report_code: req.report_code,
            generated_at: Utc::now(),
            page: PageMeta {
                next_cursor: None,
                count: rows.len().min(limit as usize) as u32,
            },
            rows,
        }
    }

    pub fn gsp_ledger(&self, ledger_type: &str, req: ReportQueryRequest) -> GspLedgerReport {
        let limit = req.limit.unwrap_or(50).min(200);
        let rows = if req.report_code == format!("gsp_{ledger_type}_ledger") {
            Vec::new()
        } else {
            vec![GspLedgerRow {
                ledger_type: ledger_type.to_string(),
                occurred_at: None,
                product_code: None,
                batch_no: None,
                quantity_delta: None,
                document_type: None,
                document_no: None,
                approval_source: None,
                approval_id: None,
                operator_id: None,
                operator_name: None,
                values: json!({
                    "warning": "unsupported_report_code",
                    "requested_report_code": req.report_code,
                    "filters": req.filters,
                }),
            }]
        };

        GspLedgerReport {
            ledger_type: ledger_type.to_string(),
            generated_at: Utc::now(),
            page: PageMeta {
                next_cursor: None,
                count: rows.len().min(limit as usize) as u32,
            },
            rows,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wms_domain::ReportQueryRequest;

    use super::ReportService;

    #[test]
    fn report_query_returns_stable_skeleton_shape() {
        let service = ReportService;
        let response = service.query(ReportQueryRequest {
            report_code: "m6_inbound_summary".to_string(),
            filters: json!({"warehouse_id": "WH-01"}),
            limit: Some(20),
        });

        assert_eq!(response.report_code, "m6_inbound_summary");
        assert_eq!(response.page.count, 1);
        assert_eq!(response.rows[0].values["metric"], "receiving_orders");
    }

    #[test]
    fn gsp_ledger_reports_have_stable_empty_shape() {
        let service = ReportService;
        for ledger_type in ["inbound", "outbound", "inventory"] {
            let response = service.gsp_ledger(
                ledger_type,
                ReportQueryRequest {
                    report_code: format!("gsp_{ledger_type}_ledger"),
                    filters: json!({"owner_id": "current"}),
                    limit: Some(20),
                },
            );

            assert_eq!(response.ledger_type, ledger_type);
            assert_eq!(response.page.count, 0);
            assert!(response.rows.is_empty());
        }
    }
}
