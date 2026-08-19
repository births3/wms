-- US-TE-006：任务类型释放规则与待释放时间。

ALTER TABLE task_types
    ADD COLUMN IF NOT EXISTS release_strategy TEXT NOT NULL DEFAULT 'immediate',
    ADD COLUMN IF NOT EXISTS release_interval_minutes INT,
    ADD COLUMN IF NOT EXISTS release_batch_size INT;

ALTER TABLE task_types
    DROP CONSTRAINT IF EXISTS task_types_release_strategy_check,
    DROP CONSTRAINT IF EXISTS task_types_release_schedule_check;
ALTER TABLE task_types
    ADD CONSTRAINT task_types_release_strategy_check
        CHECK (release_strategy IN ('immediate', 'scheduled', 'conditional', 'capacity')),
    ADD CONSTRAINT task_types_release_schedule_check
        CHECK (
            (release_strategy = 'scheduled'
                AND release_interval_minutes BETWEEN 1 AND 1440
                AND release_batch_size BETWEEN 1 AND 1000)
            OR
            (release_strategy <> 'scheduled'
                AND release_interval_minutes IS NULL
                AND release_batch_size IS NULL)
        );

ALTER TABLE warehouse_tasks
    ADD COLUMN IF NOT EXISTS predecessor_task_id UUID,
    ADD COLUMN IF NOT EXISTS release_due_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS released_at TIMESTAMPTZ;

ALTER TABLE warehouse_tasks
    DROP CONSTRAINT IF EXISTS warehouse_tasks_predecessor_not_self_check,
    DROP CONSTRAINT IF EXISTS warehouse_tasks_predecessor_task_id_fkey,
    DROP CONSTRAINT IF EXISTS warehouse_tasks_owner_predecessor_fkey;
ALTER TABLE warehouse_tasks
    ADD CONSTRAINT warehouse_tasks_owner_id_id_key UNIQUE (owner_id, id),
    ADD CONSTRAINT warehouse_tasks_owner_predecessor_fkey
        FOREIGN KEY (owner_id, predecessor_task_id)
        REFERENCES warehouse_tasks(owner_id, id) ON DELETE RESTRICT,
    ADD CONSTRAINT warehouse_tasks_predecessor_not_self_check
        CHECK (predecessor_task_id IS NULL OR predecessor_task_id <> id);

UPDATE warehouse_tasks
   SET released_at = created_at
 WHERE status <> 'pending_release'
   AND released_at IS NULL;

CREATE INDEX IF NOT EXISTS warehouse_tasks_owner_release_due_idx
    ON warehouse_tasks (owner_id, release_due_at, priority DESC, created_at, id)
    WHERE status = 'pending_release';
CREATE INDEX IF NOT EXISTS warehouse_tasks_predecessor_idx
    ON warehouse_tasks (owner_id, predecessor_task_id)
    WHERE predecessor_task_id IS NOT NULL;
