use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "../../migrations")]
async fn h6_permission_is_granted_only_to_late_system_admin_role(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let system_admin_role_id = Uuid::new_v4();
    let operator_role_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'H6 permission owner')",
    )
    .bind(owner_id)
    .bind(format!("H6-{}", Uuid::new_v4()))
    .execute(&pool)
    .await
    .expect("owner should insert");
    sqlx::query(
        r#"
        INSERT INTO auth_roles (id, owner_id, role_code, role_name)
        VALUES
            ($1, $3, 'system_admin', '系统管理员'),
            ($2, $3, 'warehouse_operator', '仓库操作员')
        "#,
    )
    .bind(system_admin_role_id)
    .bind(operator_role_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("roles should insert after migrations");

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
