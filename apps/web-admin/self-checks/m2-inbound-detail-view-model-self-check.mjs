import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  batchInfoRows,
  inboundDetailStageIndex,
  inboundDetailStages,
  orderLicenseRows,
  productInfoRows,
  processDetail,
} from "../src/pages/inbound/m2-inbound-detail-view-model.ts";

assert.deepEqual(
  inboundDetailStages.map((item) => item.label),
  ["收货", "验收", "上架", "完成"],
);
assert.equal(inboundDetailStages.some((item) => item.label === "双人签字"), false);
assert.equal(inboundDetailStageIndex("inspecting"), 1);
assert.equal(inboundDetailStageIndex("putaway"), 2);
assert.equal(inboundDetailStageIndex("completed"), 3);

const order = {
  lines: [
    {
      line_no: 1,
      product_code: "P-M2-001",
      product_id: null,
      batch_no: "B1",
      expected_qty: 45,
      production_date: "2026-01-01",
      expiry_date: "2028-01-01",
    },
    {
      line_no: 2,
      product_code: "P-M2-001",
      product_id: null,
      batch_no: "B2",
      expected_qty: 6,
      production_date: "2026-02-01",
      expiry_date: "2028-02-01",
    },
  ],
};
const productRows = productInfoRows(order);
const batchRows = batchInfoRows(order);
const licenseRows = orderLicenseRows(order);

assert.equal(productRows.length, 1);
assert.equal(productRows[0].productCode, "P-M2-001");
// 商品档案/包装主数据尚未随入库明细返回：缺数据必须展示「-」，不得虚构品名、规格、厂家和包装换算
assert.equal(productRows[0].productName, "-");
assert.equal(productRows[0].specification, "-");
assert.equal(productRows[0].manufacturer, "-");
assert.equal(productRows[0].orderQty, "51");
assert.equal(productRows[0].unit, "-");
assert.equal(productRows[0].caseQty, "-");
assert.equal(productRows[0].looseQty, "-");
assert.equal(productRows[0].middlePackQty, "-");
assert.equal(productRows[0].casePackQty, "-");
assert.equal(batchRows.length, 2);
assert.deepEqual(
  batchRows.map((item) => item.batchNo),
  ["B1", "B2"],
);
assert.deepEqual(
  batchRows.map((item) => item.batchQty),
  ["45", "6"],
);
// 批号资质字段尚未随入库明细返回：缺数据必须展示「-」，不得按商品编码虚构药监档案
assert.deepEqual(licenseRows, [
  ["批准文号", "-"],
  ["进口注册证", "-"],
  ["上市持有人", "-"],
]);
assert.equal(batchRows[0].approvalNo, "-");
assert.equal(batchRows[0].importRegistrationCertificate, "-");
assert.equal(batchRows[0].marketingAuthorizationHolder, "-");
assert.equal(batchRows[0].batchCasePackage, "-");
const receivingDetail = processDetail("receiving", 51, 1, {
  actual_qty: "45",
  shortage_qty: "3",
  rejected_qty: "3",
  arrival_temperature_celsius: 5.2,
  exception_note: "B2 外包装破损",
  occurred_at: "2026-08-13T02:30:00.000Z",
  details: {
    delivery_qty: "48",
    temperature_control_method: "冷藏车",
    vehicle_no: "苏A12345",
    origin: "南京配送中心",
    departure_at: "2026-08-13T00:00:00.000Z",
    arrival_at: "2026-08-13T02:00:00.000Z",
    storage_at: "2026-08-13T02:30:00.000Z",
    transport_mode: "公路冷链",
    carrier: "华东医药物流",
    contact_name: "张三",
    contact_phone: "13800000000",
    contact_id_no: "320101199001011234",
    seal_checked: "已核对",
    filing_checked: "已核对",
    second_receiver_id: "00000000-0000-0000-0000-000000000202",
    sales_return_batches: [
      { batch_no: "B1", quantity: "45", rejected_qty: "0", reject_reason: null },
      { batch_no: "B2", quantity: "3", rejected_qty: "3", reject_reason: "外包装破损" },
    ],
  },
});
assert.deepEqual(Object.fromEntries(receivingDetail.rows), {
  "发运地点": "南京配送中心",
  "车牌号": "苏A12345",
  "启运时间": "2026/08/13 08:00",
  "到货时间": "2026/08/13 10:00",
  "收货入库时间": "2026/08/13 10:30",
  "运输方式": "公路冷链",
  "承运商": "华东医药物流",
  "联系人（送货人）": "张三",
  "电话": "138****0000",
  "身份证": "320***********1234",
  "印章样式核对": "已核对",
  "备案件样式核对": "已核对",
  "预报数量": "51 件",
  "送货数量": "48 件",
  "实际到货数量": "45 件",
  "缺货数量": "3 件",
  "拒收数量": "3 件",
  "拒收备注": "B2 外包装破损",
  "销售退货批号 + 数量": "B1 × 45 件；B2 × 3 件",
  "销售退货批号级拒收明细": "B2：拒收 3 件（外包装破损）",
  "第二收货员验证": "00000000-0000-0000-0000-000000000202",
  "到货温度": "5.2 °C",
  "温控方式": "冷藏车",
});
const inspectionDetail = processDetail("inspection", 51, 1);
assert.equal(inspectionDetail.rows.some(([label]) => label === "验收复核"), true);
for (const [label, value] of inspectionDetail.rows) {
  assert.equal(value === "待录入" || value === "-", true, `验收信息「${label}」不得虚构：${value}`);
}

const dialogsSource = readFileSync(new URL("../src/pages/inbound/M2InboundDialogs.tsx", import.meta.url), "utf8");
const pageSource = readFileSync(new URL("../src/pages/inbound/M2InboundPage.tsx", import.meta.url), "utf8");
for (const label of [
  "预报数量",
  "送货数量",
  "实际到货数量",
  "缺货数量",
  "拒收数量",
  "发运地点",
  "启运时间",
  "到货时间",
  "收货入库时间",
  "运输方式",
  "承运商",
  "联系人（送货人）",
  "电话",
  "身份证",
  "印章样式核对",
  "备案件样式核对",
  "第二收货员验证",
  "到货温度 (°C)",
  "温控方式",
  "销售退货批号 + 数量 / 批号级拒收明细",
]) {
  assert.equal(dialogsSource.includes(label), true, `收货表单缺少字段：${label}`);
}
for (const field of ["delivery_qty", "second_receiver_id", "sales_return_batches", "reject_reason"]) {
  assert.equal(pageSource.includes(field), true, `收货请求缺少结构化字段：${field}`);
}
