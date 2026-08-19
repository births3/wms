use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "../../migrations")]
async fn h4_permissions_are_granted_to_system_admin(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    sqlx::query("ALTER TABLE auth_roles DISABLE TRIGGER auth_roles_grant_system_admin_permissions")
        .execute(&pool)
        .await
        .expect("automatic system admin grant should be disabled for migration 002 test");
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'H4 test owner')",
    )
    .bind(owner_id)
    .bind(format!("H4-{}", Uuid::new_v4()))
    .execute(&pool)
    .await
    .expect("owner should insert");
    let role_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM auth_roles WHERE owner_id = $1 AND lower(role_code) = 'system_admin'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("system admin role should be seeded");
    let h4_permissions = vec![
        "h4.notify.read",
        "h4.notify.write",
        "h4.notify.send",
        "h4.approval.write",
    ];
    sqlx::query(
        "DELETE FROM auth_role_permissions WHERE role_id = $1 AND permission_id IN (SELECT id FROM auth_permissions WHERE permission_code = ANY($2))",
    )
    .bind(role_id)
    .bind(&h4_permissions)
    .execute(&pool)
    .await
    .expect("H4 permissions should be reset for backfill test");
    let count_before_backfill: i64 =
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM auth_role_permissions role_permission JOIN auth_permissions permission ON permission.id = role_permission.permission_id WHERE role_permission.role_id = $1 AND permission.permission_code = ANY($2)",
        )
            .bind(role_id)
            .bind(&h4_permissions)
            .fetch_one(&pool)
            .await
            .expect("pre-backfill permissions should query");
    assert_eq!(
        count_before_backfill, 0,
        "test must prove migration 002 performs the backfill"
    );
    sqlx::raw_sql(include_str!(
        "../../../migrations/202607100002_h4_system_admin_permissions.sql"
    ))
    .execute(&pool)
    .await
    .expect("H4 permission migration should backfill existing system admin");
    sqlx::query("ALTER TABLE auth_roles ENABLE TRIGGER auth_roles_grant_system_admin_permissions")
        .execute(&pool)
        .await
        .expect("automatic system admin grant should be restored");

    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
          FROM auth_role_permissions role_permission
          JOIN auth_roles role ON role.id = role_permission.role_id
          JOIN auth_permissions permission ON permission.id = role_permission.permission_id
         WHERE role.role_code = 'system_admin'
           AND permission.permission_code = ANY($1)
        "#,
    )
    .bind(h4_permissions)
    .fetch_one(&pool)
    .await
    .expect("system admin H4 permissions should query");

    assert_eq!(count, 4);
}

#[sqlx::test(migrations = "../../migrations")]
async fn system_admin_created_after_migrations_receives_all_registered_permissions(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'Late role owner')",
    )
    .bind(owner_id)
    .bind(format!("LATE-{}", Uuid::new_v4()))
    .execute(&pool)
    .await
    .expect("owner should insert");
    let role_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM auth_roles WHERE owner_id = $1 AND lower(role_code) = 'system_admin'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("system admin role should be seeded after owner creation");

    let permission_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM auth_role_permissions WHERE role_id = $1")
            .bind(role_id)
            .fetch_one(&pool)
            .await
            .expect("system admin permissions should query");
    let registered_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM auth_permissions")
        .fetch_one(&pool)
        .await
        .expect("registered permissions should query");

    assert_eq!(permission_count, registered_count);
    assert!(registered_count > 0);

    let rename_error =
        sqlx::query("UPDATE auth_roles SET role_code = 'warehouse_manager' WHERE id = $1")
            .bind(role_id)
            .execute(&pool)
            .await
            .expect_err("built-in system_admin role code must be immutable");
    assert!(rename_error
        .to_string()
        .contains("system_admin role_code is immutable"));

    let second_owner_id = Uuid::new_v4();
    let operator_role_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'Reserved role owner')",
    )
    .bind(second_owner_id)
    .bind(format!("RESERVED-{}", Uuid::new_v4()))
    .execute(&pool)
    .await
    .expect("second owner should insert");
    sqlx::query(
        "INSERT INTO auth_roles (id, owner_id, role_code, role_name) VALUES ($1, $2, 'warehouse_operator', '仓库操作员')",
    )
    .bind(operator_role_id)
    .bind(second_owner_id)
    .execute(&pool)
    .await
    .expect("operator role should insert");
    let promotion_error =
        sqlx::query("UPDATE auth_roles SET role_code = 'system_admin' WHERE id = $1")
            .bind(operator_role_id)
            .execute(&pool)
            .await
            .expect_err("system_admin role code must be reserved");
    assert!(promotion_error
        .to_string()
        .contains("system_admin role_code is immutable"));
}
