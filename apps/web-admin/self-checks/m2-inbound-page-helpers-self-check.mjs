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
    canRelease,
    canReceiveOrReject,
    filterOrders,
    dualSignRequiredForPolicy,
    localDayRange,
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
  const localRange = localDayRange(new Date(2026, 6, 12, 12, 0, 0));
  assert.ok(localRange.from < localRange.to, "看板本地日期范围必须有明确起止边界");

  assert.equal(canReceiveOrReject("released"), true);
  assert.equal(canReceiveOrReject("receiving"), true);
  assert.equal(canReceiveOrReject("completed"), false);
  assert.equal(canRelease("draft"), true);
  assert.equal(canRelease("released"), false);
  assert.equal(canInspect("inspecting"), true);
  assert.equal(canInspect("receiving"), true);
  assert.equal(canInspect("completed"), false);
  assert.equal(canInspect("closed_rejected"), false);
  assert.equal(canPutaway("putaway"), true);
  assert.equal(canPutaway("inspecting"), true);
  assert.equal(canPutaway("completed"), false);
  assert.equal(canPutaway("closed_rejected"), false);
  assert.equal(dualSignRequiredForPolicy("single"), false);
  assert.equal(dualSignRequiredForPolicy("dual_scan"), true);
  assert.equal(dualSignRequiredForPolicy("dual_scan_with_approval"), true);

  const pageSource = readFileSync(fileURLToPath(new URL("../src/pages/inbound/M2InboundPage.tsx", import.meta.url)), "utf8");
  const dashboardPageSource = readFileSync(fileURLToPath(new URL("../src/pages/inbound/M2InboundDashboardPage.tsx", import.meta.url)), "utf8");
  const orderTableSource = readFileSync(fileURLToPath(new URL("../src/pages/inbound/M2InboundOrderTable.tsx", import.meta.url)), "utf8");
  const printDialogSource = readFileSync(fileURLToPath(new URL("../src/pages/inbound/M2InboundPrintDialog.tsx", import.meta.url)), "utf8");
  const dialogSource = readFileSync(fileURLToPath(new URL("../src/pages/inbound/M2InboundDialogs.tsx", import.meta.url)), "utf8");
  const inboundQueriesSource = readFileSync(fileURLToPath(new URL("../src/features/inbound/inbound-queries.ts", import.meta.url)), "utf8");
  const realE2eSource = readFileSync(fileURLToPath(new URL("../../../prototypes/e2e/web-admin-m2-real.spec.ts", import.meta.url)), "utf8");
  const helperSource = readFileSync(fileURLToPath(new URL("../src/pages/inbound/m2-inbound-page-helpers.ts", import.meta.url)), "utf8");
  const appShell = readFileSync(fileURLToPath(new URL("../src/App.tsx", import.meta.url)), "utf8");
  const devMockCore = readFileSync(fileURLToPath(new URL("../dev-mocks/web-admin-dev-mock-core.ts", import.meta.url)), "utf8");
  const devMockCommon = readFileSync(fileURLToPath(new URL("../dev-mocks/web-admin-dev-mock-core-common.ts", import.meta.url)), "utf8");
  const devMockModel = readFileSync(fileURLToPath(new URL("../dev-mocks/web-admin-dev-mock-model.ts", import.meta.url)), "utf8");
  // 页面导航契约：质量矩阵 navigation_checks 依赖这些菜单 page id 字面量
  assert.match(appShell, /id:\s*"m2-receiving"/, "管理端菜单应登记 m2-receiving");
  assert.match(appShell, /id:\s*"m2-inspecting"/, "管理端菜单应登记 m2-inspecting");
  assert.match(appShell, /id:\s*"m2-putaway"/, "管理端菜单应登记 m2-putaway");
  assert.match(pageSource, /M2InboundPage|m2-receiving|m2-inspecting|m2-putaway/, "M2 入库页应覆盖收货/验收/上架视图");
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
  assert.match(pageSource, /const created = await createMutation\.mutateAsync\(request\);[\s\S]*await ordersQuery\.refetch\(\);[\s\S]*selectOrder\(created\.id\);/, "新建 ASN 后必须先刷新列表再选中单据，保证放行按钮状态正确");
  assert.match(pageSource, /onClick: openCreateDialog/, "新建 ASN 按钮必须走重置入口");
  assert.match(pageSource, /const \[selectedRowKeys, setSelectedRowKeys\] = React\.useState<string\[\]>\(\[\]\);/, "M2 DataGrid 必须保留多选 keys，表头全选再取消才能清空");
  assert.doesNotMatch(orderTableSource, /onSelectedRowKeysChange=\{\(keys\) => onSelectOrder\(keys\.at\(-1\) \?\? null\)\}/, "M2 表格不能把全选结果压成最后一条");
  assert.match(orderTableSource, /row\.status[\s\S]*<StatusBadge[\s\S]*text-muted-foreground/, "空状态不得伪装成待处理徽标");
  assert.match(orderTableSource, /canInspect\(selectedOrder\.status\)/, "验收动作必须按状态裁剪");
  assert.match(orderTableSource, /canPutaway\(selectedOrder\.status\)/, "上架动作必须按状态裁剪");
  assert.match(orderTableSource, /canRelease\(selectedOrder\.status\)/, "ASN 放行动作必须仅允许草稿状态");
  assert.match(orderTableSource, /printAction: DataGridPrintAction/, "M2 打印必须接入 DataGrid 公共打印按钮");
  assert.match(orderTableSource, /onOpenPrint\(selectedOrder\.id\)/, "M2 打印按钮必须打开业务打印弹窗");
  assert.match(pageSource, /M2InboundPrintDialog/, "M2 页面必须挂载业务打印弹窗");
  assert.match(printDialogSource, /usePreviewPrintTemplateMutation/, "M2 打印必须复用 H9 模板预览 API");
  assert.match(printDialogSource, /useRecordPrintTemplateMutation/, "M2 打印必须写入 H9 打印记录");
  assert.match(printDialogSource, /template_type_code: templateTypeCode\(mode\)/, "M2 打印必须按入库页面类型选择 H9 模板类型");
  assert.match(pageSource, /useReleaseReceivingOrderMutation/, "M2 页面必须接入真实 ASN 放行 API");
  assert.match(pageSource, /useMasterDataRowsQuery\("m1-locations", mode === "putaway"\)/, "上架页面必须读取 M1 库位主数据");
  assert.match(pageSource, /row\.code === locationCode[\s\S]*row\.locationFields\?\.warehouseId === order\.warehouse_id/, "上架提交必须按仓库和库位编码解析真实库位 ID");
  assert.match(pageSource, /const qty = toInteger\(putawayForm\.qty\)[\s\S]*上架数量必须大于 0/, "上架数量非法时必须阻止真实提交并显示原因");
  assert.match(inboundQueriesSource, /putaway-recommendations[\s\S]*api\.GET\("\/api\/v1\/inbound\/receiving-orders\/{id}\/putaway-recommendations"/, "推荐库位必须调用真实 API");
  assert.match(dialogSource, /usePutawayRecommendationsQuery[\s\S]*activeDialog === "putaway"/, "上架弹窗必须按当前单据读取推荐库位");
  assert.match(dialogSource, /推荐原因：|推荐 #\{index \+ 1\}/, "上架弹窗必须展示推荐排序和原因");
  assert.match(dialogSource, /暂无符合温区、色标和容量规则/, "推荐库位为空时必须可见");
  assert.match(dialogSource, /clearPutawayValidationError/, "上架输入变更必须清理过期校验提示");
  assert.match(dialogSource, /<TextField label="ASN 号" placeholder="留空由 M-CG 编号规则生成"/, "ASN 号应支持由 M-CG 自动生成");
  assert.match(dialogSource, /<TextField label="供应商 ID" required/, "新建 ASN 必须要求供应商");
  assert.match(dialogSource, /<TextField label="预计到货" type="date" required/, "新建 ASN 必须要求预计到货时间");
  assert.match(dialogSource, /<TextField label="ASN 批号" required/, "销售退货 ASN 必须要求原销售批号");
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
  assert.match(pageSource, /dualSignRequiredByStrategy \|\| signForm\.dualRequired/, "提交验收时 dualRequired 必须由实时策略锁定");
  assert.match(pageSource, /useDualPersonPolicyQuery/, "验收弹窗必须按商品、货主、仓库实时查询 M-VR 策略");
  assert.match(pageSource, /firstSignerId: "当前用户 \/ 工号"/, "第一签字人 placeholder 应为当前用户/工号类文案");
  assert.match(pageSource, /secondSignerExample = "00000000-0000-0000-0000-000000000102"/, "第二签字人示例应符合 UUID 契约");
  assert.doesNotMatch(pageSource, /firstSignerId: `例如 \$\{firstSignerId\}`|secondSignerId: `例如 \$\{secondSignerId\}`/, "签字人 placeholder 不得以 UUID 样例为主");
  assert.match(dialogSource, /label="第二签字人 ID"/, "第二签字人 label 应明确要求用户 ID");
  assert.match(pageSource, /if \(!isUuid\(firstSignerId\) \|\| \(secondSignerId !== null && !isUuid\(secondSignerId\)\)\)/, "签字 ID 必须在验收提交前校验");
  assert.match(pageSource, /if \(secondSignerId === firstSignerId\) \{[\s\S]*第二签字人不能与第一签字人相同/, "相同签字人必须在发起验收请求前阻断");
  assert.match(pageSource, /await inspectMutation\.mutateAsync\([\s\S]*await signMutation\.mutateAsync\(/, "PC 验收必须先写入验收记录再提交双人签字");
  assert.match(inboundQueriesSource, /api\.POST\("\/api\/v1\/inbound\/receiving-orders\/\{id\}\/sign"/, "PC 验收必须调用真实双人签字 API");
  assert.match(dialogSource, /disabled=\{dualSignRequiredByStrategy\}/, "策略要求时双人签字 checkbox 必须 disabled");
  assert.match(dialogSource, /dualPolicyDescription/, "策略锁定需显示实时命中策略");
  assert.match(dialogSource, /SelectField label="质量状态"[\s\S]*\["qualified", "合格"\]/, "质量状态选项需中文");
  assert.match(dialogSource, /activeDialog === "inspect"[\s\S]*DialogDescription>\{orderReceiptNo/, "验收弹窗需保留单号上下文");
  assert.match(dialogSource, /activeDialog === "putaway"[\s\S]*DialogDescription>\{orderReceiptNo/, "上架弹窗需保留单号上下文");
  assert.match(dialogSource, /<TextField label="验收批号" required placeholder=\{inspectExamples\.batchNo\}/, "验收批号背景值只允许作为 placeholder");
  assert.match(dialogSource, /<TextField label="通过数量" type="number" required placeholder=\{inspectExamples\.acceptedQty\}/, "通过数量背景值只允许作为 placeholder");
  assert.doesNotMatch(submitInspectBlock, /line\?\.batch_no \|\| "BATCH-202606"|inspectForm\.traceCodes \|\| "TC-M2-0001"/, "验收提交不能用背景值或样例值兜底");
  assert.match(realE2eSource, /第二签字人 ID.*fill\(signerIds\.firstSignerId\)[\s\S]*第二签字人不能与第一签字人相同/, "真实 E2E 必须覆盖相同签字人失败路径");
  assert.match(realE2eSource, /signatureResponse\.status\(\)\)\.toBe\(200\)/, "真实 E2E 必须断言双人签字 API 成功响应");
  assert.match(realE2eSource, /first_signer_id: signerIds\.firstSignerId[\s\S]*second_signer_id: signerIds\.secondSignerId[\s\S]*dual_required: true/, "真实 E2E 必须断言双签请求携带两个不同签字人");
  assert.match(devMockModel, /devSeedOrderCount = 100/, "M2 dev mock 必须保留 100 条入库单");
  assert.match(devMockCommon, /devSeedOrderStatusOverrides\.get\(id\)/, "种子入库单查询必须读取动作后的状态覆盖");
  assert.match(devMockCommon, /devSeedOrderStatusOverrides\.set\(id, status\)/, "种子入库单动作必须持久化状态覆盖");
  assert.match(devMockCore, /page: \{ count: data\.length, next_cursor: null \}/, "M2 dev mock 列表必须返回分页元数据");
  assert.match(pageSource, /M2InboundDashboardPage/, "M2 入库页必须提供进度看板入口");
  assert.match(pageSource, /currentOwner=\{currentOwner\}/, "M2 进度看板必须继承当前货主上下文");
  assert.match(dashboardPageSource, /<QueryPanel/, "M2 入库看板必须复用 QueryPanel");
  assert.match(dashboardPageSource, /<DataGrid/, "M2 入库看板必须复用 DataGrid");
  assert.match(dashboardPageSource, /useReceivingOrdersQuery/, "M2 入库看板点击状态行必须读取对应单据");
  assert.match(dashboardPageSource, /M2InboundDetailDialog/, "M2 入库看板必须支持打开单据详情弹窗");
  assert.match(dashboardPageSource, /setInterval|刷新间隔/, "M2 入库看板必须提供可配置的自动刷新");
  assert.match(dashboardPageSource, /statusLabel\(row\.status\)/, "M2 入库看板必须显示中文状态标签");
  assert.match(devMockCore, /receiving-dashboard/, "M2 dev mock 必须提供看板真实 API 形状");
  const dashboardSource = dashboardPageSource;
  assert.match(dashboardSource, /localDayRange/, "看板默认今天必须按本地日转换为 UTC 范围");
  assert.match(helperSource, /new Date\(`\$\{value\}T10:00:00`\)/, "日期输入必须按本地日期转换，不能硬编码 UTC 日期");
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
