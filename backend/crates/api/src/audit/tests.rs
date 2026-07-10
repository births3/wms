#[cfg(test)]
use super::{commit_with_audit, AuditDiff, AuditError, AuditLog, AuditSealProgress};
use crate::auth::AuthContext;
use serde_json::json;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
struct DemoItem {
    id: String,
    owner_id: Uuid,
    name: String,
}

fn ctx() -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id: Uuid::new_v4(),
        actor_name: "alice".to_string(),
        permissions: vec!["demo:write".to_string()],
        jti: "jti-1".to_string(),
    }
}

fn create_demo_item_handler(
    items: &mut BTreeMap<String, DemoItem>,
    audit_log: &mut AuditLog,
    ctx: &AuthContext,
    id: &str,
) -> DemoItem {
    commit_with_audit(audit_log, ctx, "create", "DEMO", "demo_item", || {
        let item = DemoItem {
            id: id.to_string(),
            owner_id: ctx.owner_id,
            name: "item-a".to_string(),
        };
        items.insert(id.to_string(), item.clone());
        (item, id.to_string(), None)
    })
    .expect("audit commit should succeed")
}

fn update_demo_item_handler(
    items: &mut BTreeMap<String, DemoItem>,
    audit_log: &mut AuditLog,
    ctx: &AuthContext,
    id: &str,
    name: &str,
) -> DemoItem {
    commit_with_audit(audit_log, ctx, "update", "DEMO", "demo_item", || {
        let before = items.get(id).expect("item exists").clone();
        let after = DemoItem {
            name: name.to_string(),
            ..before.clone()
        };
        items.insert(id.to_string(), after.clone());
        let diff = AuditDiff::compute(
            json!({"name": before.name, "owner_id": before.owner_id}),
            json!({"name": after.name, "owner_id": after.owner_id}),
        );
        (after, id.to_string(), Some(diff))
    })
    .expect("audit commit should succeed")
}

#[test]
fn two_mutation_handlers_reuse_commit_with_audit() {
    let ctx = ctx();
    let mut items = BTreeMap::new();
    let mut audit_log = AuditLog::default();

    let created = create_demo_item_handler(&mut items, &mut audit_log, &ctx, "ITEM-1");
    let updated = update_demo_item_handler(&mut items, &mut audit_log, &ctx, "ITEM-1", "item-b");

    assert_eq!(created.owner_id, ctx.owner_id);
    assert_eq!(updated.name, "item-b");
    assert_eq!(audit_log.events().len(), 2);
    assert_eq!(audit_log.events()[0].action, "create");
    assert_eq!(audit_log.events()[1].action, "update");
    assert_eq!(audit_log.events()[0].actor_id, ctx.user_id);
    assert_eq!(audit_log.events()[0].owner_id, ctx.owner_id);
    assert_eq!(audit_log.events()[0].jti, ctx.jti);
    assert_eq!(
        audit_log.events()[1]
            .diff
            .as_ref()
            .expect("diff should exist")
            .changed_keys,
        vec!["name".to_string()]
    );
    audit_log
        .verify_hash_chain()
        .expect("hash chain should verify");
}

#[test]
fn hash_chain_detects_tampering_and_can_be_sealed() {
    let ctx = ctx();
    let mut items = BTreeMap::new();
    let mut audit_log = AuditLog::default();

    create_demo_item_handler(&mut items, &mut audit_log, &ctx, "ITEM-1");
    let seal = audit_log
        .seal_latest_chain(chrono::Utc::now())
        .expect("non-empty chain should seal");

    assert_eq!(seal.last_id, 1);
    assert_eq!(seal.last_self_hash, audit_log.events()[0].self_hash);

    audit_log.tamper_self_hash_for_test(1, "tampered");
    assert!(audit_log.verify_hash_chain().is_err());
}

#[test]
fn hash_chain_detects_diff_value_tampering() {
    let ctx = ctx();
    let mut items = BTreeMap::new();
    let mut audit_log = AuditLog::default();

    create_demo_item_handler(&mut items, &mut audit_log, &ctx, "ITEM-1");
    update_demo_item_handler(&mut items, &mut audit_log, &ctx, "ITEM-1", "item-b");

    audit_log.tamper_diff_for_test(
        2,
        AuditDiff::compute(
            json!({"name": "item-a", "owner_id": ctx.owner_id}),
            json!({"name": "item-c", "owner_id": ctx.owner_id}),
        ),
    );

    assert!(audit_log.verify_hash_chain().is_err());
}

#[test]
fn audit_seal_progress_validates_hash_chain_across_batches_without_accumulating_records() {
    let ctx = ctx();
    let mut items = BTreeMap::new();
    let mut audit_log = AuditLog::default();
    create_demo_item_handler(&mut items, &mut audit_log, &ctx, "ITEM-1");
    update_demo_item_handler(&mut items, &mut audit_log, &ctx, "ITEM-1", "item-b");

    let mut progress = AuditSealProgress::new();

    progress
        .observe(audit_log.events()[0].clone())
        .expect("first batch should validate");
    progress
        .observe(audit_log.events()[1].clone())
        .expect("second batch should continue previous hash");

    let (last_id, last_hash) = progress.last().expect("progress should have last record");
    assert_eq!(last_id, audit_log.events()[1].id);
    assert_eq!(last_hash, audit_log.events()[1].self_hash);
    assert_eq!(progress.records_seen, 2);
}

#[test]
fn audit_seal_progress_detects_broken_hash_chain() {
    let ctx = ctx();
    let mut items = BTreeMap::new();
    let mut audit_log = AuditLog::default();
    create_demo_item_handler(&mut items, &mut audit_log, &ctx, "ITEM-1");
    update_demo_item_handler(&mut items, &mut audit_log, &ctx, "ITEM-1", "item-b");

    let mut progress = AuditSealProgress::new();
    progress
        .observe(audit_log.events()[0].clone())
        .expect("first record should validate");
    let mut tampered = audit_log.events()[1].clone();
    tampered.prev_hash = Some("wrong".to_string());

    assert!(matches!(
        progress.observe(tampered),
        Err(AuditError::HashChainBroken { .. })
    ));
}
