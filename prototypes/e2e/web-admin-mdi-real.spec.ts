// @governance: skip-page-size M-DI 真实链路三场景共用一批种子/夹具（runPsql/附件/验收弹窗），拆分会复制夹具；后续夹具抽共享后再拆。
import { expect, test } from "@playwright/test";
import { execFileSync } from "node:child_process";
import path from "node:path";
import {
  adminUserId,
  batchA,
  batchB,
  correctedReport,
  createReceivedAsn,
  editedReport,
  evidenceDir,
  fillAcceptanceDialog,
  firstReceipt,
  firstReport,
  inventoryLocationId,
  m4ReviewBatch,
  m4ReviewOrderId,
  m4ReviewOrderNo,
  openDocuments,
  openReview,
  openStampPage,
  ownerId,
  prepareFiles,
  productId,
  readCustomerCopy,
  replacementPng,
  reportPng,
  reviewUserId,
  runTag,
  secondBatchReport,
  secondReceipt,
  setDefaultRequirementRule,
  stampPng,
  stampedReport,
  upstreamPdf,
  uploadReport,
} from "./web-admin-mdi-real-ui";
import {
  approveQualityLiaison,
  closeEntryDialog,
  dispatchWindowPointer,
  login,
  openMenu,
  switchUser,
} from "./web-admin-mdi-real-actions";

test("M-DI 真实上传、退回修改、双人确认、复用、版本和上游单据", async ({ page }) => {
  await prepareFiles(page);
  await login(page, "admin");
  await setDefaultRequirementRule(page, false);
  await createReceivedAsn(page, firstReceipt, [batchA, batchB]);
  await createReceivedAsn(page, secondReceipt, [batchA]);

  await openDocuments(page, runTag);
  await expect(page.getByRole("navigation").getByRole("button", { name: /入库资料录入/ }))
    .toHaveAttribute("aria-current", "page");
  await expect(page.locator("tbody tr").filter({ hasText: firstReceipt })).toBeVisible();
  await expect(page.locator("tbody tr").filter({ hasText: secondReceipt })).toBeVisible();
  await page.getByRole("button", { name: /药检单不齐/ }).click();
  await expect(page.locator("tbody tr").filter({ hasText: firstReceipt })).toBeVisible();
  await page.screenshot({ path: path.join(evidenceDir, "01-missing-quick-filter.png"), fullPage: true });
  await page.getByRole("button", { name: /药检单不齐/ }).click();

  await uploadReport(page, firstReceipt, {
    batchNo: batchA,
    reportNo: firstReport,
    filePath: reportPng,
    processingMode: "black_white_enhance",
    previewEvidencePath: path.join(evidenceDir, "02-image-processing-preview.png"),
  });
  await page.screenshot({ path: path.join(evidenceDir, "02-upload-submitted.png"), fullPage: true });
  await closeEntryDialog(page);

  await switchUser(page, "mvr-matrix-approver");
  await openReview(page, firstReport);
  const rejectDialog = page.getByRole("dialog", { name: new RegExp(firstReport) });
  await expect(rejectDialog.getByText("黑白增强")).toBeVisible();
  await rejectDialog.getByLabel("审核意见（退回时必填）").fill("图片字迹不够清晰，请重新上传");
  await page.screenshot({ path: path.join(evidenceDir, "03-review-reject-detail.png"), fullPage: true });
  await rejectDialog.getByRole("button", { name: "退回修改" }).click();
  await expect(page.getByRole("status")).toContainText("药检单已退回修改");

  await switchUser(page, "admin");
  await openDocuments(page, runTag);
  await uploadReport(page, firstReceipt, {
    batchNo: batchA,
    reportNo: editedReport,
    filePath: replacementPng,
    processingMode: "color_enhance",
  });
  await expect(page.getByRole("dialog").getByRole("status")).toContainText("上传并提交审核");
  await page.screenshot({ path: path.join(evidenceDir, "04-rejected-draft-edited.png"), fullPage: true });
  await closeEntryDialog(page);

  await switchUser(page, "mvr-matrix-approver");
  await openReview(page, editedReport);
  await page.screenshot({ path: path.join(evidenceDir, "05-resubmitted-review.png"), fullPage: true });
  await page.getByRole("dialog", { name: new RegExp(editedReport) })
    .getByRole("button", { name: "确认通过" })
    .click();
  await expect(page.getByRole("status")).toContainText("药检单已确认");

  await switchUser(page, "admin");
  await openDocuments(page, runTag);
  await uploadReport(page, firstReceipt, {
    batchNo: batchB,
    reportNo: secondBatchReport,
    filePath: reportPng,
    processingMode: "none",
  });
  await closeEntryDialog(page);
  await switchUser(page, "mvr-matrix-approver");
  await openReview(page, secondBatchReport);
  await page.getByRole("dialog", { name: new RegExp(secondBatchReport) })
    .getByRole("button", { name: "确认通过" })
    .click();
  await expect(page.getByRole("status")).toContainText("药检单已确认");

  await switchUser(page, "admin");
  await openDocuments(page, runTag);
  await uploadReport(page, firstReceipt, {
    batchNo: batchA,
    reportNo: correctedReport,
    filePath: replacementPng,
    processingMode: "color_enhance",
    reason: "供应商补发清晰版并更正报告编号",
  });
  await closeEntryDialog(page);

  await switchUser(page, "mvr-matrix-approver");
  await openReview(page, correctedReport);
  const correctionDialog = page.getByRole("dialog", { name: new RegExp(correctedReport) });
  await expect(correctionDialog.getByText(/v2 · 待确认/)).toBeVisible();
  await expect(correctionDialog.getByText(/v1 · 已确认/)).toBeVisible();
  await expect(correctionDialog.getByText("修改原因：供应商补发清晰版并更正报告编号")).toBeVisible();
  await page.screenshot({ path: path.join(evidenceDir, "06-version-chain-before-switch.png"), fullPage: true });
  await correctionDialog.getByRole("button", { name: "确认通过" }).click();

  await switchUser(page, "admin");
  await openDocuments(page, runTag);
  const secondRow = page.locator("tbody tr").filter({ hasText: secondReceipt });
  await secondRow.getByRole("button", { name: "录入资料" }).click();
  const reuseDialog = page.getByRole("dialog", { name: new RegExp(secondReceipt) });
  await reuseDialog.getByLabel("录入方式").selectOption("reuse");
  await reuseDialog.getByRole("button", { name: "保存药检单" }).click();
  await expect(reuseDialog.getByRole("status")).toContainText(`${batchA} 药检单已复用`);
  await page.screenshot({ path: path.join(evidenceDir, "07-confirmed-report-reused.png"), fullPage: true });
  await closeEntryDialog(page);

  const firstRow = page.locator("tbody tr").filter({ hasText: firstReceipt });
  await firstRow.getByRole("button", { name: "录入资料" }).click();
  const upstreamDialog = page.getByRole("dialog", { name: new RegExp(firstReceipt) });
  await upstreamDialog.getByRole("tab", { name: "上游随货同行单" }).click();
  const upstreamPanel = upstreamDialog.getByRole("tabpanel", { name: "上游随货同行单" });
  await upstreamPanel.locator("label").filter({ hasText: secondReceipt }).getByRole("checkbox").check();
  await upstreamPanel.locator('input[type="file"]').setInputFiles(upstreamPdf);
  await page.screenshot({ path: path.join(evidenceDir, "08-upstream-multi-asn-selection.png"), fullPage: true });
  await upstreamDialog.getByRole("button", { name: "上传并完成录入" }).click();
  await expect(upstreamDialog.getByRole("status")).toContainText("已关联 2 个 ASN");
  await closeEntryDialog(page);

  await page.reload();
  await expect(page.getByRole("heading", { name: "入库资料录入" })).toBeVisible();
  await page.getByLabel("关键字").fill(runTag);
  await page.getByRole("button", { name: "查询", exact: true }).click();
  for (const receiptNo of [firstReceipt, secondReceipt]) {
    const row = page.locator("tbody tr").filter({ hasText: receiptNo });
    await expect(row.getByText("已上传")).toBeVisible();
    await expect(row.getByText("已齐全")).toBeVisible();
  }
  await page.screenshot({ path: path.join(evidenceDir, "09-persisted-complete-list.png"), fullPage: true });
});

test("M-DI 透明图章可拖动缩放并由另一账号发布", async ({ page }) => {
  await prepareFiles(page);
  await page.setContent(`
    <svg xmlns="http://www.w3.org/2000/svg" width="160" height="160">
      <circle cx="80" cy="80" r="68" fill="none" stroke="#d11919" stroke-width="10" />
      <rect x="32" y="62" width="96" height="36" fill="none" stroke="#d11919" stroke-width="6" />
      <text x="80" y="88" text-anchor="middle" font-size="22" fill="#d11919">药检专用</text>
    </svg>
  `);
  await page.locator("svg").screenshot({ path: stampPng, omitBackground: true });
  await login(page, "admin");
  await openStampPage(page);
  await page.getByLabel("透明 PNG 图章").setInputFiles(stampPng);
  const stamp = page.getByRole("img", { name: "待发布的透明 PNG 图章" });
  await expect(stamp).toBeVisible();
  const stampBox = await stamp.boundingBox();
  if (!stampBox) throw new Error("图章预览没有可用尺寸");
  await stamp.locator("..").dispatchEvent("pointerdown", {
    pointerId: 1,
    isPrimary: true,
    button: 0,
    clientX: stampBox.x + stampBox.width / 2,
    clientY: stampBox.y + stampBox.height / 2,
  });
  await page.waitForTimeout(100);
  await dispatchWindowPointer(page, "pointermove", stampBox.x - 70, stampBox.y - 90);
  await dispatchWindowPointer(page, "pointerup", stampBox.x - 70, stampBox.y - 90);
  await expect(page.getByText("68.0%", { exact: true })).toHaveCount(0);
  const resizeHandle = page.getByRole("button", { name: "拖动缩放图章" });
  const resizeBox = await resizeHandle.boundingBox();
  if (!resizeBox) throw new Error("图章缩放手柄不可用");
  await resizeHandle.dispatchEvent("pointerdown", {
    pointerId: 1,
    isPrimary: true,
    button: 0,
    clientX: resizeBox.x + resizeBox.width / 2,
    clientY: resizeBox.y + resizeBox.height / 2,
  });
  await page.waitForTimeout(100);
  await dispatchWindowPointer(page, "pointermove", resizeBox.x + 80, resizeBox.y + 40);
  await dispatchWindowPointer(page, "pointerup", resizeBox.x + 80, resizeBox.y + 40);
  await expect(page.getByText("20.0%", { exact: true })).toHaveCount(0);
  await page.screenshot({ path: path.join(evidenceDir, "10-stamp-drag-resize.png"), fullPage: true });

  const submitted = page.waitForResponse(
    (response) => {
      const pathname = new URL(response.url()).pathname;
      return pathname.startsWith("/api/v1/drug-inspection/stamp-versions/")
        && pathname.endsWith("/submit")
        && response.request().method() === "POST";
    },
  );
  await page.getByRole("button", { name: "上传并提交审核" }).click();
  await page
    .getByRole("dialog", { name: "确认上传图章" })
    .getByRole("button", { name: "确认上传并提交审核" })
    .click();
  const version = await (await submitted).json() as { version_number: number };
  await expect(page.getByRole("status")).toContainText(`图章 v${version.version_number} 已提交`);

  await switchUser(page, "mvr-matrix-approver");
  await openStampPage(page);
  const versionRow = page.locator("tbody tr").filter({
    has: page.getByText(`v${version.version_number}`, { exact: true }),
  });
  await expect(versionRow.getByText("待发布审核")).toBeVisible();
  await page.screenshot({ path: path.join(evidenceDir, "11-stamp-pending-second-review.png"), fullPage: true });
  await versionRow.getByRole("button", { name: "发布" }).click();
  await page
    .getByRole("dialog", { name: "确认发布图章" })
    .getByRole("button", { name: "确认发布" })
    .click();
  await expect(page.getByRole("status")).toContainText("图章版本已发布");
  await expect(versionRow.getByText("已发布")).toBeVisible();
  await page.screenshot({ path: path.join(evidenceDir, "12-stamp-published.png"), fullPage: true });

  await switchUser(page, "admin");
  await openDocuments(page, runTag);
  await uploadReport(page, firstReceipt, {
    batchNo: batchA,
    reportNo: stampedReport,
    filePath: reportPng,
    processingMode: "black_white_enhance",
    reason: "发布图章后生成真实客户分发副本",
  });
  await closeEntryDialog(page);
  await switchUser(page, "mvr-matrix-approver");
  await openReview(page, stampedReport);
  await page.getByRole("dialog", { name: new RegExp(stampedReport) })
    .getByRole("button", { name: "确认通过" })
    .click();
  await expect(page.getByRole("status")).toContainText("药检单已确认");
  await expect.poll(
    () => readCustomerCopy(page, stampedReport),
    { timeout: 30_000, intervals: [500, 1_000, 2_000] },
  ).toMatchObject({ status: "available", pdfHeader: "%PDF-" });
  const customerCopy = await readCustomerCopy(page, stampedReport);
  await openStampPage(page);
  const copyRow = page.locator("tbody tr").filter({ hasText: customerCopy.versionId });
  await expect(copyRow.getByText("可用")).toBeVisible();
  await page.screenshot({ path: path.join(evidenceDir, "15-customer-copy-real-pdf.png"), fullPage: true });
  seedOversizeReview(stampedReport);
  await expect(copyRow.getByText("待超限审批")).toBeVisible({ timeout: 10_000 });
  await copyRow.getByLabel(/超限批准原因/).fill("真实 E2E 验证 50MB 软上限独立审批");
  await page.screenshot({ path: path.join(evidenceDir, "16-copy-50mb-approval.png"), fullPage: true });
  await copyRow.getByRole("button", { name: "批准" }).click();
  await page
    .getByRole("dialog", { name: "确认批准超限副本" })
    .getByRole("button", { name: "确认批准" })
    .click();
  await expect(page.getByRole("status")).toContainText("超限客户副本已批准");

  await switchUser(page, "admin");
  await openStampPage(page);
  await page.getByLabel("处理规则应用范围").selectOption("reprocess_current");
  await page.screenshot({ path: path.join(evidenceDir, "17-processing-rule-scope-choice.png"), fullPage: true });
  await page.getByRole("button", { name: "发布处理规则版本" }).click();
  await page
    .getByRole("dialog", { name: "确认发布处理规则" })
    .getByRole("button", { name: "确认发布处理规则版本" })
    .click();
  await expect(page.getByRole("status")).toContainText(/已创建 \d+ 个当前报告重处理任务/);
});

test("M-DI 受控规则真实处理缺失和不合格药检单并允许副本失败发货", async ({ page }) => {
  await login(page, "admin");
  await openMenu(page, "入库业务", "入库资料", /药检单审核/);
  await expect(page.getByRole("heading", { name: "药检单审核" })).toBeVisible();
  await page.getByLabel("药检要求商品类别").selectOption("*");
  await page.getByLabel("药检单缺失处理").selectOption("warning");
  const enabled = page.locator("form")
    .filter({ has: page.getByLabel("药检要求商品类别") })
    .getByRole("checkbox");
  if (!(await enabled.isChecked())) await enabled.check();
  await page.getByRole("button", { name: "保存规则" }).click();
  await expect(page.getByRole("status")).toContainText("规则已保存为 v");

  seedReceivingClerk();
  await switchUser(page, "mdi-receiving-clerk");
  const warningReceipt = `ASN-${runTag}-WARNING`;
  const warningBatch = `BATCH-${runTag}-WARNING`;
  const warningOrderId = await createReceivedAsn(page, warningReceipt, [warningBatch], false);
  await openMenu(page, "入库业务", "入库作业", /M2 验收管理/);
  const warningRow = page.locator("tbody tr").filter({ hasText: warningReceipt });
  await expect(warningRow).toBeVisible();
  await warningRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "验收", exact: true }).click();
  const warningDialog = page.getByRole("dialog", { name: "验收" });
  await fillAcceptanceDialog(warningDialog, warningBatch, warningReceipt);
  const warningResponse = page.waitForResponse(
    (candidate) =>
      candidate.url().endsWith(`/api/v1/inbound/receiving-orders/${warningOrderId}/inspect`)
      && candidate.request().method() === "POST",
  );
  await warningDialog.getByRole("button", { name: "提交验收" }).click();
  expect((await warningResponse).status()).toBe(200);
  await expect(page.getByRole("status")).toContainText(warningReceipt);
  expect(readAcceptanceValidationResult(warningOrderId)).toBe("missing_warning");
  await page.screenshot({
    path: path.join(evidenceDir, "13-acceptance-missing-report-warning.png"),
    fullPage: true,
  });

  await switchUser(page, "admin");
  await openMenu(page, "入库业务", "入库资料", /药检单审核/);
  await page.getByLabel("药检单缺失处理").selectOption("block");
  await page.getByRole("button", { name: "保存规则" }).click();
  await expect(page.getByRole("status")).toContainText("规则已保存为 v");
  await page.screenshot({
    path: path.join(evidenceDir, "13-acceptance-rule-enabled.png"),
    fullPage: true,
  });

  const blockedReceipt = `ASN-${runTag}-BLOCK`;
  const blockedBatch = `BATCH-${runTag}-BLOCK`;
  const blockedOrderId = await createReceivedAsn(page, blockedReceipt, [blockedBatch], false);
  await openMenu(page, "入库业务", "入库作业", /M2 验收管理/);
  const row = page.locator("tbody tr").filter({ hasText: blockedReceipt });
  await expect(row).toBeVisible();
  await row.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "验收", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "验收" });
  await fillAcceptanceDialog(dialog, blockedBatch, blockedReceipt);
  const blockedResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith(
        `/api/v1/inbound/receiving-orders/${blockedOrderId}/inspect`,
      )
      && response.request().method() === "POST",
  );
  await dialog.getByRole("button", { name: "提交验收" }).click();
  const response = await blockedResponse;
  expect(response.status()).toBe(422);
  await expect(response.json()).resolves.toMatchObject({ code: "M_DI_REPORT_REQUIRED" });
  await expect(page.getByRole("alert")).toContainText("药检单");
  await page.screenshot({ path: path.join(evidenceDir, "14-acceptance-missing-report-blocked.png"), fullPage: true });
  await dialog.getByRole("button", { name: "取消" }).click();

  const unqualifiedReceipt = `ASN-${runTag}-UNQUALIFIED`;
  const unqualifiedBatch = `BATCH-${runTag}-UNQUALIFIED`;
  const unqualifiedOrderId = await createReceivedAsn(
    page,
    unqualifiedReceipt,
    [unqualifiedBatch],
    false,
  );
  seedUnqualifiedAcceptanceFixture(unqualifiedOrderId, unqualifiedBatch);
  await page.reload();
  await expect(page.getByRole("heading", { name: "M2 验收管理" })).toBeVisible();
  await page
    .getByPlaceholder("ASN / 商品 / 批号 / 单据类型")
    .fill(unqualifiedReceipt);
  await page.getByRole("button", { name: "查询", exact: true }).click();
  const unqualifiedRow = page.locator("tbody tr").filter({ hasText: unqualifiedReceipt });
  await expect(unqualifiedRow).toBeVisible();
  await unqualifiedRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "验收", exact: true }).click();
  const unqualifiedDialog = page.getByRole("dialog", { name: "验收" });
  await fillAcceptanceDialog(unqualifiedDialog, unqualifiedBatch, unqualifiedReceipt);
  const unqualifiedResponse = page.waitForResponse(
    (candidate) =>
      candidate.url().endsWith(
        `/api/v1/inbound/receiving-orders/${unqualifiedOrderId}/inspect`,
      )
      && candidate.request().method() === "POST",
  );
  await unqualifiedDialog.getByRole("button", { name: "提交验收" }).click();
  const unqualifiedResult = await unqualifiedResponse;
  expect(unqualifiedResult.status()).toBe(422);
  await expect(unqualifiedResult.json()).resolves.toMatchObject({
    code: "M_DI_REPORT_UNQUALIFIED",
  });
  await expect(page.getByRole("alert")).toContainText("质量联系单");
  await page.screenshot({
    path: path.join(evidenceDir, "18-unqualified-quality-liaison-created.png"),
    fullPage: true,
  });
  await unqualifiedDialog.getByRole("button", { name: "取消" }).click();

  const liaisonId = readQualityLiaisonId(unqualifiedReceipt);
  await switchUser(page, "mvr-matrix-approver");
  const approval = await approveQualityLiaison(page, liaisonId);
  expect(
    approval.status,
    `质量联系单审批回写失败：${approval.status} ${JSON.stringify(approval.body)}`,
  ).toBe(200);
  expect(approval.body).toMatchObject({ status: "approved" });

  await openMenu(page, "库内业务", "库存管理", /M3 批号管理/);
  await expect(page.getByRole("heading", { name: "M3 批号管理" })).toBeVisible();
  await page.getByLabel("关键字").fill(unqualifiedBatch);
  await page.getByRole("button", { name: "查询", exact: true }).click();
  const inventoryRow = page.locator("tbody tr").filter({ hasText: unqualifiedBatch });
  await expect(inventoryRow).toContainText(/隔离|quarantined/);
  await inventoryRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "详情", exact: true }).click();
  const inventoryDialog = page.getByRole("dialog", { name: "批号详情" });
  await expect(inventoryDialog).toContainText("quality_liaison");
  await expect(inventoryDialog).toContainText(liaisonId);
  await page.screenshot({
    path: path.join(evidenceDir, "19-quality-liaison-inventory-quarantined.png"),
    fullPage: true,
  });
  await page.keyboard.press("Escape");

  seedFailedCopyShippingFixture();
  await switchUser(page, "admin");
  await openMenu(page, "出库业务", "出库作业", /M4 复核发货/);
  await expect(page.getByRole("heading", { name: "M4 复核发货" })).toBeVisible();
  await page.getByLabel("关键字").fill(m4ReviewOrderNo);
  await page.getByRole("button", { name: "查询", exact: true }).click();
  const outboundRow = page.getByRole("row").filter({ hasText: m4ReviewOrderNo }).first();
  await expect(outboundRow).toBeVisible();
  await outboundRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "复核", exact: true }).click();
  const reviewDialog = page.getByRole("dialog", { name: "复核" });
  await expect(reviewDialog).toContainText("P-M4-REVIEW-E2E-001");
  await reviewDialog
    .getByLabel("第二复核员用户 ID")
    .fill("00000000-0000-4000-8000-000000000104");
  const reviewResponse = page.waitForResponse(
    (candidate) =>
      candidate.url().endsWith(`/api/v1/outbound/orders/${m4ReviewOrderId}/review`)
      && candidate.request().method() === "POST",
  );
  await reviewDialog.getByRole("button", { name: "提交复核", exact: true }).click();
  const reviewed = await reviewResponse;
  expect(reviewed.status()).toBe(200);
  await expect(page.getByRole("status")).toContainText(`${m4ReviewOrderNo} 已复核`);

  await page.getByRole("button", { name: "交接", exact: true }).click();
  const shipDialog = page.getByRole("dialog", { name: new RegExp(`发货交接.*${m4ReviewOrderNo}`) });
  await expect(shipDialog).toContainText("生成失败时不阻塞发货");
  // 后端 wave4 校验签字附件真实存在（attachments 表 M4/outbound_handover_signature），先播种。
  seedHandoverSignature();
  // 与 wave4 发货契约对齐：配送方类型 / 快递员姓名 / 包裹数量必填。
  await shipDialog.getByLabel("配送方类型").selectOption("third_party_express");
  await shipDialog.getByLabel("车牌号").fill("沪A-E2E01");
  await shipDialog.getByLabel("快递员姓名").fill("E2E 快递员");
  await shipDialog.getByLabel("快递员电话").fill("13800000000");
  await shipDialog.getByLabel("签字附件 ID").fill("00000000-0000-0000-8000-000000000007");
  await shipDialog.getByLabel("包裹数量").fill("1");
  const shipResponse = page.waitForResponse(
    (candidate) =>
      candidate.url().endsWith(`/api/v1/outbound/orders/${m4ReviewOrderId}/ship`)
      && candidate.request().method() === "POST",
  );
  await shipDialog.getByRole("button", { name: "确认发货", exact: true }).click();
  const shipped = await shipResponse;
  expect(shipped.status()).toBe(200);
  await expect(page.getByRole("status")).toContainText("发货交接已完成");
  await expect(outboundRow).toContainText("已发货");
  expect(readFailedCopyShippingResult()).toBe("shipped|failed");
  await page.screenshot({
    path: path.join(evidenceDir, "20-customer-copy-failed-shipping-allowed.png"),
    fullPage: true,
  });
  await setDefaultRequirementRule(page, false);
});

function seedOversizeReview(reportNo: string) {
  runPsql(
    `UPDATE drug_inspection_customer_copy_jobs AS job
        SET status = 'oversize_review',
            candidate_file_id = version.customer_copy_file_id,
            candidate_hash = version.customer_copy_hash,
            candidate_size = 52428801,
            updated_at = now()
       FROM drug_inspection_report_versions AS version
      WHERE job.report_version_id = version.id
        AND version.report_no = :'report_no'
        AND job.status = 'succeeded';`,
    { report_no: reportNo },
  );
}

function seedUnqualifiedAcceptanceFixture(asnId: string, batchNo: string) {
  runPsql(
    `INSERT INTO quality_liaison_types (
         id, owner_id, type_code, type_name, approval_template_id,
         approver_user_id, timeout_seconds, enabled, created_by
       )
       VALUES (
         md5('mdi-e2e-liaison-type')::uuid, :'owner_id', 'inbound_unqualified',
         '入库不合格', 'TPL-MDI-E2E', :'reviewer_id', 3600, TRUE, :'admin_id'
       )
       ON CONFLICT (owner_id, type_code) DO UPDATE
       SET approver_user_id = EXCLUDED.approver_user_id,
           enabled = TRUE,
           updated_at = now();

       INSERT INTO auth_role_permissions (role_id, permission_id)
       SELECT role.id, permission.id
         FROM auth_roles AS role
         JOIN auth_permissions AS permission
           ON permission.permission_code = 'mql.quality-liaison.approve'
        WHERE role.owner_id = :'owner_id'
          AND role.role_code = 'system_admin'
       ON CONFLICT DO NOTHING;

       INSERT INTO inventory_batches (
         id, owner_id, product_code, batch_no, production_date, expiry_date,
         qty_on_hand, qty_locked, quality_status, location_id, location_code
       )
       VALUES (
         md5('mdi-e2e-inventory:' || :'batch_no')::uuid, :'owner_id',
         'P-M1-E2E-001', :'batch_no', '2026-01-01', '2028-01-01',
         10, 0, 'qualified', :'location_id', 'A01-01-02-03'
       );

       INSERT INTO attachments (
         id, owner_id, module, entity_type, entity_id, file_name,
         content_type, size_bytes, storage_key, sha256, uploaded_by
       )
       VALUES (
         md5('mdi-e2e-attachment:' || :'batch_no')::uuid, :'owner_id',
         'M-DI', 'drug_inspection',
         md5('mdi-e2e-report:' || :'batch_no')::uuid,
         'unqualified-report.pdf', 'application/pdf', 12,
         'mdi-e2e/unqualified/' || :'batch_no' || '.pdf',
         md5('mdi-e2e-unqualified:' || :'batch_no'), :'admin_id'
       );

       INSERT INTO drug_inspection_reports (
         id, owner_id, product_id, batch_no, created_by
       )
       VALUES (
         md5('mdi-e2e-report:' || :'batch_no')::uuid, :'owner_id',
         :'product_id', :'batch_no', :'admin_id'
       );

       INSERT INTO drug_inspection_report_versions (
         id, report_id, owner_id, version_number, report_no,
         original_file_id, original_file_hash, source, processing_mode,
         qualified, status, uploaded_by, submitted_at, reviewed_by, reviewed_at,
         review_result, customer_copy_status
       )
       VALUES (
         md5('mdi-e2e-version:' || :'batch_no')::uuid,
         md5('mdi-e2e-report:' || :'batch_no')::uuid, :'owner_id', 1,
         'REPORT-' || :'batch_no',
         md5('mdi-e2e-attachment:' || :'batch_no')::uuid,
         md5('mdi-e2e-unqualified:' || :'batch_no'),
         'manual_upload', 'none', FALSE, 'confirmed', :'admin_id', now(),
         :'reviewer_id', now(), 'confirmed', 'available'
       );

       UPDATE drug_inspection_reports
          SET current_version_id = md5('mdi-e2e-version:' || :'batch_no')::uuid,
              updated_at = now()
        WHERE id = md5('mdi-e2e-report:' || :'batch_no')::uuid;

       INSERT INTO drug_inspection_asn_links (
         id, owner_id, asn_id, batch_no, report_id,
         source_version_id, source, linked_by
       )
       VALUES (
         md5('mdi-e2e-link:' || :'asn_id')::uuid, :'owner_id', :'asn_id',
         :'batch_no', md5('mdi-e2e-report:' || :'batch_no')::uuid,
         md5('mdi-e2e-version:' || :'batch_no')::uuid, 'uploaded', :'admin_id'
       );`,
    {
      owner_id: ownerId,
      admin_id: adminUserId,
      reviewer_id: reviewUserId,
      product_id: productId,
      location_id: inventoryLocationId,
      asn_id: asnId,
      batch_no: batchNo,
    },
  );
}

function readQualityLiaisonId(receiptNo: string) {
  const liaisonId = runPsql(
    `SELECT id
       FROM quality_liaison_orders
      WHERE owner_id = :'owner_id'
        AND related_document_no = :'receipt_no'
        AND trigger_source = 'm-di.acceptance'
      ORDER BY created_at DESC
      LIMIT 1;`,
    { owner_id: ownerId, receipt_no: receiptNo },
  );
  if (!liaisonId) throw new Error(`未找到 ${receiptNo} 对应的质量联系单`);
  return liaisonId;
}

function seedFailedCopyShippingFixture() {
  runPsql(
    `DELETE FROM outbound_shipments
       WHERE owner_id = :'owner_id'
         AND outbound_order_id = :'order_id';

       INSERT INTO inventory_batches (
         id, owner_id, product_code, batch_no, production_date, expiry_date,
         qty_on_hand, qty_locked, quality_status, location_id, location_code
       )
       VALUES (
         md5('mdi-e2e-shipping-inventory')::uuid, :'owner_id',
         'P-M4-REVIEW-E2E-001', :'batch_no', '2026-01-01', '2028-01-01',
         8, 0, 'qualified', :'location_id', 'A01-01-02-03'
       )
       ON CONFLICT (id) DO UPDATE
       SET qty_on_hand = EXCLUDED.qty_on_hand,
           qty_locked = EXCLUDED.qty_locked,
           quality_status = EXCLUDED.quality_status,
           recall_flag = FALSE,
           updated_at = now();

       INSERT INTO attachments (
         id, owner_id, module, entity_type, entity_id, file_name,
         content_type, size_bytes, storage_key, sha256, uploaded_by
       )
       VALUES (
         md5('mdi-e2e-shipping-attachment')::uuid, :'owner_id',
         'M-DI', 'drug_inspection', md5('mdi-e2e-shipping-report')::uuid,
         'shipping-report.pdf', 'application/pdf', 12,
         'mdi-e2e/shipping/failed-copy.pdf',
         md5('mdi-e2e-shipping-copy-failed'), :'admin_id'
       )
       ON CONFLICT (id) DO UPDATE
       SET storage_key = EXCLUDED.storage_key,
           sha256 = EXCLUDED.sha256;

       INSERT INTO drug_inspection_reports (
         id, owner_id, product_id, batch_no, created_by
       )
       VALUES (
         md5('mdi-e2e-shipping-report')::uuid, :'owner_id',
         '00000000-0000-0000-0000-000000001704', :'batch_no', :'admin_id'
       )
       ON CONFLICT (id) DO UPDATE
       SET product_id = EXCLUDED.product_id,
           batch_no = EXCLUDED.batch_no,
           updated_at = now();

       INSERT INTO drug_inspection_report_versions (
         id, report_id, owner_id, version_number, report_no,
         original_file_id, original_file_hash, source, processing_mode,
         qualified, status, uploaded_by, submitted_at, reviewed_by, reviewed_at,
         review_result, customer_copy_status
       )
       VALUES (
         md5('mdi-e2e-shipping-version')::uuid,
         md5('mdi-e2e-shipping-report')::uuid, :'owner_id', 1,
         'REPORT-M4-COPY-FAILED',
         md5('mdi-e2e-shipping-attachment')::uuid,
         md5('mdi-e2e-shipping-copy-failed'),
         'manual_upload', 'none', TRUE, 'confirmed', :'admin_id', now(),
         :'reviewer_id', now(), 'confirmed', 'failed'
       )
       ON CONFLICT (id) DO UPDATE
       SET status = EXCLUDED.status,
           qualified = EXCLUDED.qualified,
           reviewed_by = EXCLUDED.reviewed_by,
           reviewed_at = EXCLUDED.reviewed_at,
           review_result = EXCLUDED.review_result,
           customer_copy_status = EXCLUDED.customer_copy_status,
           updated_at = now();

       UPDATE drug_inspection_reports
          SET current_version_id = md5('mdi-e2e-shipping-version')::uuid,
              updated_at = now()
        WHERE id = md5('mdi-e2e-shipping-report')::uuid;`,
    {
      owner_id: ownerId,
      admin_id: adminUserId,
      reviewer_id: reviewUserId,
      location_id: inventoryLocationId,
      batch_no: m4ReviewBatch,
      order_id: m4ReviewOrderId,
    },
  );
}

function seedReceivingClerk() {
  runPsql(
    `INSERT INTO auth_users (
         id, username, display_name, password_hash, status
       )
       SELECT
         md5('mdi-e2e-receiving-clerk')::uuid,
         'mdi-receiving-clerk',
         '药检真实 E2E 验收员',
         password_hash,
         'active'
       FROM auth_users
       WHERE id = :'admin_id'
       ON CONFLICT (id) DO UPDATE
       SET password_hash = EXCLUDED.password_hash,
           status = 'active',
           updated_at = now();

       INSERT INTO auth_user_owner_bindings (
         user_id, owner_id, is_active, is_primary
       )
       VALUES (
         md5('mdi-e2e-receiving-clerk')::uuid, :'owner_id', TRUE, FALSE
       )
       ON CONFLICT (user_id, owner_id) DO UPDATE SET is_active = TRUE;

       INSERT INTO auth_roles (
         id, owner_id, role_code, role_name
       )
       VALUES (
         md5('mdi-e2e-receiving-clerk-role')::uuid, :'owner_id',
         'receiving_clerk', '药检真实 E2E 验收岗'
       )
       ON CONFLICT (owner_id, lower(role_code)) DO UPDATE
       SET role_name = EXCLUDED.role_name;

       INSERT INTO auth_role_permissions (role_id, permission_id)
       SELECT role.id, permission.id
         FROM auth_roles AS role
         JOIN auth_permissions AS permission
           ON permission.permission_code = 'm2.write'
        WHERE role.owner_id = :'owner_id'
          AND role.role_code = 'receiving_clerk'
       ON CONFLICT DO NOTHING;

       -- 验收员需 h1.auth.me 才能看到工作台入口；缺它时前端 fail-closed 不渲染侧边栏。
       INSERT INTO auth_role_permissions (role_id, permission_id)
       SELECT role.id, permission.id
         FROM auth_roles AS role
         JOIN auth_permissions AS permission
           ON permission.permission_code = 'h1.auth.me'
        WHERE role.owner_id = :'owner_id'
          AND role.role_code = 'receiving_clerk'
       ON CONFLICT DO NOTHING;

       INSERT INTO auth_user_roles (user_id, owner_id, role_id)
       SELECT md5('mdi-e2e-receiving-clerk')::uuid, :'owner_id', role.id
         FROM auth_roles AS role
        WHERE role.owner_id = :'owner_id'
          AND role.role_code = 'receiving_clerk'
       ON CONFLICT DO NOTHING;`,
    { owner_id: ownerId, admin_id: adminUserId },
  );
}

function seedHandoverSignature() {
  runPsql(
    `INSERT INTO attachments (
        id, owner_id, module, entity_type, entity_id, file_name,
        content_type, size_bytes, storage_key, sha256, uploaded_by
      )
      VALUES (
        '00000000-0000-0000-8000-000000000007', :'owner_id',
        'M4', 'outbound_handover_signature', :'order_id',
        'handover-signature.png', 'image/png', 100,
        'e2e/handover-signature.png', 'e2e', :'admin_id'
      )
      ON CONFLICT (id) DO NOTHING;`,
    { owner_id: ownerId, admin_id: adminUserId, order_id: m4ReviewOrderId },
  );
}

function readFailedCopyShippingResult() {
  return runPsql(
    `SELECT outbound.status || '|' || version.customer_copy_status
       FROM outbound_orders AS outbound
       JOIN outbound_order_lines AS line
         ON line.outbound_order_id = outbound.id
       JOIN products AS product
         ON product.owner_id = outbound.owner_id
        AND product.product_code = line.product_code
       JOIN drug_inspection_reports AS report
         ON report.owner_id = outbound.owner_id
        AND report.product_id = product.id
        AND report.batch_no = line.batch_no
       JOIN drug_inspection_report_versions AS version
         ON version.id = report.current_version_id
      WHERE outbound.id = :'order_id';`,
    { order_id: m4ReviewOrderId },
  );
}

function readAcceptanceValidationResult(receivingOrderId: string) {
  return runPsql(
    `SELECT result
       FROM drug_inspection_acceptance_validations
      WHERE owner_id = :'owner_id'
        AND receiving_order_id = :'receiving_order_id'
      ORDER BY validated_at DESC
      LIMIT 1;`,
    { owner_id: ownerId, receiving_order_id: receivingOrderId },
  );
}

function runPsql(sql: string, variables: Record<string, string> = {}) {
  const databaseUrl = process.env.DATABASE_URL ?? process.env.WMS_DB_URL;
  if (!databaseUrl) throw new Error("M-DI real E2E fixture requires DATABASE_URL");
  const connection = new URL(databaseUrl);
  return execFileSync("psql", [
    "--host",
    connection.hostname,
    "--port",
    connection.port || "5432",
    "--username",
    decodeURIComponent(connection.username),
    "--dbname",
    connection.pathname.replace(/^\//, ""),
    "--quiet",
    "--set=ON_ERROR_STOP=1",
    "--tuples-only",
    "--no-align",
    ...Object.entries(variables).map(([key, value]) => `--set=${key}=${value}`),
    "--file=-",
  ], {
    input: sql,
    encoding: "utf8",
    env: {
      ...process.env,
      PGPASSWORD: decodeURIComponent(connection.password),
    },
    stdio: ["pipe", "pipe", "pipe"],
  }).trim();
}
