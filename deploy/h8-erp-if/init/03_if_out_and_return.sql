-- H8 出站接口表 + ERP→WMS 退货入站表（补全通道 B）
USE wms_erp_if;
GO

-- ---------------------------------------------------------------------------
-- WMS → ERP：统一出站消息（ERP 作业认领 pending → 处理 → acked）
-- 来源：WMS PostgreSQL *erp_feedback_outbox，由 H8 worker 投递
-- ---------------------------------------------------------------------------
IF OBJECT_ID(N'dbo.if_out_message', N'U') IS NULL
BEGIN
    CREATE TABLE dbo.if_out_message (
        id                   UNIQUEIDENTIFIER NOT NULL CONSTRAINT PK_if_out_message PRIMARY KEY DEFAULT NEWID(),
        event_type           NVARCHAR(64)     NOT NULL,
        owner_id             UNIQUEIDENTIFIER NOT NULL,
        source_outbox_table  NVARCHAR(128)    NOT NULL,
        source_outbox_id     NVARCHAR(64)     NOT NULL,
        external_ref         NVARCHAR(128)    NULL,
        payload_json         NVARCHAR(MAX)    NOT NULL,
        sync_status          NVARCHAR(16)     NOT NULL CONSTRAINT DF_if_out_st DEFAULT N'pending',
        retry_count          INT              NOT NULL CONSTRAINT DF_if_out_rc DEFAULT 0,
        last_error           NVARCHAR(1000)   NULL,
        idempotency_key      NVARCHAR(160)    NOT NULL,
        erp_ack_ref          NVARCHAR(128)    NULL,
        created_at           DATETIME2        NOT NULL CONSTRAINT DF_if_out_ca DEFAULT SYSUTCDATETIME(),
        updated_at           DATETIME2        NOT NULL CONSTRAINT DF_if_out_ua DEFAULT SYSUTCDATETIME(),
        CONSTRAINT UQ_if_out_idem UNIQUE (idempotency_key),
        CONSTRAINT UQ_if_out_source UNIQUE (source_outbox_table, source_outbox_id),
        CONSTRAINT CK_if_out_status CHECK (
            sync_status IN (N'pending', N'processing', N'success', N'failed', N'dead', N'acked')
        )
    );
    CREATE INDEX IX_if_out_poll ON dbo.if_out_message (sync_status, updated_at);
    CREATE INDEX IX_if_out_event ON dbo.if_out_message (event_type, created_at DESC);
END
GO

-- ---------------------------------------------------------------------------
-- ERP → WMS：销退入库预报（document_type=sales_return → 收货单）
-- ---------------------------------------------------------------------------
IF OBJECT_ID(N'dbo.if_in_return_order', N'U') IS NULL
BEGIN
    CREATE TABLE dbo.if_in_return_order (
        id                  UNIQUEIDENTIFIER NOT NULL CONSTRAINT PK_if_in_return PRIMARY KEY DEFAULT NEWID(),
        external_doc_no     NVARCHAR(64)     NOT NULL,
        owner_id            UNIQUEIDENTIFIER NOT NULL,
        warehouse_id        UNIQUEIDENTIFIER NOT NULL,
        customer_id         UNIQUEIDENTIFIER NOT NULL,
        -- 收货单 OpenAPI 仍要求 supplier_id（销退客户方在 payload/customer_id）
        supplier_id         UNIQUEIDENTIFIER NOT NULL,
        product_code        NVARCHAR(64)     NOT NULL,
        expected_qty        BIGINT           NOT NULL,
        expected_arrival_at DATETIME2        NOT NULL,
        document_type       NVARCHAR(32)     NOT NULL CONSTRAINT DF_if_in_ret_doc DEFAULT N'sales_return',
        external_ref        NVARCHAR(128)    NULL,
        receipt_no          NVARCHAR(64)     NULL,
        batch_no            NVARCHAR(64)     NOT NULL, -- sales_return 业务要求原批号
        payload_json        NVARCHAR(MAX)    NULL,
        sync_status         NVARCHAR(16)     NOT NULL CONSTRAINT DF_if_in_ret_st DEFAULT N'pending',
        retry_count         INT              NOT NULL CONSTRAINT DF_if_in_ret_rc DEFAULT 0,
        last_error          NVARCHAR(1000)   NULL,
        idempotency_key     NVARCHAR(128)    NOT NULL,
        wms_resource_id     NVARCHAR(64)     NULL,
        created_at          DATETIME2        NOT NULL CONSTRAINT DF_if_in_ret_ca DEFAULT SYSUTCDATETIME(),
        updated_at          DATETIME2        NOT NULL CONSTRAINT DF_if_in_ret_ua DEFAULT SYSUTCDATETIME(),
        CONSTRAINT UQ_if_in_ret_idem UNIQUE (idempotency_key),
        CONSTRAINT CK_if_in_ret_status CHECK (
            sync_status IN (N'pending', N'processing', N'success', N'failed', N'dead')
        ),
        CONSTRAINT CK_if_in_ret_qty CHECK (expected_qty > 0)
    );
    CREATE INDEX IX_if_in_ret_poll ON dbo.if_in_return_order (sync_status, updated_at);
END
GO

-- 已有库补列（幂等）
IF OBJECT_ID(N'dbo.if_in_return_order', N'U') IS NOT NULL
   AND COL_LENGTH(N'dbo.if_in_return_order', N'supplier_id') IS NULL
BEGIN
    ALTER TABLE dbo.if_in_return_order ADD supplier_id UNIQUEIDENTIFIER NULL;
END
GO

