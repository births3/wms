-- H8 ERP 接口库（无 API 的 ERP 标准通道）
-- 库名：wms_erp_if
-- 控制列约定：sync_status / retry_count / last_error / idempotency_key

IF DB_ID(N'wms_erp_if') IS NULL
BEGIN
    CREATE DATABASE wms_erp_if;
END
GO

USE wms_erp_if;
GO

-- 通用：sync_status = pending | processing | success | failed | dead

IF OBJECT_ID(N'dbo.if_in_asn', N'U') IS NULL
BEGIN
    CREATE TABLE dbo.if_in_asn (
        id                UNIQUEIDENTIFIER NOT NULL CONSTRAINT PK_if_in_asn PRIMARY KEY DEFAULT NEWID(),
        external_doc_no   NVARCHAR(64)     NOT NULL,
        owner_id          UNIQUEIDENTIFIER NOT NULL,
        warehouse_id      UNIQUEIDENTIFIER NOT NULL,
        supplier_id       UNIQUEIDENTIFIER NOT NULL,
        product_code      NVARCHAR(64)     NOT NULL,
        expected_qty      BIGINT           NOT NULL,
        expected_arrival_at DATETIME2      NOT NULL,
        document_type     NVARCHAR(32)     NOT NULL CONSTRAINT DF_if_in_asn_doc DEFAULT N'purchase_inbound',
        external_ref      NVARCHAR(128)    NULL,
        receipt_no        NVARCHAR(64)     NULL, -- 可选；空则 WMS 侧 M-CG 生成
        payload_json      NVARCHAR(MAX)    NULL,
        sync_status       NVARCHAR(16)     NOT NULL CONSTRAINT DF_if_in_asn_st DEFAULT N'pending',
        retry_count       INT              NOT NULL CONSTRAINT DF_if_in_asn_rc DEFAULT 0,
        last_error        NVARCHAR(1000)   NULL,
        idempotency_key   NVARCHAR(128)    NOT NULL,
        wms_resource_id   NVARCHAR(64)     NULL,
        created_at        DATETIME2        NOT NULL CONSTRAINT DF_if_in_asn_ca DEFAULT SYSUTCDATETIME(),
        updated_at        DATETIME2        NOT NULL CONSTRAINT DF_if_in_asn_ua DEFAULT SYSUTCDATETIME(),
        CONSTRAINT UQ_if_in_asn_idem UNIQUE (idempotency_key),
        CONSTRAINT CK_if_in_asn_status CHECK (sync_status IN (N'pending', N'processing', N'success', N'failed', N'dead')),
        CONSTRAINT CK_if_in_asn_qty CHECK (expected_qty > 0)
    );
    CREATE INDEX IX_if_in_asn_poll ON dbo.if_in_asn (sync_status, updated_at);
END
GO

IF OBJECT_ID(N'dbo.if_in_outbound_order', N'U') IS NULL
BEGIN
    CREATE TABLE dbo.if_in_outbound_order (
        id                UNIQUEIDENTIFIER NOT NULL CONSTRAINT PK_if_in_outbound PRIMARY KEY DEFAULT NEWID(),
        external_doc_no   NVARCHAR(64)     NOT NULL,
        owner_id          UNIQUEIDENTIFIER NOT NULL,
        warehouse_id      UNIQUEIDENTIFIER NOT NULL,
        customer_id       UNIQUEIDENTIFIER NOT NULL,
        document_type     NVARCHAR(32)     NOT NULL CONSTRAINT DF_if_in_ob_doc DEFAULT N'sales_outbound',
        erp_order_no      NVARCHAR(64)     NULL,
        wms_order_no      NVARCHAR(64)     NULL,
        product_code      NVARCHAR(64)     NOT NULL,
        batch_no          NVARCHAR(64)     NULL,
        planned_qty       BIGINT           NOT NULL,
        required_ship_at  DATETIME2        NULL,
        payload_json      NVARCHAR(MAX)    NULL,
        sync_status       NVARCHAR(16)     NOT NULL CONSTRAINT DF_if_in_ob_st DEFAULT N'pending',
        retry_count       INT              NOT NULL CONSTRAINT DF_if_in_ob_rc DEFAULT 0,
        last_error        NVARCHAR(1000)   NULL,
        idempotency_key   NVARCHAR(128)    NOT NULL,
        wms_resource_id   NVARCHAR(64)     NULL,
        created_at        DATETIME2        NOT NULL CONSTRAINT DF_if_in_ob_ca DEFAULT SYSUTCDATETIME(),
        updated_at        DATETIME2        NOT NULL CONSTRAINT DF_if_in_ob_ua DEFAULT SYSUTCDATETIME(),
        CONSTRAINT UQ_if_in_ob_idem UNIQUE (idempotency_key),
        CONSTRAINT CK_if_in_ob_status CHECK (sync_status IN (N'pending', N'processing', N'success', N'failed', N'dead')),
        CONSTRAINT CK_if_in_ob_qty CHECK (planned_qty > 0)
    );
    CREATE INDEX IX_if_in_ob_poll ON dbo.if_in_outbound_order (sync_status, updated_at);
END
GO

IF OBJECT_ID(N'dbo.if_in_product_master', N'U') IS NULL
BEGIN
    CREATE TABLE dbo.if_in_product_master (
        id                UNIQUEIDENTIFIER NOT NULL CONSTRAINT PK_if_in_product PRIMARY KEY DEFAULT NEWID(),
        external_doc_no   NVARCHAR(64)     NOT NULL,
        owner_id          UNIQUEIDENTIFIER NOT NULL,
        product_code      NVARCHAR(64)     NOT NULL,
        product_name      NVARCHAR(256)    NOT NULL,
        approval_no       NVARCHAR(64)     NULL,
        spec              NVARCHAR(128)    NULL,
        dosage_form       NVARCHAR(64)     NULL,
        manufacturer      NVARCHAR(256)    NULL,
        storage_condition NVARCHAR(32)     NULL,
        payload_json      NVARCHAR(MAX)    NULL,
        sync_status       NVARCHAR(16)     NOT NULL CONSTRAINT DF_if_in_pm_st DEFAULT N'pending',
        retry_count       INT              NOT NULL CONSTRAINT DF_if_in_pm_rc DEFAULT 0,
        last_error        NVARCHAR(1000)   NULL,
        idempotency_key   NVARCHAR(128)    NOT NULL,
        wms_resource_id   NVARCHAR(64)     NULL,
        created_at        DATETIME2        NOT NULL CONSTRAINT DF_if_in_pm_ca DEFAULT SYSUTCDATETIME(),
        updated_at        DATETIME2        NOT NULL CONSTRAINT DF_if_in_pm_ua DEFAULT SYSUTCDATETIME(),
        CONSTRAINT UQ_if_in_pm_idem UNIQUE (idempotency_key),
        CONSTRAINT CK_if_in_pm_status CHECK (sync_status IN (N'pending', N'processing', N'success', N'failed', N'dead'))
    );
    CREATE INDEX IX_if_in_pm_poll ON dbo.if_in_product_master (sync_status, updated_at);
END
GO
