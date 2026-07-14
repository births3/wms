use axum::{
    extract::{Path, Query, State},
    Json,
};
use uuid::Uuid;
use wms_domain::{PutawayRecommendationQuery, PutawayRecommendationResponse};

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
