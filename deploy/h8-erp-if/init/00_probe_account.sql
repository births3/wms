-- US-H8-004 软件路径：使用 sqlcmd 变量创建与 Worker 分离的 SELECT-only 账号。
-- wait-and-init.sh 通过 H8_MSSQL_PROBE_USER / H8_MSSQL_PROBE_PASSWORD 注入变量。
SET NOCOUNT ON;
USE wms_erp_if;
GO

DECLARE @probe_user sysname = N'$(PROBE_USER)';
DECLARE @probe_password nvarchar(128) = N'$(PROBE_PASSWORD)';
DECLARE @sql nvarchar(max);

IF NOT EXISTS (SELECT 1 FROM sys.sql_logins WHERE name = @probe_user)
BEGIN
    SET @sql = N'CREATE LOGIN ' + QUOTENAME(@probe_user)
        + N' WITH PASSWORD = ' + QUOTENAME(@probe_password, '''')
        + N', CHECK_POLICY = OFF, CHECK_EXPIRATION = OFF';
    EXEC sp_executesql @sql;
END
ELSE
BEGIN
    SET @sql = N'ALTER LOGIN ' + QUOTENAME(@probe_user)
        + N' WITH PASSWORD = ' + QUOTENAME(@probe_password, '''');
    EXEC sp_executesql @sql;
END;

IF NOT EXISTS (SELECT 1 FROM sys.database_principals WHERE name = @probe_user)
BEGIN
    SET @sql = N'CREATE USER ' + QUOTENAME(@probe_user) + N' FOR LOGIN ' + QUOTENAME(@probe_user);
    EXEC sp_executesql @sql;
END;

SET @sql = N'GRANT SELECT ON SCHEMA::dbo TO ' + QUOTENAME(@probe_user)
    + N'; DENY INSERT, UPDATE, DELETE ON SCHEMA::dbo TO ' + QUOTENAME(@probe_user);
EXEC sp_executesql @sql;
