# OpenAPI curl 示例

> 本文件由 `scripts/governance/generate_openapi_curl_examples.py` 根据 `shared/openapi/openapi.json` 生成；不要手工编辑。

使用前设置：

```bash
export WMS_API_BASE=http://127.0.0.1:9002
export WMS_TOKEN=<从 /api/v1/auth/login 获取的 access_token>
```

## GET /api-docs

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api-docs"
```

## GET /api/v1/admin/menus/draft

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/admin/menus/draft" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/admin/menus/draft/batch-enable

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/admin/menus/draft/batch-enable" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/admin/menus/draft/nodes

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/admin/menus/draft/nodes" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## PATCH /api/v1/admin/menus/draft/nodes/{id}

```bash
curl -sS \
  -X PATCH \
  "$WMS_API_BASE/api/v1/admin/menus/draft/nodes/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/admin/menus/publish

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/admin/menus/publish" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/admin/menus/published

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/admin/menus/published" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/admin/menus/rollback

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/admin/menus/rollback" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/alert-definitions

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/alert-definitions" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/alert-definitions/change-requests

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/alert-definitions/change-requests" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/alert-definitions/{id}

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/alert-definitions/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/alert-escalation-rules

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/alert-escalation-rules" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## PUT /api/v1/alert-escalation-rules/{rule_code}

```bash
curl -sS \
  -X PUT \
  "$WMS_API_BASE/api/v1/alert-escalation-rules/<rule_code>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/alerts

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/alerts" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/alerts/active

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/alerts/active" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/alerts/changes

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/alerts/changes" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/alerts/exports

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/alerts/exports" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/alerts/exports/{id}

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/alerts/exports/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/alerts/exports/{token}/download

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/alerts/exports/<token>/download" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/alerts/gsp-report

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/alerts/gsp-report" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/alerts/statistics

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/alerts/statistics" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/alerts/{id}

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/alerts/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/alerts/{id}/acknowledge

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/alerts/<id>/acknowledge" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/alerts/{id}/close

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/alerts/<id>/close" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/alerts/{id}/handling

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/alerts/<id>/handling" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/alerts/{id}/ignore

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/alerts/<id>/ignore" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/audit/archive/partitions

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/audit/archive/partitions" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/audit/archive/runs

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/audit/archive/runs" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/audit/events

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/audit/events" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/audit/events/export

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/audit/events/export" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/auth/api-keys

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/auth/api-keys" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/auth/api-keys

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/auth/api-keys" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/auth/api-keys/{api_key_id}/revoke

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/auth/api-keys/<api_key_id>/revoke" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/auth/api-keys/{api_key_id}/rotate

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/auth/api-keys/<api_key_id>/rotate" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/auth/login

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/auth/login" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/auth/logout

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/auth/logout" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/auth/me

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/auth/me" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## PUT /api/v1/auth/me/password

```bash
curl -sS \
  -X PUT \
  "$WMS_API_BASE/api/v1/auth/me/password" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/auth/permissions

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/auth/permissions" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/auth/roles

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/auth/roles" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/auth/roles

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/auth/roles" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## DELETE /api/v1/auth/roles/{role_id}

```bash
curl -sS \
  -X DELETE \
  "$WMS_API_BASE/api/v1/auth/roles/<role_id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## PUT /api/v1/auth/roles/{role_id}

```bash
curl -sS \
  -X PUT \
  "$WMS_API_BASE/api/v1/auth/roles/<role_id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## PUT /api/v1/auth/roles/{role_id}/permissions

```bash
curl -sS \
  -X PUT \
  "$WMS_API_BASE/api/v1/auth/roles/<role_id>/permissions" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/auth/sessions

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/auth/sessions" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/auth/sessions/revoke-others

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/auth/sessions/revoke-others" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/auth/sessions/{session_id}/revoke

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/auth/sessions/<session_id>/revoke" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## PUT /api/v1/auth/user-roles/batch

```bash
curl -sS \
  -X PUT \
  "$WMS_API_BASE/api/v1/auth/user-roles/batch" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/auth/users

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/auth/users" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/auth/users

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/auth/users" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/auth/users/{user_id}/kick

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/auth/users/<user_id>/kick" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## PUT /api/v1/auth/users/{user_id}/status

```bash
curl -sS \
  -X PUT \
  "$WMS_API_BASE/api/v1/auth/users/<user_id>/status" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/billing/accounts

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/billing/accounts" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/billing/charges/calculate

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/billing/charges/calculate" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/billing/contracts

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/billing/contracts" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/billing/rules

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/billing/rules" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/billing/statements

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/billing/statements" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/billing/statements/{id}/confirm

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/billing/statements/<id>/confirm" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/business-retention/jobs

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/business-retention/jobs" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/business-retention/policies

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/business-retention/policies" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/code-generator/document-number-allocations

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/code-generator/document-number-allocations" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/code-generator/document-number-rules

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/code-generator/document-number-rules" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## PUT /api/v1/code-generator/document-number-rules/{rule_code}

```bash
curl -sS \
  -X PUT \
  "$WMS_API_BASE/api/v1/code-generator/document-number-rules/<rule_code>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## PATCH /api/v1/code-generator/document-number-rules/{rule_code}/enabled

```bash
curl -sS \
  -X PATCH \
  "$WMS_API_BASE/api/v1/code-generator/document-number-rules/<rule_code>/enabled" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/cold-chain/devices

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/cold-chain/devices" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/cold-chain/devices

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/cold-chain/devices" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## PATCH /api/v1/cold-chain/devices/{device_code}

```bash
curl -sS \
  -X PATCH \
  "$WMS_API_BASE/api/v1/cold-chain/devices/<device_code>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/cold-chain/devices/{device_code}/disable

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/cold-chain/devices/<device_code>/disable" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/cold-chain/excursions

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/cold-chain/excursions" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/cold-chain/excursions/pending-disposition

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/cold-chain/excursions/pending-disposition" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/cold-chain/excursions/{external_event_id}/dispose

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/cold-chain/excursions/<external_event_id>/dispose" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/cold-chain/readings

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/cold-chain/readings" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/config-center/feature-flags/archive-file-source

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/config-center/feature-flags/archive-file-source" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/config-center/feature-flags/export

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/config-center/feature-flags/export" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/config-center/feature-flags/import

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/config-center/feature-flags/import" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/config-center/feature-flags/migrate

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/config-center/feature-flags/migrate" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/config-center/feature-flags/reconcile

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/config-center/feature-flags/reconcile" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/config-center/feature-flags/source

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/config-center/feature-flags/source" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/dock-appointments

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/dock-appointments" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/dock-appointments

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/dock-appointments" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## PATCH /api/v1/dock-appointments/{id}

```bash
curl -sS \
  -X PATCH \
  "$WMS_API_BASE/api/v1/dock-appointments/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/dock-appointments/{id}/arrive

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/dock-appointments/<id>/arrive" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/dock-appointments/{id}/cancel

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/dock-appointments/<id>/cancel" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/docks

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/docks" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/docks

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/docks" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/docks/import

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/docks/import" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## DELETE /api/v1/docks/{id}

```bash
curl -sS \
  -X DELETE \
  "$WMS_API_BASE/api/v1/docks/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## PATCH /api/v1/docks/{id}

```bash
curl -sS \
  -X PATCH \
  "$WMS_API_BASE/api/v1/docks/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/driver/tasks/today

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/driver/tasks/today" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/drug-inspection/platforms

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/drug-inspection/platforms" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/drug-inspection/platforms

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/drug-inspection/platforms" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## PATCH /api/v1/drug-inspection/platforms/{platform_id}/status

```bash
curl -sS \
  -X PATCH \
  "$WMS_API_BASE/api/v1/drug-inspection/platforms/<platform_id>/status" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/event-bus/deliveries/pending

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/event-bus/deliveries/pending" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/event-bus/deliveries/{delivery_id}/ack

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/event-bus/deliveries/<delivery_id>/ack" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/event-bus/deliveries/{delivery_id}/nack

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/event-bus/deliveries/<delivery_id>/nack" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/express/carriers

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/express/carriers" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/express/carriers

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/express/carriers" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/express/routing-rules

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/express/routing-rules" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/express/routing-rules

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/express/routing-rules" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/express/waybills

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/express/waybills" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/express/waybills/{waybill_no}/cancel

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/express/waybills/<waybill_no>/cancel" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/express/waybills/{waybill_no}/tracking

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/express/waybills/<waybill_no>/tracking" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/healthz

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/healthz"
```

## GET /api/v1/inbound/receiving-dashboard

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/inbound/receiving-dashboard" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/inbound/receiving-orders

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/inbound/receiving-orders" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/inbound/receiving-orders

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/inbound/receiving-orders" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## DELETE /api/v1/inbound/receiving-orders/{id}

```bash
curl -sS \
  -X DELETE \
  "$WMS_API_BASE/api/v1/inbound/receiving-orders/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/inbound/receiving-orders/{id}

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/inbound/receiving-orders/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## PATCH /api/v1/inbound/receiving-orders/{id}

```bash
curl -sS \
  -X PATCH \
  "$WMS_API_BASE/api/v1/inbound/receiving-orders/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/inbound/receiving-orders/{id}/inspect

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/inbound/receiving-orders/<id>/inspect" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/inbound/receiving-orders/{id}/print-data

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/inbound/receiving-orders/<id>/print-data" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/inbound/receiving-orders/{id}/putaway

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/inbound/receiving-orders/<id>/putaway" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/inbound/receiving-orders/{id}/putaway-recommendations

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/inbound/receiving-orders/<id>/putaway-recommendations" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/inbound/receiving-orders/{id}/receive

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/inbound/receiving-orders/<id>/receive" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/inbound/receiving-orders/{id}/reject

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/inbound/receiving-orders/<id>/reject" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/inbound/receiving-orders/{id}/release

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/inbound/receiving-orders/<id>/release" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/inbound/receiving-orders/{id}/sign

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/inbound/receiving-orders/<id>/sign" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/inventory/batches

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/inventory/batches" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/inventory/batches/expire

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/inventory/batches/expire" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/inventory/batches/near-expiry-report

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/inventory/batches/near-expiry-report" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/inventory/batches/putaway

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/inventory/batches/putaway" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/inventory/batches/recall

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/inventory/batches/recall" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/inventory/batches/recall/cancel

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/inventory/batches/recall/cancel" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/inventory/batches/status

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/inventory/batches/status" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/inventory/batches/{id}/trace

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/inventory/batches/<id>/trace" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/inventory/counts

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/inventory/counts" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/inventory/counts/{id}

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/inventory/counts/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/inventory/counts/{id}/approve

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/inventory/counts/<id>/approve" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/inventory/counts/{id}/lines/{line_id}/submit

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/inventory/counts/<id>/lines/<line_id>/submit" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/inventory/maintenance/records

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/inventory/maintenance/records" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/inventory/maintenance/records

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/inventory/maintenance/records" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/inventory/maintenance/tasks

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/inventory/maintenance/tasks" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/inventory/status-transitions

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/inventory/status-transitions" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## PUT /api/v1/inventory/status-transitions/{from_status}/{to_status}

```bash
curl -sS \
  -X PUT \
  "$WMS_API_BASE/api/v1/inventory/status-transitions/<from_status>/<to_status>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/m-vr/dual-person-policy

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/m-vr/dual-person-policy" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/m-vr/dual-person-policy/rules

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/m-vr/dual-person-policy/rules" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## PUT /api/v1/m-vr/dual-person-policy/rules

```bash
curl -sS \
  -X PUT \
  "$WMS_API_BASE/api/v1/m-vr/dual-person-policy/rules" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/master-data/customers

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/master-data/customers" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/master-data/customers

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/master-data/customers" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/master-data/customers/batch-sync

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/master-data/customers/batch-sync" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/master-data/customers/{customer_id}/addresses

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/master-data/customers/<customer_id>/addresses" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/master-data/customers/{customer_id}/addresses

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/master-data/customers/<customer_id>/addresses" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## PATCH /api/v1/master-data/customers/{customer_id}/addresses/{address_id}

```bash
curl -sS \
  -X PATCH \
  "$WMS_API_BASE/api/v1/master-data/customers/<customer_id>/addresses/<address_id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/master-data/customers/{customer_id}/profile

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/master-data/customers/<customer_id>/profile" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## PATCH /api/v1/master-data/customers/{customer_id}/profile

```bash
curl -sS \
  -X PATCH \
  "$WMS_API_BASE/api/v1/master-data/customers/<customer_id>/profile" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## DELETE /api/v1/master-data/customers/{id}

```bash
curl -sS \
  -X DELETE \
  "$WMS_API_BASE/api/v1/master-data/customers/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## PATCH /api/v1/master-data/customers/{id}

```bash
curl -sS \
  -X PATCH \
  "$WMS_API_BASE/api/v1/master-data/customers/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/master-data/locations

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/master-data/locations" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/master-data/locations

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/master-data/locations" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/master-data/locations/batch-create

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/master-data/locations/batch-create" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## DELETE /api/v1/master-data/locations/{id}

```bash
curl -sS \
  -X DELETE \
  "$WMS_API_BASE/api/v1/master-data/locations/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## PATCH /api/v1/master-data/locations/{id}

```bash
curl -sS \
  -X PATCH \
  "$WMS_API_BASE/api/v1/master-data/locations/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/master-data/products

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/master-data/products" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/master-data/products

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/master-data/products" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/master-data/products/batch-sync

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/master-data/products/batch-sync" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## DELETE /api/v1/master-data/products/{id}

```bash
curl -sS \
  -X DELETE \
  "$WMS_API_BASE/api/v1/master-data/products/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/master-data/products/{id}

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/master-data/products/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## PATCH /api/v1/master-data/products/{id}

```bash
curl -sS \
  -X PATCH \
  "$WMS_API_BASE/api/v1/master-data/products/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/master-data/special-drug-categories

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/master-data/special-drug-categories" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/master-data/special-drug-categories

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/master-data/special-drug-categories" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## DELETE /api/v1/master-data/special-drug-categories/{id}

```bash
curl -sS \
  -X DELETE \
  "$WMS_API_BASE/api/v1/master-data/special-drug-categories/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## PATCH /api/v1/master-data/special-drug-categories/{id}

```bash
curl -sS \
  -X PATCH \
  "$WMS_API_BASE/api/v1/master-data/special-drug-categories/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/master-data/suppliers

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/master-data/suppliers" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/master-data/suppliers

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/master-data/suppliers" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/master-data/suppliers/batch-sync

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/master-data/suppliers/batch-sync" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## DELETE /api/v1/master-data/suppliers/{id}

```bash
curl -sS \
  -X DELETE \
  "$WMS_API_BASE/api/v1/master-data/suppliers/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## PATCH /api/v1/master-data/suppliers/{id}

```bash
curl -sS \
  -X PATCH \
  "$WMS_API_BASE/api/v1/master-data/suppliers/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/master-data/warehouse-zones

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/master-data/warehouse-zones" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/master-data/warehouse-zones

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/master-data/warehouse-zones" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## DELETE /api/v1/master-data/warehouse-zones/{id}

```bash
curl -sS \
  -X DELETE \
  "$WMS_API_BASE/api/v1/master-data/warehouse-zones/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## PATCH /api/v1/master-data/warehouse-zones/{id}

```bash
curl -sS \
  -X PATCH \
  "$WMS_API_BASE/api/v1/master-data/warehouse-zones/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/master-data/warehouses

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/master-data/warehouses" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/master-data/warehouses

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/master-data/warehouses" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## DELETE /api/v1/master-data/warehouses/{id}

```bash
curl -sS \
  -X DELETE \
  "$WMS_API_BASE/api/v1/master-data/warehouses/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## PATCH /api/v1/master-data/warehouses/{id}

```bash
curl -sS \
  -X PATCH \
  "$WMS_API_BASE/api/v1/master-data/warehouses/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/outbound/orders

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/outbound/orders" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/outbound/orders

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/outbound/orders" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/outbound/orders/{id}

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/outbound/orders/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/outbound/orders/{id}/review

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/outbound/orders/<id>/review" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/outbound/orders/{id}/review

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/outbound/orders/<id>/review" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/outbound/orders/{id}/ship

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/outbound/orders/<id>/ship" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/outbound/pick-tasks/{id}/complete

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/outbound/pick-tasks/<id>/complete" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/outbound/waves

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/outbound/waves" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/outbound/waves

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/outbound/waves" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/outbound/waves/{wave_id}

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/outbound/waves/<wave_id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/outbound/waves/{wave_id}/cancel

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/outbound/waves/<wave_id>/cancel" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/packing/jobs

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/packing/jobs" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/packing/jobs/{id}/waybill

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/packing/jobs/<id>/waybill" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/packing/jobs/{id}/weigh

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/packing/jobs/<id>/weigh" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/packing/stations

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/packing/stations" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/parameter-mapping/execute

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/parameter-mapping/execute" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/parameter-mapping/traces/{execution_id}

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/parameter-mapping/traces/<execution_id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/print-templates/field-libraries

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/print-templates/field-libraries" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/print-templates/field-libraries/{version_id}/fields

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/print-templates/field-libraries/<version_id>/fields" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/print-templates/preview

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/print-templates/preview" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/print-templates/print

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/print-templates/print" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/print-templates/resolve

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/print-templates/resolve" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/print-templates/templates

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/print-templates/templates" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/print-templates/templates

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/print-templates/templates" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/print-templates/templates/{template_id}/versions

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/print-templates/templates/<template_id>/versions" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/quality-liaisons

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/quality-liaisons" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## PUT /api/v1/quality-liaisons/types/{type_code}

```bash
curl -sS \
  -X PUT \
  "$WMS_API_BASE/api/v1/quality-liaisons/types/<type_code>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/quality-liaisons/{id}

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/quality-liaisons/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/quality-liaisons/{id}/approval-callback

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/quality-liaisons/<id>/approval-callback" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/reports/gsp/inbound-ledger

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/reports/gsp/inbound-ledger" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/reports/gsp/inventory-ledger

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/reports/gsp/inventory-ledger" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/reports/gsp/outbound-ledger

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/reports/gsp/outbound-ledger" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/reports/query

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/reports/query" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/resilience/status

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/resilience/status"
```

## POST /api/v1/retail/crossdock-plans

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/retail/crossdock-plans" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/retail/replenishment-suggestions

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/retail/replenishment-suggestions" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/state-machines

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/state-machines" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/state-machines/{machine_code}

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/state-machines/<machine_code>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/state-machines/{machine_code}/transition-validation

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/state-machines/<machine_code>/transition-validation" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/stock-adjustments/loss-orders

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/stock-adjustments/loss-orders" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/stock-adjustments/loss-orders/{id}

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/stock-adjustments/loss-orders/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/stock-adjustments/loss-orders/{id}/execute

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/stock-adjustments/loss-orders/<id>/execute" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/stock-adjustments/loss-orders/{id}/quality-approval

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/stock-adjustments/loss-orders/<id>/quality-approval" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/stock-adjustments/loss-orders/{id}/start

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/stock-adjustments/loss-orders/<id>/start" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/stock-adjustments/surplus-orders

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/stock-adjustments/surplus-orders" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/stock-adjustments/surplus-orders/{id}

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/stock-adjustments/surplus-orders/<id>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/stock-adjustments/surplus-orders/{id}/execute

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/stock-adjustments/surplus-orders/<id>/execute" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/stock-adjustments/surplus-orders/{id}/quality-approval

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/stock-adjustments/surplus-orders/<id>/quality-approval" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/stock-adjustments/surplus-orders/{id}/start

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/stock-adjustments/surplus-orders/<id>/start" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/store/dashboard

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/store/dashboard" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/system-dictionaries/{dict_code}/items

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/system-dictionaries/<dict_code>/items" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## PUT /api/v1/system-dictionaries/{dict_code}/items/{item_code}

```bash
curl -sS \
  -X PUT \
  "$WMS_API_BASE/api/v1/system-dictionaries/<dict_code>/items/<item_code>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## PATCH /api/v1/system-dictionaries/{dict_code}/items/{item_code}/disable

```bash
curl -sS \
  -X PATCH \
  "$WMS_API_BASE/api/v1/system-dictionaries/<dict_code>/items/<item_code>/disable" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/system-dictionaries/{dict_code}/items/{item_code}/impact-preview

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/system-dictionaries/<dict_code>/items/<item_code>/impact-preview" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## GET /api/v1/task-engine/priority-rule

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/task-engine/priority-rule" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## PUT /api/v1/task-engine/priority-rule

```bash
curl -sS \
  -X PUT \
  "$WMS_API_BASE/api/v1/task-engine/priority-rule" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/task-engine/task-groups

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/task-engine/task-groups" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## PUT /api/v1/task-engine/task-groups/{task_group_code}

```bash
curl -sS \
  -X PUT \
  "$WMS_API_BASE/api/v1/task-engine/task-groups/<task_group_code>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/task-engine/task-types

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/task-engine/task-types" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## PUT /api/v1/task-engine/task-types/{task_type_code}

```bash
curl -sS \
  -X PUT \
  "$WMS_API_BASE/api/v1/task-engine/task-types/<task_type_code>" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## PATCH /api/v1/task-engine/task-types/{task_type_code}/enabled

```bash
curl -sS \
  -X PATCH \
  "$WMS_API_BASE/api/v1/task-engine/task-types/<task_type_code>/enabled" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/task-engine/tasks

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/task-engine/tasks" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/task-engine/tasks

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/task-engine/tasks" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/task-engine/tasks/{task_id}/transitions

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/task-engine/tasks/<task_id>/transitions" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/task-engine/workers

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/task-engine/workers" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/tms/container-recoveries

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/tms/container-recoveries" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/tms/dispatches

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/tms/dispatches" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/tms/route-plans

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/tms/route-plans" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/tms/transit-temperature-readings

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/tms/transit-temperature-readings" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/traceability/outbound-reports

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/traceability/outbound-reports" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/wechat-notify/approvals

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/wechat-notify/approvals" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/wechat-notify/approvals/{approval_id}/callback

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/wechat-notify/approvals/<approval_id>/callback" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/wechat-notify/configs

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/wechat-notify/configs" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/wechat-notify/configs

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/wechat-notify/configs" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/wechat-notify/records

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/wechat-notify/records" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/wechat-notify/records/{record_id}/resend

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/wechat-notify/records/<record_id>/resend" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/wechat-notify/send

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/wechat-notify/send" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /api/v1/wechat-notify/settings

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/wechat-notify/settings" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
```

## POST /api/v1/wechat-notify/settings

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/wechat-notify/settings" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## POST /api/v1/wechat-notify/settings/test

```bash
curl -sS \
  -X POST \
  "$WMS_API_BASE/api/v1/wechat-notify/settings/test" \
  -H \
  "Authorization: Bearer $WMS_TOKEN" \
  -H \
  "Content-Type: application/json" \
  -d \
  '{}'
```

## GET /metrics

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/metrics"
```

## GET /openapi.json

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/openapi.json"
```

## GET /redoc

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/redoc"
```
