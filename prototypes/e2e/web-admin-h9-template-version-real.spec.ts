import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const evidenceDir = path.resolve("../artifacts/screenshot-portal/real-web/h9-print-templates");
const templateCode = "m2_asn_e2e";

test("US-H9-003 模板草稿、独立发布、停用与只读预览", async ({ page }) => {
  await login(page, "admin");
  await openH9(page);

  const row = page.getByRole("row").filter({ hasText: templateCode });
  await expect(row).toContainText("M2 ASN E2E 模板");
  await row.getByRole("checkbox").check();
  await page.getByRole("button", { name: "修改", exact: true }).click();

  const designer = page.getByRole("dialog", { name: "修改打印模板" });
  await expect(designer).toBeVisible();
  await designer.getByLabel("模板名称").fill("M2 ASN E2E 模板 v2");
  const saveResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/print-templates/templates")
      && response.request().method() === "POST",
  );
  await designer.getByRole("button", { name: "保存新草稿", exact: true }).last().click();
  expect((await saveResponse).status()).toBe(200);
  await expect(designer).toBeHidden();
  await expect(row).toContainText("M2 ASN E2E 模板 v2");
  await expect(row).toContainText("v2");
  await expect(row).toContainText("草稿");

  const draftContract = await readTemplateContract(page);
  expect(draftContract.summary.latest_version_status).toBe("draft");
  expect(draftContract.versions.map((version) => version.status)).toEqual(["draft", "published"]);
  expect(draftContract.resolved.version.version_no).toBe(1);
  expect(draftContract.versions[1].field_library_version_id).toBe(
    draftContract.versions[0].field_library_version_id,
  );

  fs.mkdirSync(evidenceDir, { recursive: true });
  await page.screenshot({
    path: path.join(evidenceDir, "template-version-draft.png"),
    fullPage: false,
  });

  page.once("dialog", (confirmation) => confirmation.accept());
  const publishResponse = page.waitForResponse(
    (response) =>
      response.url().includes("/api/v1/print-templates/templates/")
      && response.url().endsWith("/publish")
      && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "发布", exact: true }).click();
  expect((await publishResponse).status()).toBe(200);
  await expect(row).toContainText("已发布");

  const publishedContract = await readTemplateContract(page);
  expect(publishedContract.summary.latest_version_status).toBe("published");
  expect(publishedContract.versions.map((version) => version.status)).toEqual([
    "published",
    "published",
  ]);
  expect(publishedContract.resolved.version.version_no).toBe(2);
  expect(publishedContract.resolved.version.template_name).toBe("M2 ASN E2E 模板 v2");

  await page.getByRole("button", { name: "版本", exact: true }).click();
  const history = page.getByRole("dialog", { name: "版本历史" });
  await expect(history.getByRole("row").filter({ hasText: "v2" })).toContainText("已发布");
  await expect(history.getByRole("row").filter({ hasText: "v1" })).toContainText("已发布");
  await history.screenshot({
    path: path.join(evidenceDir, "template-version-published.png"),
  });
  await history.getByRole("button", { name: "关闭", exact: true }).click();

  page.once("dialog", (confirmation) => confirmation.accept());
  const disableResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/enabled")
      && response.request().method() === "PATCH",
  );
  await page.getByRole("button", { name: "停用", exact: true }).click();
  expect((await disableResponse).status()).toBe(200);
  await expect(row).toContainText("停用");
  await page.screenshot({
    path: path.join(evidenceDir, "template-disabled.png"),
    fullPage: false,
  });
  page.once("dialog", (confirmation) => confirmation.accept());
  const enableResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/enabled")
      && response.request().method() === "PATCH",
  );
  await page.getByRole("button", { name: "启用", exact: true }).click();
  expect((await enableResponse).status()).toBe(200);
  await expect(row).toContainText("启用");

  await login(page, "wh-manager");
  await openH9(page);
  for (const action of ["新增", "修改", "复制", "发布", "停用", "启用"]) {
    await expect(page.getByRole("button", { name: action, exact: true })).toHaveCount(0);
  }
  const readonlyRow = page.getByRole("row").filter({ hasText: templateCode });
  await readonlyRow.getByRole("checkbox").check();
  page.once("dialog", (confirmation) => confirmation.accept());
  const previewResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/print-templates/preview")
      && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "预览", exact: true }).click();
  expect((await previewResponse).status()).toBe(200);
  await expect(page.getByRole("dialog", { name: "M2 ASN E2E 模板 v2" })).toBeVisible();

  const forbiddenPublishStatus = await page.evaluate(async () => {
    const session = JSON.parse(localStorage.getItem("wms.web-admin.auth-session") ?? "null");
    return fetch(
      "/api/v1/print-templates/templates/00000000-0000-0000-0000-000000003801/versions/00000000-0000-0000-0000-000000003901/publish",
      {
        method: "POST",
        headers: {
          Authorization: `Bearer ${session.accessToken}`,
          "Idempotency-Key": `h9-template-publish-forbidden-${Date.now()}`,
        },
      },
    ).then((response) => response.status);
  });
  expect(forbiddenPublishStatus).toBe(403);
});

async function readTemplateContract(page: Page) {
  return page.evaluate(async (code) => {
    const session = JSON.parse(localStorage.getItem("wms.web-admin.auth-session") ?? "null");
    const headers = {
      Authorization: `Bearer ${session.accessToken}`,
      "Content-Type": "application/json",
    };
    const list = await fetch("/api/v1/print-templates/templates", { headers }).then((response) =>
      response.json(),
    );
    const summary = list.data.find(
      (template: { template_code: string }) => template.template_code === code,
    );
    const versions = await fetch(
      `/api/v1/print-templates/templates/${summary.id}/versions`,
      { headers },
    ).then((response) => response.json());
    const resolved = await fetch("/api/v1/print-templates/resolve", {
      method: "POST",
      headers,
      body: JSON.stringify({
        template_code: code,
        template_type_code: "asn",
      }),
    }).then((response) => response.json());
    return { summary, versions: versions.data, resolved };
  }, templateCode);
}

async function login(page: Page, username: string) {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill(username);
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function openH9(page: Page) {
  await page.getByRole("button", { name: "基础能力", exact: true }).click();
  await page.getByRole("button", { name: "H9 打印能力", exact: true }).click();
  await page.getByRole("button", { name: /H9 打印模板/ }).click();
  await expect(page.getByRole("heading", { name: "H9 打印模板" })).toBeVisible();
}
