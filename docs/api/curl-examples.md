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

## GET /api/v1/auth/me

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/auth/me" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
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

## GET /api/v1/driver/tasks/today

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/driver/tasks/today" \
  -H \
  "Authorization: Bearer $WMS_TOKEN"
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

## GET /api/v1/healthz

```bash
curl -sS \
  -X GET \
  "$WMS_API_BASE/api/v1/healthz"
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
