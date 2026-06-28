use sqlx::PgPool;

async fn table_exists(pool: &PgPool, table: &str) -> bool {
    sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
        .bind(format!("public.{table}"))
        .fetch_one(pool)
        .await
        .expect("check table exists")
}

async fn column_type(pool: &PgPool, table: &str, column: &str) -> String {
    sqlx::query_scalar(
        r#"
        SELECT data_type
          FROM information_schema.columns
         WHERE table_schema = 'public'
           AND table_name = $1
           AND column_name = $2
        "#,
    )
    .bind(table)
    .bind(column)
    .fetch_one(pool)
    .await
    .expect("read column type")
}

async fn has_composite_fk(pool: &PgPool, child: &str, parent: &str, columns: &[&str]) -> bool {
    let columns: Vec<String> = columns.iter().map(|column| (*column).to_string()).collect();
    sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM pg_constraint c
              JOIN pg_class child_table ON child_table.oid = c.conrelid
              JOIN pg_class parent_table ON parent_table.oid = c.confrelid
             WHERE c.contype = 'f'
               AND child_table.relname = $1
               AND parent_table.relname = $2
               AND ARRAY(
                    SELECT child_column.attname::text
                      FROM unnest(c.conkey) WITH ORDINALITY key(attnum, ord)
                      JOIN pg_attribute child_column
                        ON child_column.attrelid = c.conrelid
                       AND child_column.attnum = key.attnum
                     ORDER BY key.ord
               ) = $3::text[]
        )
        "#,
    )
    .bind(child)
    .bind(parent)
    .bind(columns)
    .fetch_one(pool)
    .await
    .expect("check composite foreign key")
}

#[sqlx::test(migrations = "../../migrations")]
async fn database_schema_follows_design_standards(pool: PgPool) {
    for table in [
        "products",
        "suppliers",
        "customers",
        "customer_addresses",
        "warehouses",
        "warehouse_zones",
        "warehouse_locations",
    ] {
        assert!(
            table_exists(&pool, table).await,
            "{table} table should exist"
        );
    }

    for (table, column) in [
        ("billing_charge_calculations", "period_start"),
        ("billing_charge_calculations", "period_end"),
        ("billing_statements", "period_start"),
        ("billing_statements", "period_end"),
    ] {
        assert_eq!(column_type(&pool, table, column).await, "date");
    }

    for (child, parent, columns) in [
        (
            "receiving_order_lines",
            "receiving_orders",
            ["owner_id", "receiving_order_id"],
        ),
        (
            "inventory_movements",
            "inventory_batches",
            ["owner_id", "batch_id"],
        ),
        (
            "packing_jobs",
            "outbound_orders",
            ["owner_id", "outbound_order_id"],
        ),
        (
            "billing_charge_calculations",
            "billing_contracts",
            ["owner_id", "contract_id"],
        ),
        (
            "billing_statement_charges",
            "billing_statements",
            ["owner_id", "statement_id"],
        ),
    ] {
        assert!(
            has_composite_fk(&pool, child, parent, &columns).await,
            "{child} should reference {parent} with owner_id"
        );
    }
}
