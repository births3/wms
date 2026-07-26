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
    BatchEnableAdminMenuRequest, CreateAdminMenuNodeRequest, PublishAdminMenuRequest,
    RollbackAdminMenuRequest, UpdateAdminMenuNodeRequest, UpsertAdminMenuButtonPermissionRequest,
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
        warehouse_scope: None,
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

#[sqlx::test(migrations = "../../migrations")]
async fn seeded_drug_inspection_menu_nodes_can_be_saved(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id).await;
    let service = PgAdminMenuService::new();
    let auth = ctx(owner_id);
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 10, 0, 0)
        .single()
        .expect("valid time");

    for (node_id, view_id, icon_key, permission_key, actions) in [
        (
            "00000000-0000-0000-0000-000000130090",
            "m2-inbound-documents",
            "ClipboardList",
            "m-di.document.read",
            vec![
                ("query", "查询"),
                ("refresh", "刷新"),
                ("upload", "上传"),
                ("reuse", "复用"),
                ("review", "审核"),
                ("detail", "详情"),
            ],
        ),
        (
            "00000000-0000-0000-0000-000000130092",
            "m-di-stamp",
            "Stamp",
            "m-di.stamp.manage",
            vec![
                ("query", "查询"),
                ("upload", "上传图章"),
                ("submit", "提交审核"),
                ("review", "审核发布"),
                ("history", "版本记录"),
            ],
        ),
    ] {
        service
            .update_node(
                &pool,
                &auth,
                Uuid::parse_str(node_id).expect("static menu node uuid"),
                UpdateAdminMenuNodeRequest {
                    view_id: Some(view_id.to_string()),
                    icon_key: Some(icon_key.to_string()),
                    permission_key: Some(permission_key.to_string()),
                    button_permissions: Some(
                        actions
                            .into_iter()
                            .enumerate()
                            .map(
                                |(index, (key, label))| UpsertAdminMenuButtonPermissionRequest {
                                    action_key: key.to_string(),
                                    action_label: label.to_string(),
                                    action_kind: "standard".to_string(),
                                    enabled: true,
                                    sort_order: (index as i32 + 1) * 10,
                                },
                            )
                            .collect(),
                    ),
                    ..Default::default()
                },
                now,
                &format!("save-{view_id}"),
            )
            .await
            .expect("seeded drug inspection menu should remain editable");
    }
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
async fn admin_menu_draft_publish_batch_enable_rollback_is_idempotent_and_audited(pool: PgPool) {
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
    assert!(has_page_in_group(
        &published,
        "基础能力",
        "H1 权限租户",
        "h1-menu-management"
    ));
    assert!(has_page_in_group(
        &published,
        "基础能力",
        "H2 审计能力",
        "h2-audit-trail"
    ));
    assert!(has_page_in_group(
        &published,
        "基础能力",
        "H3 契约能力",
        "h3-api-contract"
    ));
    assert!(has_page_in_group(
        &published,
        "基础能力",
        "H4 企业微信",
        "h4-notify-configs"
    ));
    assert!(has_page_in_group(
        &published,
        "基础能力",
        "H5 快递能力",
        "h5-express"
    ));
    assert!(has_page_in_group(
        &published,
        "基础能力",
        "H9 打印能力",
        "h9-print-templates"
    ));
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
    let created_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM admin_menu_draft_nodes WHERE code = 'platform.extra'",
    )
    .fetch_one(&pool)
    .await
    .expect("created menu row count should query");
    let create_audit_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id = $1 AND action = 'admin_menu.create_node' AND resource_id = $2",
    )
    .bind(owner_id)
    .bind("h1-menu-create-extra")
    .fetch_one(&pool)
    .await
    .expect("create menu audit count should query");
    assert_eq!(created_rows, 1);
    assert_eq!(create_audit_rows, 1);

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

    let batch_request = BatchEnableAdminMenuRequest {
        ids: vec![created.value.id],
        enabled: false,
    };
    let batch_enabled = service
        .batch_enable(
            &pool,
            &auth,
            batch_request.clone(),
            now,
            "h1-menu-batch-enable",
        )
        .await
        .expect("batch enable should update draft nodes");
    let batch_replay = service
        .batch_enable(&pool, &auth, batch_request, now, "h1-menu-batch-enable")
        .await
        .expect("batch enable should replay the same idempotency key");
    assert!(!batch_enabled.replayed);
    assert!(batch_replay.replayed);

    let rollback_request = RollbackAdminMenuRequest {
        target_version_no: Some(1),
    };
    let rollback = service
        .rollback(
            &pool,
            &auth,
            rollback_request.clone(),
            now,
            "h1-menu-rollback",
        )
        .await
        .expect("menu rollback should publish a restored version");
    let rollback_replay = service
        .rollback(&pool, &auth, rollback_request, now, "h1-menu-rollback")
        .await
        .expect("menu rollback should replay the same idempotency key");
    assert!(!rollback.replayed);
    assert!(rollback_replay.replayed);
    assert_eq!(rollback.value.id, rollback_replay.value.id);

    let governance_counts: (i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'admin_menu.batch_enable'),
            (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'admin_menu.rollback'),
            (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = 'h1-menu-batch-enable'),
            (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = 'h1-menu-rollback')
        "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("menu audit and idempotency evidence should query");
    assert_eq!(governance_counts, (1, 1, 1, 1));

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

fn has_page_in_group(
    nodes: &[wms_domain::AdminMenuNode],
    section_title: &str,
    group_title: &str,
    view_id: &str,
) -> bool {
    nodes.iter().any(|section| {
        section.title == section_title
            && section.children.iter().any(|group| {
                group.title == group_title
                    && group
                        .children
                        .iter()
                        .any(|page| page.view_id.as_deref() == Some(view_id))
            })
    })
}
