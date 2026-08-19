-- ============================================================
-- WMS 现有系统表结构导出脚本（Oracle）
-- 执行后将每个查询结果导出为 CSV
-- ============================================================

-- 1. 所有表及表注释
SELECT t.table_name, c.comments AS table_comment
FROM user_tables t
LEFT JOIN user_tab_comments c ON t.table_name = c.table_name
ORDER BY t.table_name;

-- 2. 所有字段详情
SELECT
    tc.table_name,
    tc.column_name,
    tc.data_type,
    tc.data_length,
    tc.data_precision,
    tc.data_scale,
    tc.nullable,
    tc.column_id,
    cc.comments AS column_comment
FROM user_tab_columns tc
LEFT JOIN user_col_comments cc
    ON tc.table_name = cc.table_name AND tc.column_name = cc.column_name
ORDER BY tc.table_name, tc.column_id;

-- 3. 主键和唯一约束
SELECT
    uc.table_name,
    uc.constraint_name,
    uc.constraint_type,
    ucc.column_name,
    ucc.position
FROM user_constraints uc
JOIN user_cons_columns ucc
    ON uc.constraint_name = ucc.constraint_name
WHERE uc.constraint_type IN ('P', 'U')
ORDER BY uc.table_name, uc.constraint_name, ucc.position;

-- 4. 外键关系
SELECT
    uc.table_name AS from_table,
    ucc.column_name AS from_column,
    rc.table_name AS to_table,
    rcc.column_name AS to_column,
    uc.constraint_name
FROM user_constraints uc
JOIN user_cons_columns ucc ON uc.constraint_name = ucc.constraint_name
JOIN user_constraints rc ON uc.r_constraint_name = rc.constraint_name
JOIN user_cons_columns rcc ON rc.constraint_name = rcc.constraint_name AND ucc.position = rcc.position
WHERE uc.constraint_type = 'R'
ORDER BY uc.table_name, ucc.position;

-- 5. 索引
SELECT
    ui.table_name,
    ui.index_name,
    ui.uniqueness,
    uic.column_name,
    uic.column_position
FROM user_indexes ui
JOIN user_ind_columns uic ON ui.index_name = uic.index_name
WHERE ui.table_name NOT LIKE 'BIN$%'
ORDER BY ui.table_name, ui.index_name, uic.column_position;

-- 6. 各表行数（大致，需先执行 ANALYZE 或查看统计信息）
SELECT table_name, num_rows, last_analyzed
FROM user_tables
WHERE num_rows > 0
ORDER BY num_rows DESC;
