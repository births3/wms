use sqlx::PgPool;
use uuid::Uuid;

pub async fn seed_receiving_verifiers(pool: &PgPool, owner_id: Uuid, user_ids: &[Uuid]) {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '收货验收测试货主') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(format!("M2-VERIFIER-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed verifier owner");

    sqlx::query(
        "INSERT INTO auth_permissions (id, permission_code, permission_name) VALUES ($1, 'm2.write', '收货写入') ON CONFLICT DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .execute(pool)
    .await
    .expect("seed verifier permission");

    sqlx::query(
        "INSERT INTO auth_roles (id, owner_id, role_code, role_name) VALUES ($1, $2, 'receiving_clerk', '收货员（验收岗）') ON CONFLICT DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("seed verifier role");

    let role_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM auth_roles WHERE owner_id = $1 AND role_code = 'receiving_clerk'",
    )
    .bind(owner_id)
    .fetch_one(pool)
    .await
    .expect("find verifier role");
    sqlx::query(
        "INSERT INTO auth_role_permissions (role_id, permission_id) SELECT $1, id FROM auth_permissions WHERE permission_code = 'm2.write' ON CONFLICT DO NOTHING",
    )
    .bind(role_id)
    .execute(pool)
    .await
    .expect("grant verifier permission");

    for (index, user_id) in user_ids.iter().enumerate() {
        sqlx::query(
            "INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, $3, 'test-hash', 'active') ON CONFLICT (id) DO NOTHING",
        )
        .bind(*user_id)
        .bind(format!("m2-verifier-{index}-{}", &user_id.to_string()[..8]))
        .bind(format!("收货验收测试员 {index}"))
        .execute(pool)
        .await
        .expect("seed verifier user");
        sqlx::query(
            "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, $3) ON CONFLICT (user_id, owner_id) DO UPDATE SET is_active = TRUE",
        )
        .bind(*user_id)
        .bind(owner_id)
        .bind(index == 0)
        .execute(pool)
        .await
        .expect("bind verifier to owner");
        sqlx::query(
            "INSERT INTO auth_user_roles (user_id, owner_id, role_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(*user_id)
        .bind(owner_id)
        .bind(role_id)
        .execute(pool)
        .await
        .expect("assign verifier role");
    }
}
