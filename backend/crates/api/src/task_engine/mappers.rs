impl From<TaskGroupRow> for TaskGroup {
    fn from(row: TaskGroupRow) -> Self {
        let member_qualifications = row
            .member_user_ids
            .iter()
            .copied()
            .zip(row.member_qualification_valid_until.iter().copied())
            .zip(row.member_max_active_tasks.iter().copied())
            .map(
                |((user_id, valid_until), max_active_tasks)| TaskGroupMemberQualification {
                    user_id,
                    valid_until,
                    max_active_tasks,
                },
            )
            .collect();
        Self {
            id: row.id,
            owner_id: row.owner_id,
            task_group_code: row.task_group_code,
            task_group_name: row.task_group_name,
            warehouse_id: row.warehouse_id,
            zone_ids: row.zone_ids,
            task_type_codes: row.task_type_codes,
            member_user_ids: row.member_user_ids,
            member_qualifications,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
            version: row.version,
        }
    }
}

impl From<WarehouseTaskRow> for WarehouseTask {
    fn from(row: WarehouseTaskRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            task_no: row.task_no,
            task_type_code: row.task_type_code,
            source_module: row.source_module,
            source_doc_type: row.source_doc_type,
            source_doc_id: row.source_doc_id,
            source_doc_no: row.source_doc_no,
            source_line_no: row.source_line_no,
            source_task_key: row.source_task_key,
            warehouse_id: row.warehouse_id,
            task_group_code: row.task_group_code,
            product_id: row.product_id,
            product_code: row.product_code,
            batch_id: row.batch_id,
            batch_no: row.batch_no,
            planned_qty: row.planned_qty,
            actual_qty: row.actual_qty,
            source_location_id: row.source_location_id,
            source_location_code: row.source_location_code,
            target_location_id: row.target_location_id,
            target_location_code: row.target_location_code,
            priority: row.priority,
            urgent_order: row.urgent_order,
            cold_chain: row.cold_chain,
            manually_expedited: row.manually_expedited,
            estimated_minutes: row.estimated_minutes,
            predecessor_task_id: row.predecessor_task_id,
            release_due_at: row.release_due_at,
            released_at: row.released_at,
            assignee_user_id: row.assignee_user_id,
            status: row.status,
            exception_code: row.exception_code,
            exception_note: row.exception_note,
            assigned_at: row.assigned_at,
            dispatched_at: row.dispatched_at,
            started_at: row.started_at,
            completed_at: row.completed_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
            version: row.version,
        }
    }
}


impl From<TaskPriorityRuleRow> for TaskPriorityRule {
    fn from(row: TaskPriorityRuleRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            urgent_order_bonus: row.urgent_order_bonus,
            waiting_minutes_per_point: row.waiting_minutes_per_point,
            cold_chain_bonus: row.cold_chain_bonus,
            manual_expedite_bonus: row.manual_expedite_bonus,
            created_at: row.created_at,
            updated_at: row.updated_at,
            version: row.version,
        }
    }
}
