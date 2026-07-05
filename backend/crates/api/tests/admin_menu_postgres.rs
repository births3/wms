use chrono::{TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    admin_menu::{
        AdminMenuError, PgAdminMenuService, ADMIN_MENU_PUBLISH_PERMISSION,
        ADMIN_MENU_READ_PERMISSION, ADMIN_MENU_WRITE_PERMISSION,
    },
    auth::AuthContext,
};
use wms_domain::{
    CreateAdminMenuNodeRequest, PublishAdminMenuRequest, UpsertAdminMenuButtonPermissionRequest,
};

fn ctx(owner_id: Uuid) -> AuthContext {
    ctx_with_permissions(
        owner_id,
        &[
            ADMIN_MENU_READ_PERMISSION,
            ADMIN_MENU_WRITE_PERMISSION,
            ADMIN_MENU_PUBLISH_PERMISSION,
        ],
    )
}

fn ctx_with_permissions(owner_id: Uuid, permissions: &[&str]) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "admin-menu-test".to_string(),
        permissions: permissions
            .iter()
            .map(|permission| (*permission).to_string())
            .collect(),
        jti: Uuid::new_v4().to_string(),
    }
}

async fn seed_owner(pool: &PgPool, owner_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO auth_owners (id, owner_code, owner_name)
        VALUES ($1, 'MENU_OWNER', '菜单测试货主')
        "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("owner should insert");
}

fn platform_extra_request() -> CreateAdminMenuNodeRequest {
    CreateAdminMenuNodeRequest {
        parent_id: Some(
            Uuid::parse_str("00000000-0000-0000-0000-000000110006").expect("static uuid"),
        ),
        code: "platform.extra".to_string(),
        title: "扩展平台能力".to_string(),
        view_id: None,
        icon_key: "ShieldCheck".to_string(),
        permission_key: "menu.platform.extra".to_string(),
        sort_order: 90,
        enabled: true,
        button_permissions: Vec::<UpsertAdminMenuButtonPermissionRequest>::new(),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_menu_draft_publish_is_versioned_idempotent_and_validates_registered_views(
    pool: PgPool,
) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id).await;
    let service = PgAdminMenuService::new();
    let auth = ctx(owner_id);
    let now = Utc
        .with_ymd_and_hms(2026, 7, 5, 10, 0, 0)
        .single()
        .expect("valid time");

    let (published, version_no) = service
        .list_published_tree(&pool, &auth)
        .await
        .expect("published menu should list");
    assert_eq!(version_no, Some(1));
    assert!(published.iter().any(|node| node.title == "基础能力"));
    assert!(published.iter().any(|node| {
        node.children.iter().any(|group| {
            group
                .children
                .iter()
                .any(|page| page.view_id.as_deref() == Some("h1-menu-management"))
        })
    }));
    let inbound_user = ctx_with_permissions(owner_id, &["m2.write"]);
    let (inbound_menu, _) = service
        .list_published_tree(&pool, &inbound_user)
        .await
        .expect("business permission should see matching menus");
    assert!(inbound_menu.iter().any(|node| {
        node.children.iter().any(|group| {
            group
                .children
                .iter()
                .any(|page| page.view_id.as_deref() == Some("m2-receiving"))
        })
    }));
    assert!(!inbound_menu.iter().any(|node| {
        node.children.iter().any(|group| {
            group
                .children
                .iter()
                .any(|page| page.view_id.as_deref() == Some("h1-menu-management"))
        })
    }));

    let created = service
        .create_node(
            &pool,
            &auth,
            platform_extra_request(),
            now,
            "h1-menu-create-extra",
        )
        .await
        .expect("draft node should create");
    let replay = service
        .create_node(
            &pool,
            &auth,
            platform_extra_request(),
            now,
            "h1-menu-create-extra",
        )
        .await
        .expect("same idempotency key should replay");
    assert_eq!(created.value.id, replay.value.id);
    assert!(replay.replayed);

    let published_version = service
        .publish(
            &pool,
            &auth,
            PublishAdminMenuRequest {
                note: Some("发布扩展平台能力".to_string()),
            },
            now,
            "h1-menu-publish-extra",
        )
        .await
        .expect("menu should publish");
    assert_eq!(published_version.value.version_no, 2);

    let invalid = service
        .create_node(
            &pool,
            &auth,
            CreateAdminMenuNodeRequest {
                parent_id: Some(
                    Uuid::parse_str("00000000-0000-0000-0000-000000120008").expect("static uuid"),
                ),
                code: "platform.missing_view".to_string(),
                title: "不存在页面".to_string(),
                view_id: Some("missing-view".to_string()),
                icon_key: "ShieldCheck".to_string(),
                permission_key: "menu.platform.missing_view".to_string(),
                sort_order: 99,
                enabled: true,
                button_permissions: Vec::new(),
            },
            now,
            "h1-menu-invalid-view",
        )
        .await
        .expect_err("unknown view_id should fail");
    assert_eq!(invalid, AdminMenuError::UnknownView);
}
