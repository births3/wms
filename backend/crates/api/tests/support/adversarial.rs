//! 对抗测试共用货主与权限夹具。
//!
//! 集成测试按文件引入：`#[path = "support/adversarial.rs"] mod adversarial_support;`

use sqlx::PgPool;
use uuid::Uuid;
use wms_api::auth::AuthContext;

pub struct AdversarialOwnerPair {
    pub owner_a: Uuid,
    pub owner_b: Uuid,
}

pub fn ctx_with_permissions(owner_id: Uuid, actor: &str, permissions: &[&str]) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: actor.to_string(),
        permissions: permissions.iter().map(ToString::to_string).collect(),
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

pub async fn seed_owner(pool: &PgPool, owner_id: Uuid, label: &str) {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(format!("ADV-{}", &owner_id.to_string()[..8]))
    .bind(label)
    .execute(pool)
    .await
    .expect("adversarial owner should seed");
}

pub async fn seed_owner_pair(pool: &PgPool) -> AdversarialOwnerPair {
    let pair = AdversarialOwnerPair {
        owner_a: Uuid::new_v4(),
        owner_b: Uuid::new_v4(),
    };
    seed_owner(pool, pair.owner_a, "对抗测试货主A").await;
    seed_owner(pool, pair.owner_b, "对抗测试货主B").await;
    pair
}
