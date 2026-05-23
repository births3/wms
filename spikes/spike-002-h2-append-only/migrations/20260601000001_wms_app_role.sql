-- 20260601000001_wms_app_role.sql
-- SPIKE-002 H1 增强：角色权限组合
--
-- 业务连接（应用 / API）必须用 wms_app 角色，仅有 INSERT 和 SELECT；
-- 即使绕过 trigger（如 DBA 临时禁用 trigger）也无法 UPDATE/DELETE。
-- 双重保护：trigger 是技术阻断，角色是权限阻断。

-- 角色（NOLOGIN：不能直接登录；通过 SET ROLE 切换或主连接 GRANT）
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'wms_app') THEN
        CREATE ROLE wms_app NOLOGIN;
    END IF;
END $$;

-- 仅授予 INSERT, SELECT
GRANT USAGE ON SCHEMA public TO wms_app;
GRANT INSERT, SELECT ON audit_event TO wms_app;
GRANT INSERT, SELECT ON ALL TABLES IN SCHEMA public TO wms_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO wms_app;

-- 显式不授予 UPDATE, DELETE, TRUNCATE
-- （PG 默认就不授予；写本注释作意图说明）
