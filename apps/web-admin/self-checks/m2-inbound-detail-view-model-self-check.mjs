import assert from "node:assert/strict";
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
const receivingDetail = processDetail("receiving", 51, 0);
const receivingLabels = receivingDetail.rows.map(([label]) => label);
assert.equal(receivingLabels.includes("第一收货员"), true);
assert.equal(receivingLabels.includes("第二收货员"), true);
// 收货/验收回执尚未返回：除预报数量外必须展示「待录入 / -」，不得虚构承运、联系人和收货员
for (const [label, value] of receivingDetail.rows) {
  assert.equal(value === "待录入" || value === "-" || value.includes("待录入"), true, `收货信息「${label}」不得虚构：${value}`);
}
assert.equal(receivingDetail.rows.some(([, value]) => value.includes("预报 51 件")), true);
const inspectionDetail = processDetail("inspection", 51, 1);
assert.equal(inspectionDetail.rows.some(([label]) => label === "验收复核"), true);
for (const [label, value] of inspectionDetail.rows) {
  assert.equal(value === "待录入" || value === "-", true, `验收信息「${label}」不得虚构：${value}`);
}
