import { expect, type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

export const evidenceDir = path.resolve("../artifacts/screenshot-portal/real-web/m-di");
export const supplierId = "00000000-0000-0000-0000-000000001101";
export const warehouseId = "00000000-0000-0000-0000-000000001301";
export const productId = "00000000-0000-0000-0000-000000001001";
export const runTag = `MDI-${Date.now()}`;
export const firstReceipt = `ASN-${runTag}-01`;
export const secondReceipt = `ASN-${runTag}-02`;
export const batchA = `BATCH-${runTag}-A`;
export const batchB = `BATCH-${runTag}-B`;
export const firstReport = `REPORT-${runTag}-V1`;
export const editedReport = `REPORT-${runTag}-EDIT`;
export const correctedReport = `REPORT-${runTag}-V2`;
export const secondBatchReport = `REPORT-${runTag}-B`;
export const stampedReport = `REPORT-${runTag}-STAMPED`;
export const reportPng = path.join(evidenceDir, `${runTag}-report.png`);
export const replacementPng = path.join(evidenceDir, `${runTag}-replacement.png`);
export const upstreamPdf = path.join(evidenceDir, `${runTag}-upstream.pdf`);
export const stampPng = path.join(evidenceDir, `${runTag}-stamp.png`);
export const ownerId = "00000000-0000-0000-0000-000000000001";
export const adminUserId = "00000000-0000-0000-0000-000000000101";
export const reviewUserId = "00000000-0000-0000-0000-000000000103";
export const inventoryLocationId = "00000000-0000-0000-0000-000000001401";
export const m4ReviewOrderId = "00000000-0000-0000-0000-000000001702";
export const m4ReviewOrderNo = "OUT-M4-REVIEW-E2E-001";
export const m4ReviewBatch = "B-M4-REVIEW-E2E-001";

export async function prepareFiles(page: Page) {
  fs.mkdirSync(evidenceDir, { recursive: true });
  await page.setViewportSize({ width: 900, height: 1200 });
  await page.setContent(
    reportSvg("药品检验报告", "原始报告 · 彩色底纹与轻微噪点"),
  );
  await page.locator("svg").screenshot({ path: reportPng });
  await page.setContent(
    reportSvg("药品检验报告（更正版）", "更清晰的供应商补发版本"),
  );
  await page.locator("svg").screenshot({ path: replacementPng });
  fs.writeFileSync(
    upstreamPdf,
    Buffer.from("%PDF-1.4\n上游随货同行单真实 E2E\n%%EOF"),
  );
  await page.setViewportSize({ width: 1440, height: 1000 });
}

function reportSvg(title: string, subtitle: string) {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="800" height="1100">
    <rect width="800" height="1100" fill="#fffdf5"/>
    <rect x="42" y="42" width="716" height="1016" fill="none" stroke="#334155" stroke-width="3"/>
    <text x="400" y="118" text-anchor="middle" font-size="38" font-weight="700" fill="#172554">${title}</text>
    <text x="400" y="164" text-anchor="middle" font-size="20" fill="#475569">${subtitle}</text>
    <g font-size="22" fill="#1e293b">
      <text x="80" y="245">报告编号：${runTag}</text>
      <text x="80" y="295">商品：E2E 药品</text>
      <text x="80" y="345">检验结论：符合规定</text>
    </g>
    <g stroke="#94a3b8" stroke-width="2">
      ${Array.from({ length: 11 }, (_, index) => `<line x1="80" y1="${410 + index * 48}" x2="720" y2="${410 + index * 48}"/>`).join("")}
    </g>
    <circle cx="690" cy="940" r="58" fill="#ef444422" stroke="#dc2626" stroke-width="5"/>
    <text x="690" y="948" text-anchor="middle" font-size="18" fill="#b91c1c">供应商原章</text>
  </svg>`;
}

export async function createReceivedAsn(
  page: Page,
  receiptNo: string,
  batches: string[],
  inspectBatches = true,
) {
  return page.evaluate(
    async ({
      receiptNo,
      batches,
      supplierId,
      warehouseId,
      productId,
      inspectBatches,
    }) => {
      const session = JSON.parse(
        localStorage.getItem("wms.web-admin.auth-session") ?? "null",
      ) as { accessToken?: string } | null;
      const authorization = `Bearer ${session?.accessToken ?? ""}`;
      const call = async (url: string, body: unknown, key: string) => {
        const response = await fetch(url, {
          method: "POST",
          headers: {
            Authorization: authorization,
            "Content-Type": "application/json",
            "Idempotency-Key": key,
          },
          body: JSON.stringify(body),
        });
        const payload = await response.json();
        if (!response.ok) {
          throw new Error(
            `${url} ${response.status}: ${JSON.stringify(payload)}`,
          );
        }
        return payload as { id: string };
      };
      const tomorrow = new Date(
        Date.now() + 24 * 60 * 60 * 1000,
      ).toISOString();
      const created = await call(
        "/api/v1/inbound/receiving-orders",
        {
          receipt_no: receiptNo,
          document_type: "purchase_inbound",
          supplier_id: supplierId,
          warehouse_id: warehouseId,
          external_ref: `PO-${receiptNo}`,
          expected_arrival_at: tomorrow,
          lines: batches.map((_, index) => ({
            line_no: index + 1,
            product_id: productId,
            product_code: "P-M1-E2E-001",
            expected_qty: 10,
            batch_no: null,
            production_date: null,
            expiry_date: null,
          })),
        },
        `mdi-create-${receiptNo}`,
      );
      await call(
        `/api/v1/inbound/receiving-orders/${created.id}/release`,
        {},
        `mdi-release-${receiptNo}`,
      );
      await call(
        `/api/v1/inbound/receiving-orders/${created.id}/receive`,
        {
          actual_qty: batches.length * 10,
          shortage_qty: 0,
          rejected_qty: 0,
          arrival_temperature_celsius: 5,
          exception_note: null,
          details: {
            temperature_control_method: "冷链运输",
            vehicle_no: "沪A-MDI01",
            origin: "上游医药仓",
            departure_at: new Date(
              Date.now() - 3 * 60 * 60 * 1000,
            ).toISOString(),
            arrival_at: new Date(
              Date.now() - 30 * 60 * 1000,
            ).toISOString(),
            storage_at: new Date().toISOString(),
            transport_mode: "冷藏车",
            carrier: "E2E 医药物流",
            contact_name: "张送货",
            contact_phone: "13800000000",
            contact_id_no: "310101199001011234",
            seal_checked: "通过",
            filing_checked: "通过",
          },
        },
        `mdi-receive-${receiptNo}`,
      );
      if (inspectBatches) {
        for (const [index, batchNo] of batches.entries()) {
          await call(
            `/api/v1/inbound/receiving-orders/${created.id}/inspect`,
            {
              batch_no: batchNo,
              accepted_qty: 10,
              rejected_qty: 0,
              production_date: "2026-01-01",
              expiry_date: "2028-01-01",
              quality_status: "qualified",
              trace_codes: [`TRACE-${receiptNo}-${index + 1}`],
              appearance_check: "通过",
              package_check: "通过",
              instruction_check: "通过",
              label_check: "通过",
              sampling_qty: 1,
              approval_no: "国药准字E2E001",
            },
            `mdi-inspect-${receiptNo}-${index + 1}`,
          );
        }
      }
      return created.id;
    },
    {
      receiptNo,
      batches,
      supplierId,
      warehouseId,
      productId,
      inspectBatches,
    },
  );
}

export async function uploadReport(
  page: Page,
  receiptNo: string,
  input: {
    batchNo: string;
    reportNo: string;
    filePath: string;
    processingMode: "none" | "color_enhance" | "black_white_enhance";
    reason?: string;
    previewEvidencePath?: string;
  },
) {
  const row = page.locator("tbody tr").filter({ hasText: receiptNo });
  await row.getByRole("button", { name: "录入资料" }).click();
  const dialog = page.getByRole("dialog", { name: new RegExp(receiptNo) });
  await dialog.getByLabel("批号").selectOption(input.batchNo);
  await dialog.getByLabel("图片处理方式").selectOption(input.processingMode);
  await dialog.getByLabel("报告编号").fill(input.reportNo);
  await dialog.getByLabel("药检单文件").setInputFiles(input.filePath);
  await expect(dialog.getByRole("img", { name: "权威原图" })).toBeVisible();
  await expect(dialog.getByRole("img", { name: /处理后预览/ })).toBeVisible();
  if (input.previewEvidencePath) {
    await page.screenshot({ path: input.previewEvidencePath, fullPage: true });
  }
  if (input.reason) await dialog.getByLabel("修改原因").fill(input.reason);
  await dialog.getByRole("button", { name: "保存药检单" }).click();
  await expect(dialog.getByRole("status")).toContainText("上传并提交审核");
}

export async function openDocuments(page: Page, keyword: string) {
  // 菜单已归并：入库业务 → 入库资料 → 入库资料录入
  await openMenu(page, "入库业务", "入库资料", /入库资料录入/);
  await expect(
    page.getByRole("heading", { name: "入库资料录入" }),
  ).toBeVisible();
  await page.getByLabel("关键字").fill(keyword);
  await page.getByRole("button", { name: "查询", exact: true }).click();
}

export async function openReview(page: Page, reportNo: string) {
  await openMenu(page, "入库业务", "入库资料", /药检单审核/);
  await expect(
    page.getByRole("heading", { name: "药检单审核" }),
  ).toBeVisible();
  const row = page.locator("tbody tr").filter({ hasText: reportNo });
  await expect(row).toBeVisible();
  await row.getByRole("button", { name: "审核" }).click();
}

export async function openStampPage(page: Page) {
  await openMenu(page, "入库业务", "入库资料", /药检图章配置/);
  await expect(
    page.getByRole("heading", { name: "药检图章配置" }),
  ).toBeVisible();
}

export async function setDefaultRequirementRule(
  page: Page,
  enabled: boolean,
) {
  await page.evaluate(async (enabled) => {
    const session = JSON.parse(
      localStorage.getItem("wms.web-admin.auth-session") ?? "null",
    ) as { accessToken?: string } | null;
    const response = await fetch(
      "/api/v1/drug-inspection/requirement-rules/current",
      {
        method: "PUT",
        headers: {
          Authorization: `Bearer ${session?.accessToken ?? ""}`,
          "Content-Type": "application/json",
          "Idempotency-Key": `mdi-rule-${enabled}-${crypto.randomUUID()}`,
        },
        body: JSON.stringify({
          special_drug_category: "*",
          missing_behavior: "block",
          enabled,
        }),
      },
    );
    if (!response.ok) {
      throw new Error(
        `准备药检验收规则失败：${response.status} ${await response.text()}`,
      );
    }
  }, enabled);
}

export async function fillAcceptanceDialog(
  dialog: ReturnType<Page["getByRole"]>,
  batchNo: string,
  receiptNo: string,
) {
  await dialog.getByLabel("验收批号").fill(batchNo);
  await dialog.getByLabel("通过数量").fill("10");
  await dialog.getByLabel("拒收数量", { exact: true }).fill("0");
  await dialog.getByLabel("生产日期").fill("2026-01-01");
  await dialog.getByLabel("有效期至").fill("2028-01-01");
  await dialog.getByLabel("追溯码").fill(`TRACE-${receiptNo}`);
  await dialog.getByLabel("质量状态").selectOption("qualified");
  for (const label of [
    "外观核对",
    "包装核对",
    "说明书核对",
    "标签核对",
  ]) {
    await dialog.getByLabel(label).fill("通过");
  }
  await dialog.getByLabel("抽验数量").fill("1");
  await dialog.getByLabel("批准文号").fill("国药准字E2E001");
  await dialog.getByLabel("第二签字人 ID").fill(reviewUserId);
}

export async function readCustomerCopy(page: Page, reportNo: string) {
  return page.evaluate(
    async ({ productId, batchA, reportNo }) => {
      const session = JSON.parse(
        localStorage.getItem("wms.web-admin.auth-session") ?? "null",
      ) as { accessToken?: string } | null;
      const headers = {
        Authorization: `Bearer ${session?.accessToken ?? ""}`,
      };
      const reusableResponse = await fetch(
        `/api/v1/drug-inspection/reports/reusable?product_id=${productId}&batch_no=${encodeURIComponent(batchA)}`,
        { headers },
      );
      if (!reusableResponse.ok) {
        return { status: "missing", pdfHeader: "", versionId: "" };
      }
      const reusable = (await reusableResponse.json()) as {
        report_id: string;
      };
      const versionsResponse = await fetch(
        `/api/v1/drug-inspection/reports/${reusable.report_id}/versions`,
        { headers },
      );
      const versions = (await versionsResponse.json()) as Array<{
        id: string;
        report_no: string;
        customer_copy_status: string;
        customer_copy_file_id?: string;
      }>;
      const version = versions.find((item) => item.report_no === reportNo);
      if (!version?.customer_copy_file_id) {
        return {
          status: version?.customer_copy_status ?? "missing",
          pdfHeader: "",
          versionId: version?.id ?? "",
        };
      }
      const urlResponse = await fetch(
        `/api/v1/attachments/${version.customer_copy_file_id}/url`,
        { headers },
      );
      if (!urlResponse.ok) {
        return {
          status: version.customer_copy_status,
          pdfHeader: "",
          versionId: version.id,
        };
      }
      const download = (await urlResponse.json()) as { url: string };
      const bytes = new Uint8Array(
        await (await fetch(download.url)).arrayBuffer(),
      );
      return {
        status: version.customer_copy_status,
        pdfHeader: new TextDecoder().decode(bytes.slice(0, 5)),
        versionId: version.id,
      };
    },
    { productId, batchA, reportNo },
  );
}

async function openMenu(
  page: Page,
  section: string,
  group: string,
  item: RegExp,
) {
  const navigation = page.getByRole("navigation");
  const target = navigation.getByRole("button", { name: item });
  if (!(await target.isVisible().catch(() => false))) {
    const sectionButton = navigation.getByRole("button", {
      name: section,
      exact: true,
    });
    if ((await sectionButton.getAttribute("aria-expanded")) !== "true") {
      await sectionButton.click();
    }
    const groupButton = navigation.getByRole("button", {
      name: group,
      exact: true,
    });
    if ((await groupButton.getAttribute("aria-expanded")) !== "true") {
      await groupButton.click();
    }
  }
  await target.click();
}
