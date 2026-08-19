//! Route composition for H9 print orchestration handlers.

use axum::{
    routing::{get, post},
    Router,
};

use super::*;

/// Builds the H9 print orchestration routes.
pub fn print_orchestration_router(state: PrintOrchestrationAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/print-orchestration/delivery-note-candidates",
            get(list_delivery_note_candidates_handler),
        )
        .route(
            "/api/v1/print-orchestration/delivery-note-groups",
            get(list_delivery_note_groups_handler),
        )
        .route(
            "/api/v1/print-orchestration/delivery-note-groups/manual-cutoff",
            post(manual_delivery_note_cutoff_handler),
        )
        .route(
            "/api/v1/print-orchestration/route-bindings",
            get(list_route_bindings_handler).post(publish_route_binding_handler),
        )
        .route(
            "/api/v1/print-orchestration/cutoff-plans",
            get(list_cutoff_plans_handler).post(create_cutoff_plan_handler),
        )
        .route(
            "/api/v1/print-orchestration/cutoff-plans/:plan_id/publish",
            post(publish_cutoff_plan_handler),
        )
        .route(
            "/api/v1/print-orchestration/aggregation-fields",
            get(list_aggregation_fields_handler),
        )
        .route(
            "/api/v1/print-orchestration/aggregation-rules/versions",
            get(list_aggregation_rules_handler).post(create_aggregation_rule_draft_handler),
        )
        .route(
            "/api/v1/print-orchestration/aggregation-rules/versions/:version_id/test",
            post(test_aggregation_rule_handler),
        )
        .route(
            "/api/v1/print-orchestration/aggregation-rules/versions/:version_id/publish",
            post(publish_aggregation_rule_handler),
        )
        .route(
            "/api/v1/print-orchestration/aggregation-rules/versions/:version_id/disable",
            post(disable_aggregation_rule_handler),
        )
        .route(
            "/api/v1/print-orchestration/print-document-categories",
            get(list_print_document_categories_handler),
        )
        .route(
            "/api/v1/print-orchestration/print-suites/versions",
            get(list_print_suites_handler).post(create_print_suite_draft_handler),
        )
        .route(
            "/api/v1/print-orchestration/print-suites/versions/:version_id/test",
            post(test_print_suite_handler),
        )
        .route(
            "/api/v1/print-orchestration/print-suites/versions/:version_id/publish",
            post(publish_print_suite_handler),
        )
        .route(
            "/api/v1/print-orchestration/print-suites/versions/:version_id/disable",
            post(disable_print_suite_handler),
        )
        .route(
            "/api/v1/print-orchestration/suite-instances",
            get(list_print_suite_instances_handler),
        )
        .route(
            "/api/v1/print-orchestration/suite-instances/:instance_id/category-pdfs",
            get(list_category_pdfs_handler),
        )
        .route(
            "/api/v1/print-orchestration/suite-instances/:instance_id/category-pdfs/prepare",
            post(prepare_category_pdfs_handler),
        )
        .route(
            "/api/v1/print-orchestration/suite-instances/:instance_id/category-pdfs/download",
            post(download_category_pdfs_handler),
        )
        .route(
            "/api/v1/print-orchestration/suite-instances/:instance_id/category-pdfs/emergency-print",
            post(emergency_print_category_pdfs_handler),
        )
        .with_state(state)
}
