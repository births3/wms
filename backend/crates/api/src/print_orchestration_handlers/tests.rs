use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Extension, Router,
};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

use super::*;

fn context(permissions: &[&str]) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id: Uuid::new_v4(),
        actor_name: "h9-pdf-handler-test".to_string(),
        permissions: permissions.iter().map(|value| value.to_string()).collect(),
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn app(ctx: AuthContext) -> Router {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1:9/unused")
        .expect("lazy pool should build");
    let h_file = FileAttachmentService::with_memory(pool.clone());
    print_orchestration_router(PrintOrchestrationAppState::with_file_attachment_for_tests(
        pool, h_file,
    ))
    .layer(Extension(ctx))
}

#[tokio::test]
async fn category_pdf_read_does_not_inherit_general_orchestration_read() {
    let instance_id = Uuid::new_v4();
    let response = app(context(&[READ_PERMISSION]))
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/api/v1/print-orchestration/suite-instances/{instance_id}/category-pdfs"
                ))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn category_pdf_actions_require_separate_prepare_download_and_emergency_permissions() {
    let instance_id = Uuid::new_v4();
    for (path, permission) in [
        ("download", PDF_DOWNLOAD_PERMISSION),
        ("emergency-print", PDF_EMERGENCY_PERMISSION),
    ] {
        let response = app(context(&[PDF_READ_PERMISSION]))
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/api/v1/print-orchestration/suite-instances/{instance_id}/category-pdfs/{path}"
                    ))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"category_pdf_ids":[]}"#))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "{permission} must stay independent"
        );
    }

    let response = app(context(&[PDF_PREPARE_PERMISSION]))
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/print-orchestration/suite-instances/{instance_id}/category-pdfs/prepare"
                ))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "prepare permission reaches the independent idempotency guard"
    );
}
