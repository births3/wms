use std::sync::Arc;

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{CreateOutboundWaveRequest, OutboundWave};

use crate::{
    audit::AuditWriteRequest,
    auth::AuthContext,
    replenishment_service::{ReplenishmentError, ReplenishmentService},
    wave4_repository::{IdempotentMutation, PgWave4Repository, Wave4RepositoryError},
};

fn map_fill_error(error: ReplenishmentError) -> Wave4RepositoryError {
    match error {
        ReplenishmentError::Database(error) => Wave4RepositoryError::Database(error.to_string()),
        ReplenishmentError::PermissionDenied
        | ReplenishmentError::StrategyInvalid
        | ReplenishmentError::ScopeNotFound
        | ReplenishmentError::LocationBound
        | ReplenishmentError::TaskNotFound
        | ReplenishmentError::StrategyNotFound
        | ReplenishmentError::GroupNotFound
        | ReplenishmentError::SourceUnavailable
        | ReplenishmentError::NumberingUnavailable
        | ReplenishmentError::PutawayBlocked
        | ReplenishmentError::ClaimConflict
        | ReplenishmentError::QtyExceeded
        | ReplenishmentError::SourceMismatch
        | ReplenishmentError::TargetMismatch
        | ReplenishmentError::StateInvalid
        | ReplenishmentError::CancelBlocked
        | ReplenishmentError::ReturnBlocked
        | ReplenishmentError::ZoneDenied
        | ReplenishmentError::IdempotencyRequired
        | ReplenishmentError::IdempotencyConflict => Wave4RepositoryError::ReplenishmentGap,
    }
}

pub struct Wave4ReplenishService {
    pool: PgPool,
    waves: Arc<PgWave4Repository>,
    replenishment: Arc<ReplenishmentService>,
}

impl Wave4ReplenishService {
    pub fn new(
        pool: PgPool,
        waves: Arc<PgWave4Repository>,
        replenishment: Arc<ReplenishmentService>,
    ) -> Self {
        Self {
            pool,
            waves,
            replenishment,
        }
    }

    pub async fn create_outbound_wave(
        &self,
        ctx: &AuthContext,
        req: CreateOutboundWaveRequest,
        now: chrono::DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<OutboundWave>, Wave4RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| Wave4RepositoryError::Database(error.to_string()))?;
        let outcome = self
            .waves
            .create_outbound_wave_in_tx(&mut tx, ctx, req, now, idempotency_key, audit)
            .await?;
        if !outcome.replayed {
            self.replenishment
                .fill_wave_pick_gaps_in_tx(&mut tx, ctx.owner_id, outcome.value.id)
                .await
                .map_err(map_fill_error)?;
        }
        tx.commit()
            .await
            .map_err(|error| Wave4RepositoryError::Database(error.to_string()))?;
        Ok(outcome)
    }

    pub async fn release_outbound_wave(
        &self,
        ctx: &AuthContext,
        wave_id: Uuid,
        now: chrono::DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<OutboundWave>, Wave4RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| Wave4RepositoryError::Database(error.to_string()))?;
        let outcome = self
            .waves
            .release_outbound_wave_in_tx(&mut tx, ctx, wave_id, now, idempotency_key, audit)
            .await?;
        if !outcome.replayed {
            self.replenishment
                .fill_wave_pick_gaps_in_tx(&mut tx, ctx.owner_id, wave_id)
                .await
                .map_err(map_fill_error)?;
        }
        tx.commit()
            .await
            .map_err(|error| Wave4RepositoryError::Database(error.to_string()))?;
        Ok(outcome)
    }
}
