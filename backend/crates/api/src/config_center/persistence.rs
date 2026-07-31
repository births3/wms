use uuid::Uuid;
use wms_domain::{FeatureFlagConfig, FeatureFlagExportResponse};

use crate::audit::{append_event_in_tx, AuditWriteRequest};

use super::{ConfigCenterAppState, ConfigCenterHandlerError};

impl ConfigCenterAppState {
    pub(super) async fn persist_feature_flags(
        &self,
        ctx: &crate::operation_context::OperationContext,
        action: &str,
        flags: &[FeatureFlagConfig],
    ) -> Result<(), ConfigCenterHandlerError> {
        let Some(pool) = &self.pool else {
            return Ok(());
        };
        let mut tx = pool
            .begin()
            .await
            .map_err(|error| ConfigCenterHandlerError::Storage(error.to_string()))?;
        for flag in flags {
            sqlx::query(
                r#"
                INSERT INTO config_center_feature_flags
                    (owner_id, flag_key, owner, created_at, cleanup_by, enabled, source)
                VALUES ($1, $2, $3, $4, $5, $6, 'm1_config_center')
                ON CONFLICT (owner_id, flag_key) DO UPDATE SET
                    owner = EXCLUDED.owner,
                    created_at = EXCLUDED.created_at,
                    cleanup_by = EXCLUDED.cleanup_by,
                    enabled = EXCLUDED.enabled,
                    source = EXCLUDED.source,
                    updated_at = now()
                "#,
            )
            .bind(ctx.owner_id)
            .bind(&flag.key)
            .bind(&flag.owner)
            .bind(&flag.created_at)
            .bind(&flag.cleanup_by)
            .bind(flag.enabled)
            .execute(&mut *tx)
            .await
            .map_err(|error| ConfigCenterHandlerError::Storage(error.to_string()))?;
        }
        append_event_in_tx(
            &mut tx,
            &AuditWriteRequest::from_auth_context(
                ctx,
                action,
                "M1",
                "feature_flag",
                "feature_flags",
                None,
            ),
        )
        .await
        .map_err(|error| ConfigCenterHandlerError::Audit(format!("{error:?}")))?;
        tx.commit()
            .await
            .map_err(|error| ConfigCenterHandlerError::Storage(error.to_string()))?;
        Ok(())
    }

    pub(super) async fn export_feature_flags_from_postgres(
        &self,
        owner_id: Uuid,
    ) -> Result<FeatureFlagExportResponse, ConfigCenterHandlerError> {
        let Some(pool) = &self.pool else {
            return Ok(self.store.lock().await.export_feature_flags());
        };
        let rows: Vec<(String, String, String, String, bool, String)> = sqlx::query_as(
            r#"
            SELECT flag_key, owner, created_at, cleanup_by, enabled, source
              FROM config_center_feature_flags
             WHERE owner_id = $1
             ORDER BY flag_key
            "#,
        )
        .bind(owner_id)
        .fetch_all(pool)
        .await
        .map_err(|error| ConfigCenterHandlerError::Storage(error.to_string()))?;
        let source = self.store.lock().await.active_source_name().to_string();
        Ok(FeatureFlagExportResponse {
            source,
            flags: rows
                .into_iter()
                .map(
                    |(key, owner, created_at, cleanup_by, enabled, source)| FeatureFlagConfig {
                        key,
                        owner,
                        created_at,
                        cleanup_by,
                        enabled,
                        source,
                    },
                )
                .collect(),
        })
    }
}
