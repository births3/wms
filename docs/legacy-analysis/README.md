# 现有 WMS 表结构分析

> 本目录存放现有 Oracle WMS 系统的表结构导出和对比分析。

## 文件清单

| 文件 | 内容 | 来源 |
|------|------|------|
| `1_tables.csv` | 所有表 + 表注释 | Oracle 导出 |
| `2_columns.csv` | 所有字段（类型/长度/可空/注释） | Oracle 导出 |
| `3_constraints.csv` | 主键 + 唯一约束 | Oracle 导出 |
| `4_foreign_keys.csv` | 外键关系 | Oracle 导出 |
| `5_indexes.csv` | 索引 | Oracle 导出 |
| `6_row_counts.csv` | 各表行数 | Oracle 导出 |
| `legacy-comparison-matrix.md` | 对比矩阵（分析产出） | 分析后生成 |

## 导出 SQL

见 `export-scripts.sql`

## 使用方式

1. 在 Oracle 中执行 `export-scripts.sql` 中的 6 个查询
2. 每个查询结果导出为 CSV（UTF-8 编码）
3. 放入本目录
4. 通知 AI 助手进行对比分析
