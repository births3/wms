use super::*;

pub(super) async fn resolve_outbound_review_policy(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    order_id: Uuid,
    order: &OutboundOrder,
    second_reviewer_id: Option<Uuid>,
) -> Result<
    (
        crate::dual_person_policy::ResolvedDualPersonPolicy,
        Option<Uuid>,
    ),
    Wave4RepositoryError,
> {
    let product_codes = order
        .lines
        .iter()
        .map(|line| line.product_code.clone())
        .collect::<Vec<_>>();
    let strategy = crate::dual_person_policy::resolve_for_product_codes_in_tx(
        tx,
        ctx.owner_id,
        order.warehouse_id,
        &product_codes,
        "出库",
        "复核",
    )
    .await
    .map_err(|error| Wave4RepositoryError::Database(format!("M-VR 双人策略解析失败: {error:?}")))?;
    if strategy.policy != wms_domain::DualPersonPolicy::Single && second_reviewer_id.is_none() {
        return Err(Wave4RepositoryError::MissingSecondReviewer);
    }
    if let Some(second_reviewer_id) = second_reviewer_id {
        let qualified = crate::dual_person_policy::is_active_operator_with_role_in_tx(
            tx,
            ctx.owner_id,
            second_reviewer_id,
            "custodian",
        )
        .await
        .map_err(|error| {
            Wave4RepositoryError::Database(format!("M-VR 第二复核员资质校验失败: {error:?}"))
        })?;
        if !qualified {
            return Err(Wave4RepositoryError::UnqualifiedSecondReviewer);
        }
    }
    let approval_record_id =
        if strategy.policy == wms_domain::DualPersonPolicy::DualScanWithApproval {
            crate::dual_person_policy::approved_dual_person_record_in_tx(
                tx,
                ctx.owner_id,
                &order_id.to_string(),
            )
            .await
            .map_err(|error| {
                Wave4RepositoryError::Database(format!("M-VR 审批记录查询失败: {error:?}"))
            })?
            .ok_or(Wave4RepositoryError::DualPersonApprovalRequired)?
            .into()
        } else {
            None
        };
    Ok((strategy, approval_record_id))
}
