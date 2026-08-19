#[sqlx::test(migrations = "../../migrations")]
async fn template_changes_stay_draft_until_separately_published_and_can_be_disabled(pool: PgPool) {
    let repo = PgPrintTemplateRepository::new();
    let owner_id = Uuid::new_v4();
    let auth = ctx(owner_id);
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 11, 0, 0)
        .single()
        .expect("valid time");
    let field_library = published_library(
        &repo,
        &pool,
        &auth,
        "ASN 号",
        now,
        "h9-template-lifecycle-field-library",
    )
    .await;

    let initial_request = template_request(field_library.id);
    let initial_draft = repo
        .save_template(
            &pool,
            &auth,
            initial_request.clone(),
            now + chrono::Duration::minutes(1),
            "h9-template-lifecycle-save-v1",
        )
        .await
        .expect("initial draft should save");
    assert_eq!(initial_draft.value.status, "draft");
    assert_eq!(
        repo.resolve_template(
            &pool,
            &auth,
            wms_api::print_template::ResolvePrintTemplateRequest {
                template_code: Some("m2_asn_default".to_string()),
                template_type_code: PRINT_TEMPLATE_TYPE_ASN.to_string(),
            },
        )
        .await
        .expect_err("draft must not resolve"),
        wms_api::print_template::PrintTemplateError::TemplateNotFound
    );
    let save_replay = repo
        .save_template(
            &pool,
            &auth,
            initial_request.clone(),
            now + chrono::Duration::minutes(1),
            "h9-template-lifecycle-save-v1",
        )
        .await
        .expect("same save key should replay");
    assert_eq!(save_replay.value.id, initial_draft.value.id);
    assert!(save_replay.replayed);
    assert_eq!(
        repo.save_template(
            &pool,
            &auth,
            initial_request,
            now + chrono::Duration::minutes(1),
            "h9-template-lifecycle-duplicate",
        )
        .await
        .expect_err("new template with duplicate code should fail"),
        wms_api::print_template::PrintTemplateError::TemplateDuplicate
    );

    let published_v1 = repo
        .publish_template_draft(
            &pool,
            &auth,
            initial_draft.value.template_id,
            initial_draft.value.id,
            now + chrono::Duration::minutes(2),
            "h9-template-lifecycle-publish-v1",
        )
        .await
        .expect("initial draft should publish");
    assert_eq!(published_v1.value.id, initial_draft.value.id);
    assert_eq!(published_v1.value.status, "published");
    let publish_replay = repo
        .publish_template_draft(
            &pool,
            &auth,
            initial_draft.value.template_id,
            initial_draft.value.id,
            now + chrono::Duration::minutes(2),
            "h9-template-lifecycle-publish-v1",
        )
        .await
        .expect("same publish key should replay");
    assert_eq!(publish_replay.value.id, published_v1.value.id);
    assert!(publish_replay.replayed);

    let upgraded_field_library = published_library(
        &repo,
        &pool,
        &auth,
        "ASN 编号",
        now + chrono::Duration::minutes(3),
        "h9-template-lifecycle-field-library-v2",
    )
    .await;
    let published_v1_after_library_upgrade = repo
        .list_template_versions(&pool, &auth, initial_draft.value.template_id)
        .await
        .expect("template versions should remain queryable")
        .into_iter()
        .find(|version| version.id == published_v1.value.id)
        .expect("published v1 should remain");
    assert_eq!(
        published_v1_after_library_upgrade.field_library_version_id,
        field_library.id
    );

    let mut changed_request = template_request(upgraded_field_library.id);
    changed_request.template_id = Some(initial_draft.value.template_id);
    changed_request.template_name = "M2 ASN 草稿调整模板".to_string();
    changed_request.hiprint_json["panels"][0]["paperType"] = json!("A5");
    let draft_v2 = repo
        .save_template(
            &pool,
            &auth,
            changed_request,
            now + chrono::Duration::minutes(3),
            "h9-template-lifecycle-save-v2",
        )
        .await
        .expect("changed draft should save");
    assert_eq!(draft_v2.value.status, "draft");

    let before_publish = repo
        .resolve_template(
            &pool,
            &auth,
            wms_api::print_template::ResolvePrintTemplateRequest {
                template_code: Some("m2_asn_default".to_string()),
                template_type_code: PRINT_TEMPLATE_TYPE_ASN.to_string(),
            },
        )
        .await
        .expect("published v1 should remain available");
    assert_eq!(before_publish.version.id, published_v1.value.id);
    assert_eq!(before_publish.version.template_name, "M2 ASN 默认模板");
    assert_eq!(
        before_publish.version.hiprint_json["panels"][0]["paperType"],
        "A4"
    );

    let published_v2 = repo
        .publish_template_draft(
            &pool,
            &auth,
            draft_v2.value.template_id,
            draft_v2.value.id,
            now + chrono::Duration::minutes(4),
            "h9-template-lifecycle-publish-v2",
        )
        .await
        .expect("changed draft should publish");
    let after_publish = repo
        .resolve_template(
            &pool,
            &auth,
            wms_api::print_template::ResolvePrintTemplateRequest {
                template_code: Some("m2_asn_default".to_string()),
                template_type_code: PRINT_TEMPLATE_TYPE_ASN.to_string(),
            },
        )
        .await
        .expect("published v2 should become available");
    assert_eq!(after_publish.version.id, published_v2.value.id);
    assert_eq!(after_publish.version.template_name, "M2 ASN 草稿调整模板");
    assert_eq!(
        after_publish.version.hiprint_json["panels"][0]["paperType"],
        "A5"
    );
    assert!(
        sqlx::query("UPDATE print_template_versions SET paper = '{}'::jsonb WHERE id = $1")
            .bind(published_v2.value.id)
            .execute(&pool)
            .await
            .is_err(),
        "published template version must reject updates"
    );
    assert!(
        sqlx::query("DELETE FROM print_template_versions WHERE id = $1")
            .bind(published_v2.value.id)
            .execute(&pool)
            .await
            .is_err(),
        "published template version must reject deletes"
    );

    let disabled = repo
        .set_template_enabled(
            &pool,
            &auth,
            draft_v2.value.template_id,
            false,
            now + chrono::Duration::minutes(5),
            "h9-template-lifecycle-disable",
        )
        .await
        .expect("template should disable");
    assert!(!disabled.value.enabled);
    let disable_replay = repo
        .set_template_enabled(
            &pool,
            &auth,
            draft_v2.value.template_id,
            false,
            now + chrono::Duration::minutes(5),
            "h9-template-lifecycle-disable",
        )
        .await
        .expect("same enabled key should replay");
    assert_eq!(disable_replay.value.id, disabled.value.id);
    assert!(disable_replay.replayed);
    assert_eq!(
        repo.resolve_template(
            &pool,
            &auth,
            wms_api::print_template::ResolvePrintTemplateRequest {
                template_code: Some("m2_asn_default".to_string()),
                template_type_code: PRINT_TEMPLATE_TYPE_ASN.to_string(),
            },
        )
        .await
        .expect_err("disabled template must not resolve"),
        wms_api::print_template::PrintTemplateError::TemplateDisabled
    );

    let audits: Vec<(String, Option<serde_json::Value>)> = sqlx::query_as(
        r#"
        SELECT action, diff
          FROM audit_event
         WHERE module = 'H9'
           AND resource_type = 'print_template'
           AND owner_id = $1
         ORDER BY occurred_at
        "#,
    )
    .bind(owner_id)
    .fetch_all(&pool)
    .await
    .expect("template audits should query");
    assert_eq!(
        audits
            .iter()
            .map(|(action, _)| action.as_str())
            .collect::<Vec<_>>(),
        vec![
            "save_print_template",
            "publish_print_template",
            "save_print_template",
            "publish_print_template",
            "set_print_template_enabled",
        ]
    );
    assert!(audits.iter().all(|(_, diff)| diff.is_some()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn only_latest_template_draft_can_be_published(pool: PgPool) {
    let repo = PgPrintTemplateRepository::new();
    let auth = ctx(Uuid::new_v4());
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 11, 30, 0)
        .single()
        .expect("valid time");
    let field_library = published_library(
        &repo,
        &pool,
        &auth,
        "ASN 号",
        now,
        "h9-template-latest-draft-field-library",
    )
    .await;
    let first = repo
        .save_template(
            &pool,
            &auth,
            template_request(field_library.id),
            now + chrono::Duration::minutes(1),
            "h9-template-latest-draft-v1",
        )
        .await
        .expect("first draft should save");
    let mut next_request = template_request(field_library.id);
    next_request.template_id = Some(first.value.template_id);
    next_request.template_name = "M2 ASN 第二草稿".to_string();
    repo.save_template(
        &pool,
        &auth,
        next_request,
        now + chrono::Duration::minutes(2),
        "h9-template-latest-draft-v2",
    )
    .await
    .expect("second draft should save");

    assert_eq!(
        repo.publish_template_draft(
            &pool,
            &auth,
            first.value.template_id,
            first.value.id,
            now + chrono::Duration::minutes(3),
            "h9-template-publish-stale-draft",
        )
        .await
        .expect_err("an older draft must not be published"),
        wms_api::print_template::PrintTemplateError::TemplateVersionNotLatest
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn template_type_rejects_a_different_field_library(pool: PgPool) {
    let repo = PgPrintTemplateRepository::new();
    let auth = ctx(Uuid::new_v4());
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 11, 45, 0)
        .single()
        .expect("valid time");
    let field_library = published_library(
        &repo,
        &pool,
        &auth,
        "ASN 号",
        now,
        "h9-template-library-mismatch",
    )
    .await;
    let draft = repo
        .save_template(
            &pool,
            &auth,
            template_request(field_library.id),
            now + chrono::Duration::minutes(1),
            "h9-template-library-mismatch-template-draft",
        )
        .await
        .expect("draft should save before dictionary binding changes");
    sqlx::query(
        r#"
        UPDATE system_dictionary_items
           SET params = jsonb_set(params, '{field_library_code}', '"other_library"')
         WHERE dict_code = $1
           AND item_code = $2
           AND owner_id IS NULL
        "#,
    )
    .bind(SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE)
    .bind(PRINT_TEMPLATE_TYPE_ASN)
    .execute(&pool)
    .await
    .expect("test dictionary binding should update");

    assert_eq!(
        repo.save_template(
            &pool,
            &auth,
            template_request(field_library.id),
            now + chrono::Duration::minutes(2),
            "h9-template-library-mismatch-save",
        )
        .await
        .expect_err("template type must reject a different field library"),
        wms_api::print_template::PrintTemplateError::TemplateFieldMismatch(vec![
            "field_library_code:m2_asn".to_string(),
        ])
    );
    assert_eq!(
        repo.publish_template_draft(
            &pool,
            &auth,
            draft.value.template_id,
            draft.value.id,
            now + chrono::Duration::minutes(3),
            "h9-template-library-mismatch-template-publish",
        )
        .await
        .expect_err("draft must revalidate the current template type binding before publish"),
        wms_api::print_template::PrintTemplateError::TemplateFieldMismatch(vec![
            "field_library_code:m2_asn".to_string(),
        ])
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn template_validation_rejects_invalid_hiprint_json_and_unknown_binding(pool: PgPool) {
    let repo = PgPrintTemplateRepository::new();
    let auth = ctx(Uuid::new_v4());
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 10, 0, 0)
        .single()
        .expect("valid time");
    let field_library = published_library(
        &repo,
        &pool,
        &auth,
        "ASN 号",
        now,
        "h9-template-validation-field-library",
    )
    .await;

    let mut invalid_json = template_request(field_library.id);
    invalid_json.hiprint_json = json!({ "panels": "not-an-array" });
    assert_eq!(
        repo.save_template(
            &pool,
            &auth,
            invalid_json,
            now + chrono::Duration::minutes(1),
            "h9-template-validation-json",
        )
        .await
        .expect_err("invalid hiprint JSON must fail"),
        wms_api::print_template::PrintTemplateError::TemplateJsonInvalid
    );

    for (index, option) in [
        "formatter",
        "styler",
        "rowsColumnsMerge",
        "footerFormatter",
        "rowStyler",
        "styler2",
        "renderFormatter",
        "formatter2",
        "gridColumnsFooterFormatter",
    ]
    .into_iter()
    .enumerate()
    {
        let mut executable_json = template_request(field_library.id);
        executable_json.hiprint_json["panels"][0]["printElements"][0]["options"][option] =
            json!("function(value) { return value; }");
        assert_eq!(
            repo.save_template(
                &pool,
                &auth,
                executable_json,
                now + chrono::Duration::minutes(index as i64 + 2),
                &format!("h9-template-validation-executable-{option}"),
            )
            .await
            .expect_err("executable hiprint options must fail"),
            wms_api::print_template::PrintTemplateError::TemplateJsonInvalid,
            "{option} must be rejected"
        );
    }

    let mut unknown_binding = template_request(field_library.id);
    unknown_binding.field_bindings[0].field_path = "missing.field".to_string();
    assert_eq!(
        repo.save_template(
            &pool,
            &auth,
            unknown_binding,
            now + chrono::Duration::minutes(12),
            "h9-template-validation-binding",
        )
        .await
        .expect_err("unknown field binding must fail"),
        wms_api::print_template::PrintTemplateError::TemplateFieldMismatch(vec![
            "missing.field".to_string()
        ])
    );
}
