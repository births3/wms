use axum::{
    body::{to_bytes, Body},
    http::{header::CONTENT_TYPE, Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::path::PathBuf;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    file_attachment_handlers::{file_attachment_router, FileAttachmentAppState},
};

fn context(owner_id: Uuid, user_id: Uuid, permissions: &[&str]) -> AuthContext {
    AuthContext {
        user_id,
        owner_id,
        actor_name: "h-file-test".to_string(),
        permissions: permissions
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn json_request(
    method: &str,
    path: &str,
    ctx: AuthContext,
    idempotency_key: Option<&str>,
    payload: Value,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(CONTENT_TYPE, "application/json");
    if let Some(key) = idempotency_key {
        builder = builder.header("Idempotency-Key", key);
    }
    let mut request = builder
        .body(Body::from(payload.to_string()))
        .expect("JSON request should build");
    request.extensions_mut().insert(ctx);
    request
}

async fn response_json(response: axum::response::Response) -> Value {
    serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable"),
    )
    .expect("response should be JSON")
}

async fn seed_actor(pool: &PgPool, owner_id: Uuid, user_id: Uuid) {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'H-FILE 测试货主')",
    )
    .bind(owner_id)
    .bind(format!("HFILE-{}", owner_id.simple()))
    .execute(pool)
    .await
    .expect("owner should seed");
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, 'H-FILE 测试用户', 'test-hash', 'active')",
    )
    .bind(user_id)
    .bind(format!("hfile-{}", user_id.simple()))
    .execute(pool)
    .await
    .expect("user should seed");
    sqlx::query(
        "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, TRUE)",
    )
    .bind(user_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("owner binding should seed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn attachment_upload_confirm_download_is_owner_scoped_idempotent_and_audited(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    seed_actor(&pool, owner_id, user_id).await;
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '其他货主')")
        .bind(other_owner_id)
        .bind(format!("OTHER-{}", other_owner_id.simple()))
        .execute(&pool)
        .await
        .expect("other owner should seed");

    let storage_root = storage_root();
    std::fs::create_dir_all(&storage_root).expect("test storage root should create");
    let app = file_attachment_router(FileAttachmentAppState::with_local_storage(
        pool.clone(),
        storage_root.clone(),
    ));
    let write_context = context(
        owner_id,
        user_id,
        &["h-file.attachment.read", "h-file.attachment.write"],
    );
    let entity_id = Uuid::new_v4();
    let content = b"%PDF-1.4 real attachment";

    let create = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/attachments/uploads",
            write_context.clone(),
            Some("h-file-create"),
            json!({
                "module": "M-DI",
                "entity_type": "drug_inspection",
                "entity_id": entity_id,
                "file_name": "inspection.pdf",
                "content_type": "application/pdf",
                "size_bytes": content.len()
            }),
        ))
        .await
        .expect("upload session should respond");
    assert_eq!(create.status(), StatusCode::OK);
    let create_body = response_json(create).await;
    let upload_id = create_body["upload_id"]
        .as_str()
        .expect("upload id should exist");
    let upload_url = create_body["upload_url"]
        .as_str()
        .expect("upload URL should exist");

    let unauthorized_upload = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(with_invalid_token(upload_url))
                .header(CONTENT_TYPE, "application/pdf")
                .body(Body::from(content.as_slice()))
                .expect("unauthorized upload request should build"),
        )
        .await
        .expect("unauthorized upload should respond");
    assert_eq!(unauthorized_upload.status(), StatusCode::UNAUTHORIZED);

    let upload = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(upload_url)
                .header(CONTENT_TYPE, "application/pdf")
                .body(Body::from(content.as_slice()))
                .expect("upload request should build"),
        )
        .await
        .expect("upload should respond");
    assert_eq!(upload.status(), StatusCode::NO_CONTENT);

    let confirm_payload = json!({ "upload_id": upload_id });
    let confirm = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/attachments/confirm",
            write_context.clone(),
            Some("h-file-confirm"),
            confirm_payload.clone(),
        ))
        .await
        .expect("confirm should respond");
    assert_eq!(confirm.status(), StatusCode::OK);
    let attachment = response_json(confirm).await;
    let attachment_id = attachment["id"]
        .as_str()
        .expect("attachment id should exist");
    assert_eq!(attachment["size_bytes"], content.len());
    assert_eq!(attachment["content_type"], "application/pdf");

    let replay = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/attachments/confirm",
            write_context.clone(),
            Some("h-file-confirm"),
            confirm_payload.clone(),
        ))
        .await
        .expect("confirm replay should respond");
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(response_json(replay).await["id"], attachment_id);
    sqlx::query(
        "UPDATE idempotency_request SET method = 'PATCH', path = '/wrong-path' WHERE owner_id = $1 AND idempotency_key = $2",
    )
    .bind(owner_id)
    .bind("h-file-confirm")
    .execute(&pool)
    .await
    .expect("idempotency metadata should be mutable for the regression check");
    let metadata_conflict = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/attachments/confirm",
            write_context.clone(),
            Some("h-file-confirm"),
            confirm_payload,
        ))
        .await
        .expect("confirm metadata conflict should respond");
    assert_eq!(metadata_conflict.status(), StatusCode::CONFLICT);

    let url_response = app
        .clone()
        .oneshot(json_request(
            "GET",
            &format!("/api/v1/attachments/{attachment_id}/url"),
            write_context,
            None,
            json!({}),
        ))
        .await
        .expect("download URL should respond");
    assert_eq!(url_response.status(), StatusCode::OK);
    let download_url = response_json(url_response).await["url"]
        .as_str()
        .expect("download URL should exist")
        .to_string();
    let unauthorized_download = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(with_invalid_token(&download_url))
                .body(Body::empty())
                .expect("unauthorized download request should build"),
        )
        .await
        .expect("unauthorized download should respond");
    assert_eq!(unauthorized_download.status(), StatusCode::UNAUTHORIZED);

    let download = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(download_url)
                .body(Body::empty())
                .expect("download request should build"),
        )
        .await
        .expect("download should respond");
    assert_eq!(download.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(download.into_body(), usize::MAX)
            .await
            .expect("download body should read")
            .as_ref(),
        content
    );

    let forbidden = app
        .oneshot(json_request(
            "GET",
            &format!("/api/v1/attachments/{attachment_id}/url"),
            context(other_owner_id, Uuid::new_v4(), &["h-file.attachment.read"]),
            None,
            json!({}),
        ))
        .await
        .expect("cross-owner request should respond");
    assert_eq!(forbidden.status(), StatusCode::NOT_FOUND);

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND resource_type = 'attachment'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("attachment audit should query");
    assert_eq!(audit_count, 2);

    std::fs::remove_dir_all(storage_root).expect("test storage root should clean");
}

fn storage_root() -> PathBuf {
    std::env::temp_dir().join(format!("wms-h-file-{}", Uuid::new_v4()))
}

fn with_invalid_token(url: &str) -> String {
    let (prefix, _) = url
        .rsplit_once("token=")
        .expect("temporary URL should contain token");
    format!("{prefix}token=invalid")
}
