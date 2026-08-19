-- US-M1-006 user creation requires a phone number owned by the auth user.

ALTER TABLE auth_users
    ADD COLUMN IF NOT EXISTS phone TEXT NOT NULL DEFAULT '';

GRANT SELECT, INSERT, UPDATE ON auth_users TO wms_app;
