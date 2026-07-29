import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const evidenceDir = path.resolve("../artifacts/screenshot-portal/real-web/h9-delivery-note-aggregation");
const orderNo = "OUT-H9-E2E-006";

test.use({ viewport: { width: 1600, height: 900 } });

test("US-H9-006 真实线路、计划与随货同行单归集", async ({ browser, page }) => {
  fs.mkdirSync(evidenceDir, { recursive: true });
  await login(page);
  await openDeliveryNoteAggregation(page);

  const candidateRow = page.getByRole("row").filter({ hasText: orderNo });
  await expect(candidateRow).toContainText("ERP-H9-E2E-006");
  await expect(candidateRow).toContainText("E2E 客户门店");
  await expect(candidateRow).toContainText("真实数据路 006 号");
  await expect(candidateRow).toContainText("LINE-H9-E2E-006");
  await page.screenshot({ path: path.join(evidenceDir, "pending-orders.png"), fullPage: false });

  await candidateRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "人工截单", exact: true }).click();
  const cutoffDialog = page.getByRole("dialog", { name: "授权人工截单" });
  await cutoffDialog.getByLabel("截单原因").fill("真实 E2E 装车前授权截单");
  const cutoffResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/print-orchestration/delivery-note-groups/manual-cutoff")
      && response.request().method() === "POST",
  );
  await cutoffDialog.getByRole("button", { name: "确认截单" }).click();
  const cutoff = await cutoffResponse;
  expect(cutoff.ok(), await cutoff.text()).toBeTruthy();
  const group = await cutoff.json() as { delivery_note_no: string };
  await expect(page.getByRole("status")).toContainText(group.delivery_note_no);
  await page.getByRole("tab", { name: /截单结果/ }).click();
  const groupRow = page.getByRole("row").filter({ hasText: group.delivery_note_no });
  await expect(groupRow).toContainText(orderNo);
  await expect(groupRow).toContainText("人工截单");
  await expect(groupRow).toContainText("真实 E2E 装车前授权截单");
  await page.screenshot({ path: path.join(evidenceDir, "cutoff-result.png"), fullPage: false });

  await page.getByRole("tab", { name: /截单计划/ }).click();
  const seededPlan = page.getByRole("row").filter({ hasText: "E2E 客户截单计划" });
  await expect(seededPlan).toContainText("客户");
  await expect(seededPlan).toContainText("周一 17:00");
  await expect(seededPlan).toContainText("2026-08-01 12:00");
  await page.getByRole("button", { name: "新建计划", exact: true }).click();
  const planDialog = page.getByRole("dialog", { name: "新建截单计划" });
  const planName = `E2E 线路截单计划 ${Date.now()}`;
  await planDialog.getByLabel("计划名称").fill(planName);
  await planDialog.getByLabel("适用层级").selectOption("route");
  await planDialog.getByLabel("线路编码").fill("LINE-H9-E2E-006");
  const createPlanResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/print-orchestration/cutoff-plans")
      && response.request().method() === "POST",
  );
  await planDialog.getByRole("button", { name: "保存草稿" }).click();
  expect((await createPlanResponse).ok()).toBeTruthy();
  const planRow = page.getByRole("row").filter({ hasText: planName });
  await expect(planRow).toContainText("草稿");
  await planRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "发布计划", exact: true }).click();
  const publishPlanDialog = page.getByRole("dialog", { name: "发布截单计划" });
  await expect(publishPlanDialog).toContainText(planName);
  const publishPlanResponse = page.waitForResponse(
    (response) =>
      response.url().includes("/api/v1/print-orchestration/cutoff-plans/")
      && response.url().endsWith("/publish")
      && response.request().method() === "POST",
  );
  await publishPlanDialog.getByRole("button", { name: "确认发布" }).click();
  expect((await publishPlanResponse).ok()).toBeTruthy();
  await expect(planRow).toContainText("已发布");
  await page.screenshot({ path: path.join(evidenceDir, "cutoff-plans.png"), fullPage: false });

  await page.getByRole("tab", { name: /线路绑定/ }).click();
  const seededRoute = page.getByRole("row").filter({ hasText: "LINE-H9-E2E-006" });
  await expect(seededRoute).toContainText("E2E 客户门店");
  await expect(seededRoute).toContainText("上海市上海市浦东新区真实数据路 006 号");
  await page.getByRole("button", { name: "发布线路", exact: true }).click();
  const routeDialog = page.getByRole("dialog", { name: "发布送货地址线路" });
  await routeDialog.getByLabel("线路编码").fill("LINE-H9-E2E-NEXT");
  await routeDialog.getByLabel("生效时间").fill("2100-01-02T00:00");
  const publishRouteResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/print-orchestration/route-bindings")
      && response.request().method() === "POST",
  );
  await routeDialog.getByRole("button", { name: "发布线路" }).click();
  expect((await publishRouteResponse).ok()).toBeTruthy();
  await expect(page.getByRole("row").filter({ hasText: "LINE-H9-E2E-NEXT" })).toBeVisible();
  await page.screenshot({ path: path.join(evidenceDir, "plans-and-routes.png"), fullPage: false });

  const refreshedContext = await browser.newContext({ viewport: { width: 1600, height: 900 } });
  const refreshedPage = await refreshedContext.newPage();
  await login(refreshedPage);
  await openDeliveryNoteAggregation(refreshedPage);
  await refreshedPage.getByRole("tab", { name: /线路绑定/ }).click();
  await expect(refreshedPage.getByRole("row").filter({ hasText: "LINE-H9-E2E-NEXT" })).toBeVisible();
  await refreshedContext.close();
});

test("US-H9-007 归集维度规则配置：草稿、样本测试、发布与停用", async ({ page }) => {
  const ruleEvidenceDir = path.resolve("../artifacts/screenshot-portal/real-web/h9-aggregation-rules");
  fs.mkdirSync(ruleEvidenceDir, { recursive: true });
  await login(page);
  await openDeliveryNoteAggregation(page);
  await page.getByRole("tab", { name: /归集规则/ }).click();
  await expect(
    page.getByText("维度只能从已登记订单标准字段中等值归组；货主 + 仓库 + 送货地址是不可覆盖的硬边界，规则不能跨地址归集。"),
  ).toBeVisible();

  // 创建草稿：仅能从已登记字段目录中挑选，方式固定等值
  await page.getByRole("button", { name: "新建规则版本", exact: true }).click();
  const ruleDialog = page.getByRole("dialog", { name: "新建归集规则版本" });
  const ruleName = `E2E 发票归集规则 ${Date.now()}`;
  await ruleDialog.getByLabel("规则名称").fill(ruleName);
  await ruleDialog.getByLabel("添加维度（等值归组）").selectOption("invoice_no");
  await expect(ruleDialog.getByText("发票号 · 等值")).toBeVisible();
  const createRuleResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/print-orchestration/aggregation-rules/versions")
      && response.request().method() === "POST",
  );
  await ruleDialog.getByRole("button", { name: "保存草稿" }).click();
  const created = await createRuleResponse;
  expect(created.ok(), await created.text()).toBeTruthy();
  const rule = await created.json() as { id: string; version_no: number };
  const ruleRow = page.getByRole("row").filter({ hasText: ruleName });
  await expect(ruleRow).toContainText("草稿");
  await expect(ruleRow).toContainText("发票号");

  // 样本订单测试：展示命中规则、分组键与预计归集结果（AC4）
  await ruleRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "测试规则", exact: true }).click();
  const testDialog = page.getByRole("dialog", { name: "样本订单测试" });
  await testDialog.getByRole("checkbox").first().waitFor();
  await testDialog.locator("label").filter({ hasText: "OUT-H9-E2E-007" }).getByRole("checkbox").check();
  await testDialog.locator("label").filter({ hasText: "OUT-H9-E2E-008" }).getByRole("checkbox").check();
  const testResponse = page.waitForResponse(
    (response) => response.url().endsWith(`/aggregation-rules/versions/${rule.id}/test`),
  );
  await testDialog.getByRole("button", { name: "执行测试" }).click();
  expect((await testResponse).ok()).toBeTruthy();
  await expect(testDialog.getByText(/预计归集结果（2 组）/)).toBeVisible();
  await expect(testDialog.getByText("发票号=INV-H9-E2E-007")).toBeVisible();
  await expect(testDialog.getByText("发票号=INV-H9-E2E-008")).toBeVisible();
  await page.screenshot({ path: path.join(ruleEvidenceDir, "rule-test-preview.png"), fullPage: false });
  await testDialog.getByRole("button", { name: "关闭" }).click();
  await expect(ruleRow).toContainText("已测试");

  // 发布（AC3/AC6）：仅已测试版本可发布
  await ruleRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "发布规则", exact: true }).click();
  const publishRuleDialog = page.getByRole("dialog", { name: "发布归集规则" });
  await expect(publishRuleDialog).toContainText(ruleName);
  const publishResponse = page.waitForResponse(
    (response) => response.url().endsWith(`/aggregation-rules/versions/${rule.id}/publish`),
  );
  await publishRuleDialog.getByRole("button", { name: "确认发布" }).click();
  expect((await publishResponse).ok()).toBeTruthy();
  await expect(ruleRow).toContainText("已发布");
  await page.screenshot({ path: path.join(ruleEvidenceDir, "rule-published.png"), fullPage: false });

  // 停用，恢复仅按硬边界归集，也保证套件可重复执行
  await ruleRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "停用规则", exact: true }).click();
  const disableRuleDialog = page.getByRole("dialog", { name: "停用归集规则" });
  await expect(disableRuleDialog).toContainText(ruleName);
  const disableResponse = page.waitForResponse(
    (response) => response.url().endsWith(`/aggregation-rules/versions/${rule.id}/disable`),
  );
  await disableRuleDialog.getByRole("button", { name: "确认停用" }).click();
  expect((await disableResponse).ok()).toBeTruthy();
  await expect(ruleRow).toContainText("已停用");
});

test("US-H9-008/009 打印组套冻结、分类 PDF 渲染留存与选择打印", async ({ page }) => {
  const suiteEvidenceDir = path.resolve("../artifacts/screenshot-portal/real-web/h9-print-suites");
  const pdfEvidenceDir = path.resolve("../artifacts/screenshot-portal/real-web/h9-category-pdfs");
  fs.mkdirSync(suiteEvidenceDir, { recursive: true });
  fs.mkdirSync(pdfEvidenceDir, { recursive: true });
  await login(page);
  await openDeliveryNoteAggregation(page);
  await page.getByRole("tab", { name: "打印组套" }).click();
  await expect(
    page.getByText("组套匹配顺序固定为送货地址、客户、线路、货主 + 仓库默认；已发布版本不可改写，"),
  ).toBeVisible();

  // AC3/AC4：分类来自 M1 字典；rendered 绑定已发布模板版本，external_file 绑定 H-FILE 引用
  await page.getByRole("button", { name: "新建组套版本", exact: true }).click();
  const createDialog = page.getByRole("dialog", { name: "新建打印组套版本" });
  const suiteName = `E2E 打印组套 ${Date.now()}`;
  await createDialog.getByLabel("组套名称").fill(suiteName);
  // 适用层级默认“客户”，客户下拉默认选中唯一的 E2E 客户门店（C-M1-E2E-001）。
  await createDialog.getByLabel("生效时间").fill("2026-01-01T00:00");
  await createDialog.getByLabel("添加打印项分类").selectOption("delivery_note");
  await createDialog
    .getByLabel("模板版本（第 1 项）")
    .selectOption({ label: "M4 随货同行单 E2E 模板 V1" });
  await createDialog.getByLabel("添加打印项分类").selectOption("invoice");
  await expect(createDialog.getByLabel("H-FILE 文件引用（第 2 项）")).toHaveValue("h-file:invoice");
  const createResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/print-orchestration/print-suites/versions")
      && response.request().method() === "POST",
  );
  await createDialog.getByRole("button", { name: "保存草稿" }).click();
  const created = await createResponse;
  expect(created.ok(), await created.text()).toBeTruthy();
  const suite = await created.json() as { id: string; version_no: number };
  const suiteRow = page.getByRole("row").filter({ hasText: suiteName });
  await expect(suiteRow).toContainText("草稿");
  await expect(suiteRow).toContainText("随货同行单");
  await expect(suiteRow).toContainText("发票");

  // AC5/AC9：真实样本归集组的就绪性/完整性预检与解析层级展示
  await suiteRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "测试组套", exact: true }).click();
  const testDialog = page.getByRole("dialog", { name: "样本归集组测试" });
  await testDialog
    .locator("label")
    .filter({ hasText: "SHTX-E2E-H9-008-0001" })
    .getByRole("checkbox")
    .check();
  const testResponse = page.waitForResponse(
    (response) => response.url().endsWith(`/print-suites/versions/${suite.id}/test`),
  );
  await testDialog.getByRole("button", { name: "执行测试" }).click();
  expect((await testResponse).ok()).toBeTruthy();
  await expect(testDialog.getByText(/解析层级：客户（命中本版本）/)).toBeVisible();
  await expect(testDialog.getByText(/随货同行单（渲染，必需）：就绪/)).toBeVisible();
  await expect(testDialog.getByText(/发票（外部文件，必需）：就绪，绑定 1 个权威文件/)).toBeVisible();
  await page.screenshot({ path: path.join(suiteEvidenceDir, "suite-test-readiness.png"), fullPage: false });
  await testDialog.getByRole("button", { name: "关闭" }).click();
  await expect(suiteRow).toContainText("已测试");

  // AC2/AC9：仅已测试版本可发布
  await suiteRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "发布组套", exact: true }).click();
  const publishSuiteDialog = page.getByRole("dialog", { name: "发布打印组套" });
  await expect(publishSuiteDialog).toContainText(suiteName);
  const publishResponse = page.waitForResponse(
    (response) => response.url().endsWith(`/print-suites/versions/${suite.id}/publish`),
  );
  await publishSuiteDialog.getByRole("button", { name: "确认发布" }).click();
  expect((await publishResponse).ok()).toBeTruthy();
  await expect(suiteRow).toContainText("已发布");
  await page.screenshot({ path: path.join(suiteEvidenceDir, "suite-published.png"), fullPage: false });

  // AC1/AC7/AC8：截单后按解析结果创建冻结组套实例（客户级命中）
  await page.getByRole("tab", { name: /待截单订单/ }).click();
  const candidateRow = page.getByRole("row").filter({ hasText: "OUT-H9-E2E-010" });
  await candidateRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "人工截单", exact: true }).click();
  const cutoffDialog = page.getByRole("dialog", { name: "授权人工截单" });
  await cutoffDialog.getByLabel("截单原因").fill("US-H9-008 组套实例真实验证");
  const cutoffResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/print-orchestration/delivery-note-groups/manual-cutoff")
      && response.request().method() === "POST",
  );
  await cutoffDialog.getByRole("button", { name: "确认截单" }).click();
  const cutoff = await cutoffResponse;
  expect(cutoff.ok(), await cutoff.text()).toBeTruthy();
  const group = await cutoff.json() as { delivery_note_no: string };
  await page.getByRole("tab", { name: "打印组套" }).click();
  const instanceRow = page.getByRole("row").filter({ hasText: group.delivery_note_no });
  await expect(instanceRow).toContainText(`V${suite.version_no}`);
  await expect(instanceRow).toContainText("等待分类 PDF");
  await expect(instanceRow).toContainText("1.delivery_note✓ → 2.invoice✓");
  await page.screenshot({ path: path.join(suiteEvidenceDir, "suite-instance.png"), fullPage: false });

  // US-H9-009 AC1/AC2/AC4/AC5：源单据已就绪仍先等待分类 PDF；服务端准备成功才进入待打印。
  const pdfPanel = page.getByRole("region", { name: "分类 PDF 生成与留存" });
  const instanceOption = pdfPanel
    .getByLabel("组套实例")
    .locator("option")
    .filter({ hasText: group.delivery_note_no });
  const instanceId = await instanceOption.getAttribute("value");
  expect(instanceId).toBeTruthy();
  await pdfPanel.getByLabel("组套实例").selectOption(instanceId as string);
  const actualPrepareResponse = page.waitForResponse(
    (response) =>
      response.url().includes("/api/v1/print-orchestration/suite-instances/")
      && response.url().endsWith("/category-pdfs/prepare")
      && response.request().method() === "POST",
  );
  await pdfPanel.getByRole("button", { name: "生成分类 PDF", exact: true }).click();
  const prepared = await actualPrepareResponse;
  expect(prepared.ok(), await prepared.text()).toBeTruthy();
  await expect(instanceRow).toContainText("待打印");
  const deliveryPdfRow = pdfPanel.getByRole("row").filter({ hasText: "随货同行单" });
  const invoicePdfRow = pdfPanel.getByRole("row").filter({ hasText: "发票" });
  await expect(deliveryPdfRow).toContainText("服务端渲染");
  await expect(deliveryPdfRow).toContainText("GSP 五年归档");
  await expect(deliveryPdfRow).toContainText("已就绪");
  await expect(invoicePdfRow).toContainText("权威外部 PDF");
  await expect(invoicePdfRow).toContainText("短期缓存");
  await expect(invoicePdfRow).toContainText("已就绪");
  await expect(invoicePdfRow).toContainText("不适用（权威外部 PDF）");
  await pdfPanel.scrollIntoViewIfNeeded();
  await page.screenshot({ path: path.join(pdfEvidenceDir, "category-pdfs-ready.png"), fullPage: false });

  // AC3/AC6：只选发票下载；响应为真实 PDF，文件读取由独立权限控制并写审计。
  await invoicePdfRow.getByRole("checkbox", { name: "选择此行" }).check();
  await expect(pdfPanel.getByRole("button", { name: "下载所选分类" })).toBeEnabled();
  await page.screenshot({ path: path.join(pdfEvidenceDir, "category-pdfs-selection.png"), fullPage: false });
  const selectedPdfResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/category-pdfs/download")
      && response.request().method() === "POST",
  );
  const browserDownload = page.waitForEvent("download");
  await pdfPanel.getByRole("button", { name: "下载所选分类" }).click();
  const selectedPdf = await selectedPdfResponse;
  expect(selectedPdf.ok()).toBeTruthy();
  expect(selectedPdf.headers()["content-type"]).toContain("application/pdf");
  const downloaded = await browserDownload;
  const downloadedPath = await downloaded.path();
  expect(downloadedPath).toBeTruthy();
  expect(fs.readFileSync(downloadedPath as string).subarray(0, 5).toString()).toBe("%PDF-");

  // BUSINESS-CONTENT：浏览器下载的 rendered 分类必须携带同一真实出库单业务键，
  // 不能只证明接口返回了一个 PDF 文件头。
  await invoicePdfRow.getByRole("checkbox", { name: "选择此行" }).uncheck();
  await deliveryPdfRow.getByRole("checkbox", { name: "选择此行" }).check();
  const renderedDownload = page.waitForEvent("download");
  await pdfPanel.getByRole("button", { name: "下载所选分类" }).click();
  const renderedPath = await (await renderedDownload).path();
  expect(renderedPath).toBeTruthy();
  const renderedBytes = fs.readFileSync(renderedPath as string);
  expect(renderedBytes.subarray(0, 5).toString()).toBe("%PDF-");
  expect(renderedBytes.length).toBeGreaterThan(5_000);
  expect(renderedBytes.toString("latin1")).toContain("/Subtype /Image");
  const jpegStart = renderedBytes.indexOf(Buffer.from([0xff, 0xd8]));
  const jpegEnd = renderedBytes.indexOf(Buffer.from([0xff, 0xd9]), jpegStart);
  expect(jpegStart).toBeGreaterThanOrEqual(0);
  expect(jpegEnd).toBeGreaterThan(jpegStart);
  const renderedPage = await page.context().newPage();
  await renderedPage.setContent(
    `<img alt="真实分类 PDF 页面" src="data:image/jpeg;base64,${
      renderedBytes.subarray(jpegStart, jpegEnd + 2).toString("base64")
    }">`,
  );
  const renderedImage = renderedPage.getByRole("img", { name: "真实分类 PDF 页面" });
  await expect(renderedImage).toBeVisible();
  await renderedImage.screenshot({
    path: path.join(pdfEvidenceDir, "category-pdf-rendered-document.png"),
  });
  await renderedPage.close();

  const emergencyPdfResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/category-pdfs/emergency-print")
      && response.request().method() === "POST",
  );
  const emergencyPopup = page.waitForEvent("popup");
  await pdfPanel.getByRole("button", { name: "应急打印所选" }).click();
  expect((await emergencyPdfResponse).ok()).toBeTruthy();
  await (await emergencyPopup).close();

  // 停用组套，保证套件可重复执行；既有实例快照不受影响
  const publishedRow = page.getByRole("row").filter({ hasText: suiteName });
  await publishedRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "停用组套", exact: true }).click();
  const disableSuiteDialog = page.getByRole("dialog", { name: "停用打印组套" });
  await expect(disableSuiteDialog).toContainText(suiteName);
  const disableResponse = page.waitForResponse(
    (response) => response.url().endsWith(`/print-suites/versions/${suite.id}/disable`),
  );
  await disableSuiteDialog.getByRole("button", { name: "确认停用" }).click();
  expect((await disableResponse).ok()).toBeTruthy();
  await expect(publishedRow).toContainText("已停用");
  await expect(instanceRow).toContainText(`V${suite.version_no}`);
});

async function login(page: Page) {
  await page.goto("/");
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function openDeliveryNoteAggregation(page: Page) {
  await page.getByRole("button", { name: "基础能力", exact: true }).click();
  await page.getByRole("button", { name: "H9 打印能力", exact: true }).click();
  await page.getByRole("button", { name: /作业·随货同行单归集/ }).click();
  await expect(page.getByRole("heading", { name: "作业·随货同行单归集" })).toBeVisible();
}
