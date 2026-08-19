use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "../../migrations")]
async fn h6_permission_is_granted_only_to_seeded_system_admin_role(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let operator_role_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'H6 permission owner')",
    )
    .bind(owner_id)
    .bind(format!("H6-{}", Uuid::new_v4()))
    .execute(&pool)
    .await
    .expect("owner should insert");
    let system_admin_role_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM auth_roles WHERE owner_id = $1 AND lower(role_code) = 'system_admin'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("system admin role should be seeded");
    sqlx::query(
        r#"
        INSERT INTO auth_roles (id, owner_id, role_code, role_name)
        VALUES ($1, $2, 'warehouse_operator', '仓库操作员')
        "#,
    )
    .bind(operator_role_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("operator role should insert after migrations");

    let rows = sqlx::query_as::<_, (Uuid, String)>(
        r#"
        SELECT role_permission.role_id, permission.permission_code
          FROM auth_role_permissions role_permission
          JOIN auth_permissions permission ON permission.id = role_permission.permission_id
         WHERE role_permission.role_id = ANY($1)
           AND permission.permission_code = 'h6.state_machine.read'
         ORDER BY role_permission.role_id
        "#,
    )
    .bind(vec![system_admin_role_id, operator_role_id])
    .fetch_all(&pool)
    .await
    .expect("H6 permission grants should query");

    assert_eq!(
        rows,
        vec![(system_admin_role_id, "h6.state_machine.read".to_string())]
    );
}
