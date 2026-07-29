//! 公共 JWT 用户仓库授权查询。

use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AuthContext;

pub(crate) async fn load_user_warehouse_scopes(
    pool: &PgPool,
    ctx: &AuthContext,
) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT warehouse_id FROM auth_user_warehouse_scopes WHERE user_id = $1 AND owner_id = $2 ORDER BY warehouse_id",
    )
    .bind(ctx.user_id)
    .bind(ctx.owner_id)
    .fetch_all(pool)
    .await
}
