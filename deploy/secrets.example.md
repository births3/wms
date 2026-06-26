# Deployment Secrets

The real `deploy/secrets/` directory is intentionally ignored by git.

For the docker-compose staging environment, create the following files on the staging host:

```bash
mkdir -p deploy/secrets deploy/env
printf '%s' "$WMS_STAGING_DB_PASSWORD" > deploy/secrets/wms_staging_db_password.txt
cp deploy/env/staging.env.example deploy/env/staging.env
```

Required runtime values:

- `deploy/secrets/wms_staging_db_password.txt`: PostgreSQL password for `wms_staging`.
- `deploy/env/staging.env`: contains `WMS_STAGING_DB_PASSWORD`, `WMS_JWT_SECRET`, and optional `WMS_STAGING_API_PORT`.

For the Wave 1 H2 dev PostgreSQL environment, create:

```bash
mkdir -p deploy/secrets deploy/env
printf '%s' "$WMS_DEV_H2_DB_PASSWORD" > deploy/secrets/wms_dev_h2_db_password.txt
cp deploy/env/dev-h2.env.example deploy/env/dev-h2.env
```

Required dev H2 values:

- `deploy/secrets/wms_dev_h2_db_password.txt`: PostgreSQL password for `wms_dev_h2`.
- `deploy/env/dev-h2.env`: contains `WMS_DEV_H2_DB_PASSWORD` and optional `WMS_DEV_H2_DB_PORT`.

Do not commit real secret files.
