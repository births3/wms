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
assert.equal(productRows[0].productName, "感冒灵颗粒");
assert.equal(productRows[0].specification, "10g*9袋");
assert.equal(productRows[0].manufacturer, "示例药业");
assert.equal(productRows[0].orderQty, "51");
assert.equal(productRows[0].unit, "件");
assert.equal(productRows[0].caseQty, "2");
assert.equal(productRows[0].looseQty, "11");
assert.equal(productRows[0].middlePackQty, "10 件/中包");
assert.equal(productRows[0].casePackQty, "20 件/件");
assert.equal(batchRows.length, 2);
assert.deepEqual(
  batchRows.map((item) => item.batchNo),
  ["B1", "B2"],
);
assert.deepEqual(
  batchRows.map((item) => item.batchQty),
  ["45", "6"],
);
assert.deepEqual(licenseRows, [
  ["批准文号", "国药准字Z0001"],
  ["进口注册证", "-"],
  ["上市持有人", "示例药业"],
]);
assert.equal(batchRows[0].approvalNo, "国药准字Z0001");
assert.equal(batchRows[0].importRegistrationCertificate, "-");
assert.equal(batchRows[0].marketingAuthorizationHolder, "示例药业");
assert.equal(batchRows[0].batchCasePackage, "2 件 + 5 零 / 20 件/件");
const receivingLabels = processDetail("receiving", 51, 0).rows.map(([label]) => label);
assert.equal(receivingLabels.includes("第一收货员"), true);
assert.equal(receivingLabels.includes("第二收货员"), true);
assert.equal(processDetail("inspection", 51, 1).rows.some(([label]) => label === "验收复核"), true);
