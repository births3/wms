-- US-TE-002/005：任务组成员资格有效期与同时在手任务容量。

ALTER TABLE task_group_memberships
    ADD COLUMN IF NOT EXISTS qualification_valid_until TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS max_active_tasks INT;

ALTER TABLE task_group_memberships
    DROP CONSTRAINT IF EXISTS task_group_memberships_max_active_tasks_check;
ALTER TABLE task_group_memberships
    ADD CONSTRAINT task_group_memberships_max_active_tasks_check
    CHECK (max_active_tasks IS NULL OR max_active_tasks > 0);

CREATE INDEX IF NOT EXISTS task_group_memberships_active_qualification_idx
    ON task_group_memberships (
        owner_id,
        task_group_id,
        qualification_valid_until,
        user_id
    );
