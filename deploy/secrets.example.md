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
- `deploy/env/staging.env`: contains `WMS_STAGING_DB_PASSWORD`, `WMS_JWT_SECRET`,
  `WMS_HFILE_ACCESS_KEY`, `WMS_HFILE_SECRET_KEY` and optional API / MinIO ports.
  It must also contain an independent `WMS_H9_RENDER_TOKEN` shared only by the
  WMS API and H9 Render Worker. The H8 Rust Worker additionally requires
  `H8_CONNECTOR_ID`, a least-privilege `WMS_H8_WORKER_API_TOKEN`, and
  `WMS_H8_SECRET_ALIASES`. This map must contain separate keys for the connector
  Worker password alias and the H8-004 SELECT-only probe password alias; the API
  consumes only the probe alias and the Worker consumes only its transport alias.

For the Wave 1 H2 dev PostgreSQL environment, create:

```bash
mkdir -p deploy/secrets deploy/env
printf '%s' "$WMS_DEV_H2_DB_PASSWORD" > deploy/secrets/wms_dev_h2_db_password.txt
cp deploy/env/dev-h2.env.example deploy/env/dev-h2.env
```

Required dev H2 values:

- `deploy/secrets/wms_dev_h2_db_password.txt`: PostgreSQL password for `wms_dev_h2`.
- `deploy/env/dev-h2.env`: contains `WMS_DEV_H2_DB_PASSWORD`,
  `WMS_HFILE_ACCESS_KEY`, `WMS_HFILE_SECRET_KEY` and optional PostgreSQL / MinIO ports.
  It must also contain a development-only `WMS_H9_RENDER_TOKEN`.

Do not commit real secret files.
