use sqlx::PgPool;

const STATIC_SCHEMA_FINGERPRINT: &str = "e9a6b2972389675ce9022a29bd0c99e9";

#[sqlx::test(migrations = "../../migrations")]
async fn empty_database_migrations_have_stable_schema_and_seeded_contract(pool: PgPool) {
    let static_table_count: i64 = sqlx::query_scalar(
        r#"
        SELECT count(*)
        FROM pg_class AS relation
        JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
        LEFT JOIN pg_inherits AS inheritance ON inheritance.inhrelid = relation.oid
        WHERE namespace.nspname = 'public'
          AND relation.relkind IN ('r', 'p')
          AND relation.relname <> '_sqlx_migrations'
          AND inheritance.inhrelid IS NULL
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("count static PostgreSQL relations");
    assert_eq!(static_table_count, 209);

    let static_index_count: i64 = sqlx::query_scalar(
        r#"
        SELECT count(*)
        FROM pg_indexes AS indexes
        JOIN pg_class AS relation ON relation.relname = indexes.tablename
        JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
        LEFT JOIN pg_inherits AS inheritance ON inheritance.inhrelid = relation.oid
        WHERE indexes.schemaname = 'public'
          AND namespace.nspname = 'public'
          AND relation.relname <> '_sqlx_migrations'
          AND inheritance.inhrelid IS NULL
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("count static PostgreSQL indexes");
    assert_eq!(static_index_count, 642);

    let fingerprint: String = sqlx::query_scalar(
        r#"
        WITH static_relations AS (
            SELECT relation.oid, namespace.nspname, relation.relname
            FROM pg_class AS relation
            JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
            LEFT JOIN pg_inherits AS inheritance ON inheritance.inhrelid = relation.oid
            WHERE namespace.nspname = 'public'
              AND relation.relkind IN ('r', 'p')
              AND relation.relname <> '_sqlx_migrations'
              AND inheritance.inhrelid IS NULL
        ), schema_rows AS (
            SELECT 'column|' || static_relations.nspname || '|' || static_relations.relname || '|'
                || attribute.attnum || '|' || attribute.attname || '|'
                || pg_catalog.format_type(attribute.atttypid, attribute.atttypmod) || '|'
                || attribute.attnotnull || '|'
                || COALESCE(pg_get_expr(default_value.adbin, default_value.adrelid), '') AS value
            FROM static_relations
            JOIN pg_attribute AS attribute ON attribute.attrelid = static_relations.oid
            LEFT JOIN pg_attrdef AS default_value
                ON default_value.adrelid = attribute.attrelid
               AND default_value.adnum = attribute.attnum
            WHERE attribute.attnum > 0
              AND NOT attribute.attisdropped
            UNION ALL
            SELECT 'index|' || indexes.schemaname || '|' || indexes.tablename || '|'
                || indexes.indexname || '|' || indexes.indexdef
            FROM pg_indexes AS indexes
            JOIN static_relations
                ON static_relations.nspname = indexes.schemaname
               AND static_relations.relname = indexes.tablename
            UNION ALL
            SELECT 'constraint|' || namespace.nspname || '|' || relation.relname || '|'
                || constraint_row.conname || '|' || pg_get_constraintdef(constraint_row.oid)
            FROM pg_constraint AS constraint_row
            JOIN pg_class AS relation ON relation.oid = constraint_row.conrelid
            JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
            JOIN static_relations ON static_relations.oid = relation.oid
        )
        SELECT md5(COALESCE(string_agg(value, E'\n' ORDER BY value), ''))
        FROM schema_rows
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("calculate static PostgreSQL schema fingerprint");

    let constraint_count: i64 = sqlx::query_scalar(
        r#"
        SELECT count(*)
        FROM pg_constraint AS constraint_row
        JOIN pg_class AS relation ON relation.oid = constraint_row.conrelid
        JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
        LEFT JOIN pg_inherits AS inheritance ON inheritance.inhrelid = relation.oid
        WHERE namespace.nspname = 'public'
          AND relation.relname <> '_sqlx_migrations'
          AND inheritance.inhrelid IS NULL
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("count static PostgreSQL constraints");

    let (permission_count, category_count, item_count): (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM auth_permissions), (SELECT count(*) FROM system_dictionary_categories), (SELECT count(*) FROM system_dictionary_items)",
    )
    .fetch_one(&pool)
    .await
    .expect("read migration seed rows");
    assert_eq!(permission_count, 133);
    assert_eq!(category_count, 13);
    assert_eq!(item_count, 65);

    // PostgreSQL 18 把 NOT NULL 登记为 pg_constraint.contype='n'。把指纹和约束数
    // 放在同一次断言中，schema 变更时一次 CI 就能给出完整的实际基线。
    assert_eq!(
        (fingerprint.as_str(), constraint_count),
        (STATIC_SCHEMA_FINGERPRINT, 3268)
    );
}
