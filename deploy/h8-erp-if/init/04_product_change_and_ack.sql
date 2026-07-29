-- H8：ERP→WMS 商品档案变更回写 + 出站确认辅助
USE wms_erp_if;
GO

-- 档案补录响应 / 商品主数据变更（ERP 处理完补录后回写）
IF OBJECT_ID(N'dbo.if_in_product_change', N'U') IS NULL
BEGIN
    CREATE TABLE dbo.if_in_product_change (
        id                UNIQUEIDENTIFIER NOT NULL CONSTRAINT PK_if_in_pc PRIMARY KEY DEFAULT NEWID(),
        external_doc_no   NVARCHAR(64)     NOT NULL,
        owner_id          UNIQUEIDENTIFIER NOT NULL,
        product_code      NVARCHAR(64)     NOT NULL,
        product_id        UNIQUEIDENTIFIER NULL, -- 可选，优先于 code 解析
        field_name        NVARCHAR(64)     NOT NULL,
        new_value         NVARCHAR(MAX)    NOT NULL,
        liaison_id        UNIQUEIDENTIFIER NULL,
        asn_id            UNIQUEIDENTIFIER NULL,
        schema_version    NVARCHAR(16)     NOT NULL CONSTRAINT DF_if_in_pc_sv DEFAULT N'1',
        payload_json      NVARCHAR(MAX)    NULL,
        sync_status       NVARCHAR(16)     NOT NULL CONSTRAINT DF_if_in_pc_st DEFAULT N'pending',
        retry_count       INT              NOT NULL CONSTRAINT DF_if_in_pc_rc DEFAULT 0,
        last_error        NVARCHAR(1000)   NULL,
        idempotency_key   NVARCHAR(128)    NOT NULL,
        wms_resource_id   NVARCHAR(64)     NULL,
        created_at        DATETIME2        NOT NULL CONSTRAINT DF_if_in_pc_ca DEFAULT SYSUTCDATETIME(),
        updated_at        DATETIME2        NOT NULL CONSTRAINT DF_if_in_pc_ua DEFAULT SYSUTCDATETIME(),
        CONSTRAINT UQ_if_in_pc_idem UNIQUE (idempotency_key),
        CONSTRAINT CK_if_in_pc_status CHECK (
            sync_status IN (N'pending', N'processing', N'success', N'failed', N'dead')
        )
    );
    CREATE INDEX IX_if_in_pc_poll ON dbo.if_in_product_change (sync_status, updated_at);
END
GO
