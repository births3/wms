-- US-M3-012 / Phase 2 §6.3：补货 H4 告警定义种子。

CREATE OR REPLACE FUNCTION seed_m3_replenishment_alert_definitions(target_owner_id UUID)
RETURNS VOID LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO alert_definitions (
        id, owner_id, alert_code, name, event_type, condition_expression,
        default_severity, recipient_roles, escalation_ref, silence_period_seconds,
        is_disable_allowed, message_template, is_gsp_forced
    )
    SELECT md5(target_owner_id::text || ':m3-replenish-alert:' || defaults.alert_code)::uuid,
           target_owner_id, defaults.*
      FROM (VALUES
        (
            'replenishment_patrol_fail_repeat',
            '补货巡检连续失败',
            'replenishment_patrol_fail_repeat',
            '{"field":"consecutive_fail_count","op":"gte","value":3}',
            'warning',
            ARRAY['warehouse_manager']::TEXT[],
            'm3-replenish-default',
            300::BIGINT,
            TRUE,
            '同一拣选位补货连续三次生成失败：{{target_location_id}}',
            FALSE
        ),
        (
            'replenishment_urgent_unclaimed',
            '紧急补货超时未领',
            'replenishment_urgent_unclaimed',
            '{"field":"unclaimed_minutes","op":"gte","value":10}',
            'warning',
            ARRAY['warehouse_manager']::TEXT[],
            'm3-replenish-default',
            300::BIGINT,
            TRUE,
            '紧急补货任务超过10分钟未领取：{{task_no}}',
            FALSE
        ),
        (
            'replenishment_urgent_timeout',
            '紧急补货超时取消',
            'replenishment_urgent_timeout',
            '{"field":"unclaimed_minutes","op":"gte","value":20}',
            'critical',
            ARRAY['warehouse_manager']::TEXT[],
            'm3-replenish-default',
            300::BIGINT,
            TRUE,
            '紧急补货任务超过20分钟已自动取消：{{task_no}}',
            FALSE
        ),
        (
            'replenishment_no_progress',
            '补货任务一小时无进展',
            'replenishment_no_progress',
            '{"field":"stale_minutes","op":"gte","value":60}',
            'warning',
            ARRAY['warehouse_manager']::TEXT[],
            'm3-replenish-default',
            300::BIGINT,
            TRUE,
            '补货任务领取后一小时无进展：{{task_no}}',
            FALSE
        ),
        (
            'replenishment_source_frozen',
            '补货来源被冻结',
            'replenishment_source_frozen',
            '{"field":"task_status","op":"eq","value":"suspended"}',
            'critical',
            ARRAY['warehouse_manager']::TEXT[],
            'm3-replenish-default',
            300::BIGINT,
            TRUE,
            '补货来源可下架量不足，任务已挂起：{{task_no}}',
            FALSE
        ),
        (
            'replenishment_source_mismatch',
            '补货来源扫码不符',
            'replenishment_source_mismatch',
            '{"field":"return_reason","op":"eq","value":"source_mismatch"}',
            'warning',
            ARRAY['warehouse_manager']::TEXT[],
            'm3-replenish-default',
            300::BIGINT,
            TRUE,
            '补货退回：扫描来源与任务不符 {{task_no}}',
            FALSE
        )
    ) AS defaults(
        alert_code, name, event_type, condition_expression, default_severity,
        recipient_roles, escalation_ref, silence_period_seconds,
        is_disable_allowed, message_template, is_gsp_forced
    )
    ON CONFLICT (owner_id, alert_code) DO NOTHING;
END;
$$;

CREATE OR REPLACE FUNCTION seed_m3_replenishment_alert_definitions_for_new_owner()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    PERFORM seed_m3_replenishment_alert_definitions(NEW.id);
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_owners_seed_m3_replenishment_alert_definitions ON auth_owners;
CREATE TRIGGER auth_owners_seed_m3_replenishment_alert_definitions
AFTER INSERT ON auth_owners FOR EACH ROW
EXECUTE FUNCTION seed_m3_replenishment_alert_definitions_for_new_owner();

SELECT seed_m3_replenishment_alert_definitions(id) FROM auth_owners;
