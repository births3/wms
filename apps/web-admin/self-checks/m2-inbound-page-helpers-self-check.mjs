import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const server = await createServer({
  root: fileURLToPath(new URL("..", import.meta.url)),
  logLevel: "silent",
  server: { middlewareMode: true },
  appType: "custom",
});

try {
  const {
    canInspect,
    canPutaway,
    canReceiveOrReject,
    filterOrders,
    INSPECTION_DUAL_SIGN_REQUIRED_BY_STRATEGY,
    nextM2InboundSelectedId,
    ownerLabel,
    statusKey,
    statusLabel,
    statusColumnFilterOptions,
    statusFilterOptions,
  } = await server.ssrLoadModule("/src/pages/inbound/m2-inbound-page-helpers.ts");
  const { createAsnBatchNo } = await server.ssrLoadModule("/src/pages/inbound/m2-inbound-document-type.ts");

  const ownerA = "00000000-0000-0000-0000-000000000001";
  const ownerB = "11111111-0000-0000-0000-000000000002";
  const ownerContext = { ownerId: ownerA, ownerCode: "PY_OWNER" };
  const orders = [
    orderFixture({ id: "a", owner_id: ownerA, receipt_no: "ASN-A" }),
    orderFixture({ id: "b", owner_id: ownerB, receipt_no: "ASN-B" }),
  ];

  assert.equal(ownerLabel(ownerA, ownerContext), "PY_OWNER");
  assert.equal(ownerLabel(ownerB, ownerContext), "11111111");
  assert.deepEqual(
    filterOrders(orders, "", [], [], "", "", "", "", "PY_OWNER", ownerContext).map((item) => item.receipt_no),
    ["ASN-A"],
  );
  assert.deepEqual(
    filterOrders(orders, "", [], [], "", "", "", "", "11111111", ownerContext).map((item) => item.receipt_no),
    ["ASN-B"],
  );
  assert.equal(createAsnBatchNo("purchase_inbound", "BATCH-001"), null);
  assert.equal(createAsnBatchNo("sales_return", "BATCH-001"), "BATCH-001");
  assert.equal(nextM2InboundSelectedId(null, ["a", "b"], false), "a");
  assert.equal(nextM2InboundSelectedId(null, ["a", "b"], true), null);
  assert.equal(nextM2InboundSelectedId("b", ["a", "b"], true), "b");
  assert.equal(nextM2InboundSelectedId("missing", ["a"], false), "a");
  assert.deepEqual(statusFilterOptions("inspecting").map((item) => item.value), ["inspecting"]);
  assert.deepEqual(statusColumnFilterOptions("inspecting").map((item) => item.value), ["inspecting"]);
  assert.deepEqual(statusFilterOptions("putaway").map((item) => item.value), ["putaway", "completed"]);
  assert.ok(statusColumnFilterOptions("putaway").some((item) => item.value === "putaway"));
  assert.equal(statusLabel(null), "-");
  assert.equal(statusKey(null), "pending");

  assert.equal(canReceiveOrReject("released"), true);
  assert.equal(canReceiveOrReject("receiving"), true);
  assert.equal(canReceiveOrReject("completed"), false);
  assert.equal(canInspect("inspecting"), true);
  assert.equal(canInspect("receiving"), true);
  assert.equal(canInspect("completed"), false);
  assert.equal(canInspect("closed_rejected"), false);
  assert.equal(canPutaway("putaway"), true);
  assert.equal(canPutaway("inspecting"), true);
  assert.equal(canPutaway("completed"), false);
  assert.equal(canPutaway("closed_rejected"), false);
  assert.equal(INSPECTION_DUAL_SIGN_REQUIRED_BY_STRATEGY, true, "验收双人签字策略默认锁定");

  const pageSource = readFileSync(fileURLToPath(new URL("../src/pages/inbound/M2InboundPage.tsx", import.meta.url)), "utf8");
  const orderTableSource = readFileSync(fileURLToPath(new URL("../src/pages/inbound/M2InboundOrderTable.tsx", import.meta.url)), "utf8");
  const dialogSource = readFileSync(fileURLToPath(new URL("../src/pages/inbound/M2InboundDialogs.tsx", import.meta.url)), "utf8");
  const devMockCore = readFileSync(fileURLToPath(new URL("../dev-mocks/web-admin-dev-mock-core.ts", import.meta.url)), "utf8");
  const devMockCommon = readFileSync(fileURLToPath(new URL("../dev-mocks/web-admin-dev-mock-core-common.ts", import.meta.url)), "utf8");
  const devMockModel = readFileSync(fileURLToPath(new URL("../dev-mocks/web-admin-dev-mock-model.ts", import.meta.url)), "utf8");
  const createFormBlock = /const emptyCreateForm: CreateFormState = \{([\s\S]*?)\};/.exec(pageSource)?.[1] ?? "";
  const inspectFormBlock = /const emptyInspectForm: InspectFormState = \{([\s\S]*?)\};/.exec(pageSource)?.[1] ?? "";
  const signFormBlock = /const emptySignForm: SignFormState = \{([\s\S]*?)\};/.exec(pageSource)?.[1] ?? "";
  const submitInspectBlock = /async function submitInspect\([\s\S]*?\n  \}/.exec(pageSource)?.[0] ?? "";
  assert.ok(createFormBlock, "M2 新建 ASN 表单必须使用可复位的空初始值");
  for (const field of ["receiptNo", "documentType", "supplierId", "warehouseId", "expectedArrivalDate", "productCode", "batchNo", "expectedQty", "productionDate", "expiryDate"]) {
    assert.match(createFormBlock, new RegExp(`${field}: ""`), `新建 ASN 默认值必须为空: ${field}`);
  }
  assert.doesNotMatch(createFormBlock, /ASN-M2-PC-0002|P-M2-002|2026-02-01|2028-02-01|"60"/, "新建 ASN 样例值不能作为表单 value");
  assert.match(pageSource, /function openCreateDialog\(\) \{[\s\S]*setCreateForm\(emptyCreateForm\);[\s\S]*setActiveDialog\("create"\);[\s\S]*\}/, "点击新建 ASN 必须重置为空表单");
  assert.match(pageSource, /onClick: openCreateDialog/, "新建 ASN 按钮必须走重置入口");
  assert.match(pageSource, /const \[selectedRowKeys, setSelectedRowKeys\] = React\.useState<string\[\]>\(\[\]\);/, "M2 DataGrid 必须保留多选 keys，表头全选再取消才能清空");
  assert.doesNotMatch(orderTableSource, /onSelectedRowKeysChange=\{\(keys\) => onSelectOrder\(keys\.at\(-1\) \?\? null\)\}/, "M2 表格不能把全选结果压成最后一条");
  assert.match(orderTableSource, /row\.status[\s\S]*<StatusBadge[\s\S]*text-muted-foreground/, "空状态不得伪装成待处理徽标");
  assert.match(orderTableSource, /canInspect\(selectedOrder\.status\)/, "验收动作必须按状态裁剪");
  assert.match(orderTableSource, /canPutaway\(selectedOrder\.status\)/, "上架动作必须按状态裁剪");
  assert.match(dialogSource, /<TextField label="ASN 号" required placeholder="例如 ASN-M2-PC-0002"/, "ASN 样例只允许作为 placeholder");
  assert.match(dialogSource, /<ProductLookupField[\s\S]*placeholder="例如 P-M2-002"[\s\S]*required/, "商品编码样例只允许作为 ProductLookupField placeholder");
  assert.match(dialogSource, /<TextField label="预报数量" type="number" required placeholder="例如 60"/, "预报数量样例只允许作为 placeholder");
  assert.ok(inspectFormBlock, "M2 验收表单必须使用可复位的空初始值");
  for (const field of ["batchNo", "acceptedQty", "rejectedQty", "productionDate", "expiryDate", "qualityStatus", "traceCodes", "appearanceCheck", "packageCheck", "instructionCheck", "labelCheck", "note"]) {
    assert.match(inspectFormBlock, new RegExp(`${field}: ""`), `验收默认值必须为空: ${field}`);
  }
  assert.ok(signFormBlock, "M2 验收签字表单必须使用可复位的空初始值");
  for (const field of ["firstSignerId", "secondSignerId", "strategyNote", "note"]) {
    assert.match(signFormBlock, new RegExp(`${field}: ""`), `验收签字默认值必须为空: ${field}`);
  }
  assert.match(signFormBlock, /dualRequired: true/, "签字表单 dualRequired 默认 true");
  assert.match(pageSource, /createSignFormForCurrentUser/, "打开验收时第一签字人默认当前用户账号");
  assert.match(pageSource, /INSPECTION_DUAL_SIGN_REQUIRED_BY_STRATEGY \|\| signForm\.dualRequired/, "提交验收时 dualRequired 必须被策略锁定");
  assert.match(pageSource, /firstSignerId: "当前用户 \/ 工号"/, "第一签字人 placeholder 应为当前用户/工号类文案");
  assert.match(pageSource, /secondSignerExample = "00000000-0000-0000-0000-000000000102"/, "第二签字人示例应符合 UUID 契约");
  assert.doesNotMatch(pageSource, /firstSignerId: `例如 \$\{firstSignerId\}`|secondSignerId: `例如 \$\{secondSignerId\}`/, "签字人 placeholder 不得以 UUID 样例为主");
  assert.match(dialogSource, /label="第二签字人 ID"/, "第二签字人 label 应明确要求用户 ID");
  assert.match(pageSource, /if \(!isUuid\(firstSignerId\) \|\| \(secondSignerId !== null && !isUuid\(secondSignerId\)\)\)/, "签字 ID 必须在验收提交前校验");
  assert.match(dialogSource, /disabled=\{INSPECTION_DUAL_SIGN_REQUIRED_BY_STRATEGY\}/, "策略要求时双人签字 checkbox 必须 disabled");
  assert.match(dialogSource, /策略要求，不可关闭/, "策略锁定需有可读提示");
  assert.match(dialogSource, /SelectField label="质量状态"[\s\S]*\["qualified", "合格"\]/, "质量状态选项需中文");
  assert.match(dialogSource, /activeDialog === "inspect"[\s\S]*DialogDescription>\{orderReceiptNo/, "验收弹窗需保留单号上下文");
  assert.match(dialogSource, /activeDialog === "putaway"[\s\S]*DialogDescription>\{orderReceiptNo/, "上架弹窗需保留单号上下文");
  assert.match(dialogSource, /<TextField label="验收批号" required placeholder=\{inspectExamples\.batchNo\}/, "验收批号背景值只允许作为 placeholder");
  assert.match(dialogSource, /<TextField label="通过数量" type="number" required placeholder=\{inspectExamples\.acceptedQty\}/, "通过数量背景值只允许作为 placeholder");
  assert.doesNotMatch(submitInspectBlock, /line\?\.batch_no \|\| "BATCH-202606"|inspectForm\.traceCodes \|\| "TC-M2-0001"/, "验收提交不能用背景值或样例值兜底");
  assert.match(devMockModel, /devSeedOrderCount = 100/, "M2 dev mock 必须保留 100 条入库单");
  assert.match(devMockCommon, /devSeedOrderStatusOverrides\.get\(id\)/, "种子入库单查询必须读取动作后的状态覆盖");
  assert.match(devMockCommon, /devSeedOrderStatusOverrides\.set\(id, status\)/, "种子入库单动作必须持久化状态覆盖");
  assert.match(devMockCore, /page: \{ count: data\.length, next_cursor: null \}/, "M2 dev mock 列表必须返回分页元数据");
} finally {
  await server.close();
}

function orderFixture(overrides) {
  return {
    id: overrides.id,
    owner_id: overrides.owner_id,
    receipt_no: overrides.receipt_no,
    document_type: "purchase_inbound",
    warehouse_id: "00000000-0000-0000-0000-000000003001",
    status: "released",
    expected_arrival_at: "2026-06-28T10:00:00.000Z",
    external_ref: null,
    supplier_id: "00000000-0000-0000-0000-000000005001",
    created_at: "2026-06-28T09:00:00.000Z",
    updated_at: "2026-06-28T09:00:00.000Z",
    lines: [
      {
        line_no: 1,
        product_code: "P-M2-001",
        product_id: null,
        batch_no: null,
        expected_qty: 10,
        production_date: null,
        expiry_date: null,
      },
    ],
  };
}
