use crate::{
    audit, auth::PortalAuth, models::DownloadUrlResponse, query::authorize_report,
    resolve_storage_key, PortalError, PortalState,
};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderValue, Response},
    Json,
};
use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

const DOWNLOAD_TTL_MINUTES: i64 = 15;

pub async fn create_report_download(
    State(state): State<PortalState>,
    auth: PortalAuth,
    Path(report_version_id): Path<Uuid>,
) -> Result<Json<DownloadUrlResponse>, PortalError> {
    let (storage_key, file_name) = authorize_report(&state, &auth, report_version_id).await?;
    let response = create_download_session(
        &state,
        &auth,
        "report",
        report_version_id,
        storage_key,
        file_name,
    )
    .await?;
    audit(
        &state.pool,
        Some(auth.user_id),
        Some(auth.customer_id),
        "download_authorized",
        "report",
        &report_version_id.to_string(),
        serde_json::json!({ "expires_in_minutes": DOWNLOAD_TTL_MINUTES }),
    )
    .await?;
    Ok(Json(response))
}

pub async fn create_export_download(
    State(state): State<PortalState>,
    auth: PortalAuth,
    Path(export_id): Path<Uuid>,
) -> Result<Json<DownloadUrlResponse>, PortalError> {
    let row = sqlx::query(
        "SELECT result_storage_key, result_file_name
         FROM portal_export_jobs
         WHERE id = $1
           AND customer_id = $2
           AND created_by = $3
           AND status = 'completed'
           AND expires_at > now()",
    )
    .bind(export_id)
    .bind(auth.customer_id)
    .bind(auth.user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(PortalError::NotFound)?;
    let response = create_download_session(
        &state,
        &auth,
        "export",
        export_id,
        row.try_get("result_storage_key")?,
        row.try_get("result_file_name")?,
    )
    .await?;
    audit(
        &state.pool,
        Some(auth.user_id),
        Some(auth.customer_id),
        "download_authorized",
        "export",
        &export_id.to_string(),
        serde_json::json!({ "expires_in_minutes": DOWNLOAD_TTL_MINUTES }),
    )
    .await?;
    Ok(Json(response))
}

async fn create_download_session(
    state: &PortalState,
    auth: &PortalAuth,
    resource_type: &str,
    resource_id: Uuid,
    storage_key: String,
    file_name: String,
) -> Result<DownloadUrlResponse, PortalError> {
    resolve_storage_key(&state.storage_root, &storage_key)?;
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let token_hash = hash_token(&token);
    let expires_at = Utc::now() + Duration::minutes(DOWNLOAD_TTL_MINUTES);
    sqlx::query(
        "INSERT INTO portal_download_sessions (
            id, user_id, resource_type, resource_id, storage_key, file_name,
            token_hash, expires_at
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(Uuid::new_v4())
    .bind(auth.user_id)
    .bind(resource_type)
    .bind(resource_id)
    .bind(storage_key)
    .bind(&file_name)
    .bind(token_hash)
    .bind(expires_at)
    .execute(&state.pool)
    .await?;
    Ok(DownloadUrlResponse {
        url: format!("/api/v1/files/{token}"),
        file_name,
        expires_at,
    })
}

pub async fn serve_download(
    State(state): State<PortalState>,
    Path(token): Path<String>,
) -> Result<Response<Body>, PortalError> {
    let row = sqlx::query(
        "SELECT s.id, s.user_id, s.resource_type, s.resource_id,
                s.storage_key, s.file_name, u.customer_id
         FROM portal_download_sessions s
         JOIN portal_users u ON u.id = s.user_id
         WHERE s.token_hash = $1
           AND s.expires_at > now()
           AND u.status = 'active'",
    )
    .bind(hash_token(&token))
    .fetch_optional(&state.pool)
    .await?
    .ok_or(PortalError::NotFound)?;
    let storage_key: String = row.try_get("storage_key")?;
    let path = resolve_storage_key(&state.storage_root, &storage_key)?;
    // 导出 ZIP 最大 2GB，必须流式回传，禁止整读进内存。
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|error| PortalError::Internal(format!("下载文件读取失败：{error}")))?;
    let content_length = file
        .metadata()
        .await
        .map_err(|error| PortalError::Internal(format!("下载文件读取失败：{error}")))?
        .len();
    let file_name: String = row.try_get("file_name")?;
    let resource_type: String = row.try_get("resource_type")?;
    let resource_id: Uuid = row.try_get("resource_id")?;
    let user_id: Uuid = row.try_get("user_id")?;
    let customer_id: Uuid = row.try_get("customer_id")?;
    audit(
        &state.pool,
        Some(user_id),
        Some(customer_id),
        if resource_type == "export" {
            "zip_download"
        } else {
            "report_download"
        },
        &resource_type,
        &resource_id.to_string(),
        serde_json::json!({}),
    )
    .await?;
    let mut response = Response::new(Body::from_stream(tokio_util::io::ReaderStream::new(file)));
    response
        .headers_mut()
        .insert(header::CONTENT_LENGTH, HeaderValue::from(content_length));
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(if resource_type == "export" {
            "application/zip"
        } else {
            "application/pdf"
        }),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!(
            "attachment; filename=\"{}\"; filename*=UTF-8''{}",
            if resource_type == "export" {
                "export.zip"
            } else {
                "report.pdf"
            },
            encode_header_file_name(&file_name)
        ))
        .map_err(|error| PortalError::Internal(error.to_string()))?,
    );
    Ok(response)
}

fn encode_header_file_name(file_name: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(file_name.len());
    for byte in file_name.bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'!' | b'#' | b'$' | b'&' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
            )
        {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push(HEX[(byte >> 4) as usize] as char);
            encoded.push(HEX[(byte & 0x0f) as usize] as char);
        }
    }
    encoded
}

fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}
