use std::collections::HashSet;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{TaskTransitionAction, TransitionWarehouseTaskRequest};

use crate::{
    auth::AuthContext,
    task_engine::{PgTaskEngineRepository, TaskEngineError},
};

const ACTOR: &str = "system-scheduler";

#[derive(Debug)]
pub enum TaskReleaseJobError {
    Database(String),
    TaskEngine(TaskEngineError),
}

pub async fn run_once(pool: &PgPool, now: DateTime<Utc>) -> Result<usize, TaskReleaseJobError> {
    let due: Vec<(Uuid, Uuid, String, String, Option<i32>, i64)> = sqlx::query_as(
        r#"
        WITH ranked AS (
            SELECT task.id, task.owner_id, task.task_type_code,
                   task_type.release_strategy,
                   task_type.release_interval_minutes,
                   task_type.release_batch_size,
                   task.version,
                   row_number() OVER (
                       PARTITION BY task.owner_id, task.task_type_code
                       ORDER BY task.priority DESC, task.created_at, task.id
                   ) AS release_rank,
                   row_number() OVER (
                       PARTITION BY task.owner_id, task.warehouse_id, task.task_group_code
                       ORDER BY task.priority DESC, task.created_at, task.id
                   ) AS capacity_rank,
                   CASE WHEN task_type.release_strategy = 'capacity' THEN
                       GREATEST(
                           COALESCE((
                               SELECT sum(worker_capacity.free_slots)
                                 FROM (
                                     SELECT GREATEST(
                                                COALESCE(membership.max_active_tasks, 2147483647)::BIGINT
                                                - count(active_task.id),
                                                0
                                            ) AS free_slots
                                       FROM task_groups task_group
                                       JOIN task_group_memberships membership
                                         ON membership.task_group_id = task_group.id
                                        AND membership.owner_id = task_group.owner_id
                                       JOIN auth_users auth_user ON auth_user.id = membership.user_id
                                       JOIN auth_user_owner_bindings binding
                                         ON binding.user_id = membership.user_id
                                        AND binding.owner_id = membership.owner_id
                                       LEFT JOIN warehouse_tasks active_task
                                         ON active_task.owner_id = membership.owner_id
                                        AND active_task.assignee_user_id = membership.user_id
                                        AND active_task.status IN ('assigned', 'dispatched', 'in_progress')
                                      WHERE task_group.owner_id = task.owner_id
                                        AND task_group.task_group_code = task.task_group_code
                                        AND task_group.warehouse_id = task.warehouse_id
                                        AND task_group.enabled
                                        AND task.task_type_code = ANY(task_group.task_type_codes)
                                        AND auth_user.status = 'active'
                                        AND binding.is_active
                                        AND (membership.qualification_valid_until IS NULL OR
                                             membership.qualification_valid_until > $1)
                                      GROUP BY membership.user_id, membership.max_active_tasks
                                 ) worker_capacity
                           ), 0)
                           - (
                               SELECT count(*)
                                 FROM warehouse_tasks queued_task
                                WHERE queued_task.owner_id = task.owner_id
                                  AND queued_task.warehouse_id = task.warehouse_id
                                  AND queued_task.task_group_code = task.task_group_code
                                  AND queued_task.status = 'pending_assignment'
                           ),
                           0
                       )
                   END AS available_capacity
              FROM warehouse_tasks task
              JOIN task_types task_type
                ON task_type.owner_id = task.owner_id
               AND task_type.task_type_code = task.task_type_code
             WHERE task.status = 'pending_release'
               AND task_type.enabled
               AND (
                    task_type.release_strategy = 'immediate'
                    OR
                    (task_type.release_strategy = 'scheduled' AND task.release_due_at <= $1)
                    OR
                    (task_type.release_strategy = 'conditional' AND EXISTS (
                        SELECT 1
                          FROM warehouse_tasks predecessor
                         WHERE predecessor.owner_id = task.owner_id
                           AND predecessor.id = task.predecessor_task_id
                           AND predecessor.status = 'completed'
                    ))
                    OR
                    task_type.release_strategy = 'capacity'
               )
        )
        SELECT id, owner_id, task_type_code, release_strategy, release_interval_minutes, version
          FROM ranked
         WHERE (release_strategy <> 'scheduled' OR release_rank <= release_batch_size)
           AND (release_strategy <> 'capacity' OR capacity_rank <= available_capacity)
         ORDER BY owner_id, task_type_code, release_rank
        "#,
    )
    .bind(now)
    .fetch_all(pool)
    .await
    .map_err(|error| TaskReleaseJobError::Database(error.to_string()))?;

    let repository = PgTaskEngineRepository::new(pool.clone());
    let mut released = 0;
    let mut scheduled_types = HashSet::new();
    for (task_id, owner_id, task_type_code, release_strategy, interval_minutes, version) in due {
        let ctx = AuthContext {
            user_id: Uuid::nil(),
            owner_id,
            actor_name: ACTOR.to_string(),
            permissions: vec!["mte.task.assign".to_string()],
            jti: format!("{ACTOR}:mte-release:{task_id}"),
        };
        repository
            .transition_task(
                &ctx,
                task_id,
                TransitionWarehouseTaskRequest {
                    action: TaskTransitionAction::Release,
                    assignee_user_id: None,
                    actual_qty: None,
                    exception_code: None,
                    exception_note: None,
                },
                now,
                &format!("mte-release-job:{task_id}:{version}"),
            )
            .await
            .map_err(TaskReleaseJobError::TaskEngine)?;
        released += 1;
        if release_strategy == "scheduled" {
            scheduled_types.insert((owner_id, task_type_code, interval_minutes.unwrap_or(1)));
        }
    }

    for (owner_id, task_type_code, interval_minutes) in scheduled_types {
        sqlx::query(
            r#"
            UPDATE warehouse_tasks
               SET release_due_at = $1 + make_interval(mins => $4),
                   updated_at = $1
             WHERE owner_id = $2
               AND task_type_code = $3
               AND status = 'pending_release'
               AND release_due_at <= $1
            "#,
        )
        .bind(now)
        .bind(owner_id)
        .bind(task_type_code)
        .bind(interval_minutes)
        .execute(pool)
        .await
        .map_err(|error| TaskReleaseJobError::Database(error.to_string()))?;
    }

    let timed_out: Vec<(Uuid, Uuid, i64)> = sqlx::query_as(
        r#"
        SELECT id, owner_id, version
          FROM warehouse_tasks
         WHERE status = 'dispatched'
           AND started_at IS NULL
           AND dispatched_at + make_interval(mins => estimated_minutes) <= $1
         ORDER BY owner_id, dispatched_at, id
        "#,
    )
    .bind(now)
    .fetch_all(pool)
    .await
    .map_err(|error| TaskReleaseJobError::Database(error.to_string()))?;
    for (task_id, owner_id, version) in timed_out {
        let ctx = AuthContext {
            user_id: Uuid::nil(),
            owner_id,
            actor_name: ACTOR.to_string(),
            permissions: vec!["mte.task.assign".to_string()],
            jti: format!("{ACTOR}:mte-timeout:{task_id}:{version}"),
        };
        match repository
            .transition_task(
                &ctx,
                task_id,
                TransitionWarehouseTaskRequest {
                    action: TaskTransitionAction::Reassign,
                    assignee_user_id: None,
                    actual_qty: None,
                    exception_code: None,
                    exception_note: None,
                },
                now,
                &format!("mte-timeout-reassign:{task_id}:{version}"),
            )
            .await
        {
            Ok(_) => released += 1,
            Err(TaskEngineError::NoAvailableWorker) => {}
            Err(error) => return Err(TaskReleaseJobError::TaskEngine(error)),
        }
    }
    Ok(released)
}

pub fn spawn(pool: PgPool) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(error) = run_once(&pool, Utc::now()).await {
                tracing::error!(?error, "M-TE 任务自动释放失败");
            }
        }
    });
}
