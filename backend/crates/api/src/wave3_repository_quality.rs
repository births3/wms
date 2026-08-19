async fn resolve_quality_color(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    quality_status: &str,
    now: DateTime<Utc>,
) -> Result<String, Wave3RepositoryError> {
    sqlx::query_scalar(
        r#"
        WITH candidates AS (
            SELECT item.item_code,
                   ROW_NUMBER() OVER (
                       ORDER BY
                           CASE WHEN item.owner_id = $1 THEN 0 ELSE 1 END,
                           item.updated_at DESC,
                           item.item_code
                   ) AS scope_rank
              FROM system_dictionary_items item
              JOIN system_dictionary_categories category
                ON category.dict_code = item.dict_code
               AND category.enabled = TRUE
             WHERE item.dict_code = 'quality_color'
               AND item.params->>'inventory_quality_status' = $2
               AND (item.owner_id IS NULL OR item.owner_id = $1)
               AND item.enabled = TRUE
               AND (item.effective_from IS NULL OR item.effective_from <= $3)
               AND (item.effective_to IS NULL OR item.effective_to > $3)
        )
        SELECT item_code
          FROM candidates
         WHERE scope_rank = 1
        "#,
    )
    .bind(owner_id)
    .bind(quality_status)
    .bind(now)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(Wave3RepositoryError::InvalidQualityStatus)
}
