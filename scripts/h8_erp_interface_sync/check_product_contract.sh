#!/usr/bin/env bash
# US-H8-002：验证开发 MSSQL 的完整商品与结构化包装契约。
set -euo pipefail

CONTAINER="${H8_MSSQL_CONTAINER:-wms-mssql-erp-if}"
SA_PASSWORD="${H8_MSSQL_SA_PASSWORD:-Wms_Erp_If_Dev_2026!}"

docker exec "${CONTAINER}" /opt/mssql-tools18/bin/sqlcmd \
  -S localhost -d wms_erp_if -U sa -P "${SA_PASSWORD}" -C -b -Q "
SET NOCOUNT ON;
DECLARE @required TABLE (name SYSNAME, nullable BIT);
INSERT INTO @required (name, nullable) VALUES
  (N'spec', 0), (N'special_drug_category', 0), (N'udi_code', 1),
  (N'electronic_regulatory_code', 1), (N'length_mm', 1),
  (N'width_mm', 1), (N'height_mm', 1), (N'volume_cm3', 1),
  (N'weight_g', 1), (N'packaging_json', 0), (N'schema_version', 0);
IF EXISTS (
  SELECT 1
    FROM @required r
    LEFT JOIN sys.columns c
      ON c.object_id = OBJECT_ID(N'dbo.if_in_product_master')
     AND c.name = r.name
   WHERE c.name IS NULL OR c.is_nullable <> r.nullable
)
  THROW 51000, N'if_in_product_master contract columns missing or nullable mismatch', 1;
IF NOT EXISTS (
  SELECT 1 FROM sys.check_constraints
   WHERE parent_object_id = OBJECT_ID(N'dbo.if_in_product_master')
     AND name = N'CK_if_in_pm_spec'
)
  THROW 51000, N'product spec non-blank check constraint missing', 1;
IF NOT EXISTS (
  SELECT 1 FROM sys.check_constraints
   WHERE parent_object_id = OBJECT_ID(N'dbo.if_in_product_master')
     AND name = N'CK_if_in_pm_packaging_json'
)
  THROW 51000, N'packaging JSON check constraint missing', 1;
IF NOT EXISTS (
  SELECT 1 FROM dbo.if_in_product_master
   WHERE external_doc_no = N'DEMO-PM-001'
     AND schema_version = N'1'
     AND special_drug_category = N'普通药品'
     AND ISJSON(packaging_json) = 1
     AND JSON_VALUE(packaging_json, '\$[1].unit') = N'盒'
     AND TRY_CONVERT(INT, JSON_VALUE(packaging_json, '\$[1].ratio_to_base')) = 24
)
  THROW 51000, N'DEMO-PM-001 complete product contract missing', 1;
"

echo "H8 product contract OK"
