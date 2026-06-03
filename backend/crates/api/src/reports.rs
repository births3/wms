//! Wave 2 M6 report query skeleton.

use chrono::Utc;
use serde_json::json;
use wms_domain::{PageMeta, ReportQueryRequest, ReportQueryResponse, ReportRow};

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
}
