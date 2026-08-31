import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const evidenceDir = path.resolve("../artifacts/screenshot-portal/real-web/h9-print-templates");
const libraryCode = "m2_receiving_order_e2e";

test("US-H9-002 字段库真实数据生成、维护与发布", async ({ page }) => {
  await login(page, "admin");
  // 种子缺口补授：warehouse_manager 角色缺少 h1.auth.me（工作台菜单权限键），否则后端按权限
  // 过滤后其菜单树缺工作台，App 判定“已发布菜单为空或缺少工作台入口”不渲染任何菜单，
  // 本测试后段 wh-manager 只读场景将无法经菜单导航到 H9 打印模板。
  await grantWhManagerMenu(page);
  await openH9(page);
  await page.getByRole("button", { name: "字段库管理" }).click();
  const dialog = page.getByRole("dialog", { name: "字段库管理" });
  await dialog.getByLabel("字段库编码").fill(libraryCode);
  await dialog.getByLabel("字段库名称").fill("M2 收货单字段库 E2E");
  await dialog.getByLabel("业务模块").fill("M2");
  await dialog.getByLabel("来源 Schema").fill("CreateReceivingOrderRequest");
  const draftResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/print-templates/field-libraries/drafts")
      && response.request().method() === "POST",
  );
  await dialog.getByRole("button", { name: "生成草稿" }).click();
  expect((await draftResponse).status()).toBe(200);

  await expect(dialog.getByLabel("字段库版本")).toHaveValue(libraryCode);
  await expect(dialog.getByText("lines[].product_code", { exact: true })).toBeVisible();
  const receiptRow = dialog.getByRole("row").filter({ hasText: "receipt_no" });
  await expect(receiptRow).toBeVisible();
  await receiptRow.getByRole("button", { name: "编辑" }).click();
  await dialog.getByLabel("显示名称").fill("收货单号");
  await dialog.getByLabel("分组编码").fill("order");
  await dialog.getByLabel("分组名称").fill("单据信息");
  await dialog.getByLabel("排序号").fill("10");
  await dialog.getByLabel("说明").fill("仓库收货作业业务单号");
  await dialog.getByLabel("示例值").fill("ASN-E2E-202607260001");
  await dialog.getByLabel("脱敏规则").fill("keep_last_4");
  await dialog.getByLabel("格式化规则").fill("uppercase");
  await dialog.getByText("敏感字段", { exact: true }).click();
  await dialog.getByText("支持条码", { exact: true }).click();
  await dialog.getByText("支持二维码", { exact: true }).click();
  const updateResponse = page.waitForResponse(
    (response) =>
      response.url().includes("/api/v1/print-templates/field-libraries/")
      && response.url().includes("/fields/")
      && response.request().method() === "PATCH",
  );
  await dialog.getByRole("button", { name: "保存字段元数据" }).click();
  expect((await updateResponse).status()).toBe(200);
  await expect(dialog.getByText("收货单号", { exact: true })).toBeVisible();
  await expect(receiptRow).toContainText("可打印 / 敏感 / 条码 / 二维码");

  fs.mkdirSync(evidenceDir, { recursive: true });
  await dialog.evaluate((element) => element.scrollTo({ top: 0 }));
  await dialog.screenshot({
    path: path.join(evidenceDir, "field-library-draft-metadata.png"),
  });

  page.once("dialog", (confirmation) => confirmation.accept());
  const publishResponse = page.waitForResponse(
    (response) =>
      response.url().includes(`/api/v1/print-templates/field-libraries/`)
      && response.url().endsWith("/publish")
      && response.request().method() === "POST",
  );
  await dialog.getByRole("button", { name: "发布字段库" }).click();
  expect((await publishResponse).status()).toBe(200);
  await expect(dialog.getByText(`${libraryCode} v1 已发布`, { exact: true })).toBeVisible();
  await expect(dialog.getByRole("button", { name: "编辑" })).toHaveCount(0);
  await dialog.evaluate((element) => element.scrollTo({ top: 0 }));
  await dialog.screenshot({
    path: path.join(evidenceDir, "field-library-published.png"),
  });

  const contract = await page.evaluate(async (code) => {
    const session = JSON.parse(localStorage.getItem("wms.web-admin.auth-session") ?? "null");
    const headers = { Authorization: `Bearer ${session.accessToken}` };
    const libraries = await fetch("/api/v1/print-templates/field-libraries", { headers }).then((response) => response.json());
    const library = libraries.data.find((item: { library_code: string }) => item.library_code === code);
    const fields = await fetch(`/api/v1/print-templates/field-libraries/${library.latest_version_id}/fields`, { headers })
      .then((response) => response.json());
    return {
      library,
      receiptNo: fields.data.find((field: { field_path: string }) => field.field_path === "receipt_no"),
    };
  }, libraryCode);
  expect(contract.library).toMatchObject({
    business_module: "M2",
    source_schema: "CreateReceivingOrderRequest",
    latest_version_status: "published",
    latest_published_version_no: 1,
  });
  expect(contract.receiptNo).toMatchObject({
    display_name: "收货单号",
    group_code: "order",
    group_name: "单据信息",
    description: "仓库收货作业业务单号",
    example_value: "ASN-E2E-202607260001",
    printable: true,
    sensitive: true,
    masking_rule: "keep_last_4",
    formatting_rule: "uppercase",
    supports_barcode: true,
    supports_qrcode: true,
    is_table_detail: false,
    sort_order: 10,
  });

  await login(page, "wh-manager");
  await openH9(page);
  await expect(page.getByRole("button", { name: "字段库管理" })).toHaveCount(0);
  const forbidden = await page.evaluate(async () => {
    const session = JSON.parse(localStorage.getItem("wms.web-admin.auth-session") ?? "null");
    return fetch("/api/v1/print-templates/field-libraries/drafts", {
      method: "POST",
      headers: {
        Authorization: `Bearer ${session.accessToken}`,
        "Content-Type": "application/json",
        "Idempotency-Key": `h9-field-library-denied-${Date.now()}`,
      },
      body: JSON.stringify({
        library_code: "forbidden",
        library_name: "非法字段库",
        business_module: "M2",
        source_schema: "CreateReceivingOrderRequest",
      }),
    }).then((response) => response.status);
  });
  expect(forbidden).toBe(403);
});

/** E2E 种子缺口补授：warehouse_manager 角色未授予 h1.auth.me（工作台菜单节点的权限键），
 *  后端按权限过滤后仓库主管的已发布菜单树缺工作台，App 判定“已发布菜单为空或缺少工作台入口”
 *  而不渲染任何菜单，导致 wh-manager 只读场景无法经菜单导航。此处用 admin 会话经 H1 角色
 *  权限 API 在 wh-manager 登录前补授该权限（幂等：已存在则跳过）。 */
async function grantWhManagerMenu(page: Page) {
  await page.evaluate(async () => {
    const session = JSON.parse(localStorage.getItem("wms.web-admin.auth-session") ?? "null");
    const headers: Record<string, string> = {
      Authorization: `Bearer ${session.accessToken}`,
      "Content-Type": "application/json",
    };
    const rolesResponse = await fetch("/api/v1/auth/roles", { headers });
    if (!rolesResponse.ok) throw new Error(`列出角色失败: ${rolesResponse.status}`);
    const roles = (await rolesResponse.json()) as {
      items: Array<{ id: string; role_code: string; permission_codes: string[] }>;
    };
    const manager = roles.items.find((role) => role.role_code === "warehouse_manager");
    if (!manager) throw new Error("未找到 warehouse_manager 角色");
    if (manager.permission_codes.includes("h1.auth.me")) return;
    const response = await fetch(`/api/v1/auth/roles/${manager.id}/permissions`, {
      method: "PUT",
      headers: {
        ...headers,
        "Idempotency-Key": `e2e-wh-manager-menu-${Date.now()}`,
      },
      body: JSON.stringify({ permission_codes: [...manager.permission_codes, "h1.auth.me"] }),
    });
    if (!response.ok) {
      throw new Error(`补授 h1.auth.me 失败: ${response.status} ${await response.text()}`);
    }
  });
}

async function login(page: Page, username: string) {
  await page.goto("/");
  if (await page.getByRole("button", { name: "退出" }).isVisible()) {
    await page.getByRole("button", { name: "退出" }).click();
  }
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
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
