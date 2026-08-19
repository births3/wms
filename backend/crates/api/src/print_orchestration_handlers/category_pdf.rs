//! Category PDF HTTP actions with independent H1 permissions.

use super::*;

pub(super) async fn list_category_pdfs_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Path(instance_id): Path<Uuid>,
) -> Result<Json<CategoryPdfOutputListResponse>, PrintOrchestrationHandlerError> {
    ctx.require_permission(PDF_READ_PERMISSION)?;
    Ok(Json(
        state.service.list_category_pdfs(&ctx, instance_id).await?,
    ))
}

pub(super) async fn prepare_category_pdfs_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Path(instance_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<CategoryPdfPreparation>, PrintOrchestrationHandlerError> {
    ctx.require_permission(PDF_PREPARE_PERMISSION)?;
    let result = state
        .service
        .prepare_category_pdfs(
            &ctx,
            instance_id,
            Utc::now(),
            idempotency_key_from_headers(&headers)?,
        )
        .await?;
    Ok(Json(result.value))
}

pub(super) async fn download_category_pdfs_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Path(instance_id): Path<Uuid>,
    Json(request): Json<SelectCategoryPdfsRequest>,
) -> Result<Response, PrintOrchestrationHandlerError> {
    ctx.require_permission(PDF_DOWNLOAD_PERMISSION)?;
    let content = state
        .service
        .download_category_pdfs(
            &ctx,
            instance_id,
            &request.category_pdf_ids,
            false,
            Utc::now(),
        )
        .await?;
    Ok(pdf_response(content, false))
}

pub(super) async fn emergency_print_category_pdfs_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Path(instance_id): Path<Uuid>,
    Json(request): Json<SelectCategoryPdfsRequest>,
) -> Result<Response, PrintOrchestrationHandlerError> {
    ctx.require_permission(PDF_EMERGENCY_PERMISSION)?;
    let content = state
        .service
        .download_category_pdfs(
            &ctx,
            instance_id,
            &request.category_pdf_ids,
            true,
            Utc::now(),
        )
        .await?;
    Ok(pdf_response(content, true))
}

fn pdf_response(content: Vec<u8>, inline: bool) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/pdf"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static(if inline {
            "inline; filename=\"h9-category-pdfs.pdf\""
        } else {
            "attachment; filename=\"h9-category-pdfs.pdf\""
        }),
    );
    (StatusCode::OK, headers, content).into_response()
}
