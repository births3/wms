-- 示例种子：UUID 需替换为本地 WMS 货主/仓库/供应商/客户 ID 后再跑同步。
-- 占位 UUID 仅用于结构演示；sync.py 支持 --seed-demo 写入与环境一致的占位（仍须匹配 WMS 数据）。

USE wms_erp_if;
GO

-- 清空演示行（仅开发库）
DELETE FROM dbo.if_in_asn WHERE external_doc_no LIKE N'DEMO-%';
DELETE FROM dbo.if_in_outbound_order WHERE external_doc_no LIKE N'DEMO-%';
DELETE FROM dbo.if_in_product_master WHERE external_doc_no LIKE N'DEMO-%';
GO

-- 注意：下列 UUID 为占位，请用真实 WMS UUID 替换，或由 runbook 脚本注入。
DECLARE @owner UNIQUEIDENTIFIER = '00000000-0000-0000-0000-000000000001';
DECLARE @wh UNIQUEIDENTIFIER = '00000000-0000-0000-0000-000000000002';
DECLARE @sup UNIQUEIDENTIFIER = '00000000-0000-0000-0000-000000000003';
DECLARE @cust UNIQUEIDENTIFIER = '00000000-0000-0000-0000-000000000004';

INSERT INTO dbo.if_in_product_master (
    external_doc_no, owner_id, product_code, product_name, approval_no, spec,
    dosage_form, manufacturer, storage_condition, idempotency_key, sync_status
) VALUES (
    N'DEMO-PM-001', @owner, N'DEMO-P-001', N'演示商品-对乙酰氨基酚片', N'国药准字H000000', N'0.5g*24片',
    N'片剂', N'演示制药', N'normal', N'h8-demo-pm-001', N'pending'
);

INSERT INTO dbo.if_in_asn (
    external_doc_no, owner_id, warehouse_id, supplier_id, product_code, expected_qty,
    expected_arrival_at, document_type, external_ref, idempotency_key, sync_status
) VALUES (
    N'DEMO-ASN-001', @owner, @wh, @sup, N'DEMO-P-001', 100,
    DATEADD(day, 3, SYSUTCDATETIME()), N'purchase_inbound', N'ERP-PO-DEMO-001',
    N'h8-demo-asn-001', N'pending'
), (
    N'DEMO-ASN-002', @owner, @wh, @sup, N'DEMO-P-001', 20,
    DATEADD(day, 4, SYSUTCDATETIME()), N'purchase_inbound', N'ERP-PO-DEMO-002',
    N'h8-demo-asn-002', N'failed'
);

-- US-H8-002 入站闭环专用：时间置旧，H8_BATCH_SIZE=1 时不会消费 H8-004 的 pending 固定样本。
INSERT INTO dbo.if_in_asn (
    external_doc_no, owner_id, warehouse_id, supplier_id, product_code, expected_qty,
    expected_arrival_at, document_type, external_ref, idempotency_key, sync_status,
    created_at, updated_at
) VALUES (
    N'DEMO-ASN-FLOW-001', @owner,
    '00000000-0000-0000-0000-000000001301',
    '00000000-0000-0000-0000-000000001101', N'P-M1-E2E-001', 30,
    DATEADD(day, 5, SYSUTCDATETIME()), N'purchase_inbound', N'ERP-PO-DEMO-FLOW-001',
    N'h8-demo-asn-flow-001', N'pending', DATEADD(day, -10, SYSUTCDATETIME()),
    DATEADD(day, -10, SYSUTCDATETIME())
);

INSERT INTO dbo.if_in_outbound_order (
    external_doc_no, owner_id, warehouse_id, customer_id, document_type, erp_order_no,
    product_code, batch_no, planned_qty, required_ship_at, idempotency_key, sync_status
) VALUES (
    N'DEMO-OB-001', @owner, @wh, @cust, N'sales_outbound', N'ERP-SO-DEMO-001',
    N'DEMO-P-001', N'BATCH-DEMO-01', 10, DATEADD(day, 2, SYSUTCDATETIME()),
    N'h8-demo-ob-001', N'pending'
);
GO
