# 现有 WMS 表结构分析

> 本目录只保留导出脚本和分析结论。Oracle 原始 CSV 导出含生产系统结构/规模信息，不进入 Git。

## 文件清单

| 文件 | 内容 | 来源 |
|------|------|------|
| `export-scripts.sql` | 6 个 Oracle 导出查询 | 手工维护 |
| `legacy-comparison-matrix.md` | 对比矩阵（分析产出） | 分析后生成 |

## 原始 CSV

`1.csv` 到 `6.csv` 为临时/外置产物，按 `.gitignore` 不入库：

- `1.csv`：所有表 + 表注释
- `2.csv`：所有字段（类型/长度/可空/注释）
- `3.csv`：主键 + 唯一约束
- `4.csv`：外键关系
- `5.csv`：索引
- `6.csv`：各表行数

## 使用方式

1. 在 Oracle 中执行 `export-scripts.sql` 中的 6 个查询
2. 每个查询结果导出为 CSV（UTF-8 编码）
3. 放入本目录临时分析，或放入受控证据归档
4. 生成/更新 `legacy-comparison-matrix.md`
5. 分析完成后删除 CSV，避免原始生产导出进入 Git
