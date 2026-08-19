use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use uuid::Uuid;
use wms_domain::{
    PutawayLocationValidationRequest, PutawayLocationValidationResponse,
    PutawayRecommendationQuery, PutawayRecommendationResponse,
};

use crate::auth::AuthContext;

use super::{Wave3AppState, Wave3HandlerError};

pub(crate) async fn recommend_putaway_locations_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(receiving_order_id): Path<Uuid>,
    Query(query): Query<PutawayRecommendationQuery>,
) -> Result<Json<PutawayRecommendationResponse>, Wave3HandlerError> {
    ctx.require_permission("m2.putaway.write")?;
    let repository = state.wave3_repository.as_ref().ok_or_else(|| {
        Wave3HandlerError::Repository(crate::wave3_repository::Wave3RepositoryError::Database(
            "上架库位推荐需要 PostgreSQL repository".to_string(),
        ))
    })?;
    Ok(Json(
        repository
            .recommend_putaway_locations(&ctx, receiving_order_id, query)
            .await?,
    ))
}

pub(crate) async fn validate_putaway_location_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Json(req): Json<PutawayLocationValidationRequest>,
) -> Result<Json<PutawayLocationValidationResponse>, Wave3HandlerError> {
    ctx.require_permission("m2.putaway.write")?;
    let repository = state.wave3_repository.as_ref().ok_or_else(|| {
        Wave3HandlerError::Repository(crate::wave3_repository::Wave3RepositoryError::Database(
            "上架库位 6 维校验需要 PostgreSQL repository".to_string(),
        ))
    })?;
    let now = Utc::now();
    let res = repository.validate_putaway_location(&ctx, req, now).await?;
    Ok(Json(res))
}
