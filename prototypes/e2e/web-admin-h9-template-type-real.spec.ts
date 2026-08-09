import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const evidenceDir = path.resolve("../artifacts/screenshot-portal/real-web/m1-system-dictionary");
const consumerEvidenceDir = path.resolve("../artifacts/screenshot-portal/real-web/h9-print-templates");
const ownerId = "00000000-0000-0000-0000-000000000001";
const ownerOverrideCode = "product_label";

test("US-H9-001 打印模板类型字典真实数据验收", async ({ page }) => {
  await login(page, "admin");
  // 种子缺口补授：warehouse_manager 角色缺少 h1.auth.me（工作台菜单权限键），否则后端按权限
  // 过滤后其菜单树缺工作台，App 判定“已发布菜单为空或缺少工作台入口”不渲染任何菜单，
  // 本测试后段 wh-manager 只读场景将无法经菜单导航。
  await grantWhManagerMenu(page);
  await openSystemDictionary(page);
  await page.getByRole("button", { name: /打印模板类型 print_template_type/ }).click();

  for (const code of [
    "asn",
    "acceptance_record",
    "delivery_note",
    "location_label",
    "lpn_label",
    "product_label",
  ]) {
    await expect(page.getByRole("button", { name: code, exact: true })).toBeVisible();
  }
  await page.getByRole("button", { name: "asn", exact: true }).click();
  for (const text of ["字段库编码", "业务模块", "业务方向", "纸张类型", "默认作用域", "排序号"]) {
    await expect(page.getByText(text, { exact: true }).first()).toBeVisible();
  }
  await expect(page.getByText("m2_asn", { exact: true })).toBeVisible();
  const sortOrders = await page.evaluate(async () => {
    const session = JSON.parse(localStorage.getItem("wms.web-admin.auth-session") ?? "null");
    const response = await fetch("/api/v1/system-dictionaries/print_template_type/items", {
      headers: { Authorization: `Bearer ${session.accessToken}` },
    });
    return (await response.json()).data.map((item: { sort_order: number }) => item.sort_order);
  });
  expect(sortOrders).toEqual([10, 20, 30, 40, 50, 60]);
  await expect(page.getByText("入库", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("A4", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("全局", { exact: true }).first()).toBeVisible();

  await page.getByRole("button", { name: "基础能力", exact: true }).click();
  await page.getByRole("button", { name: "H9 打印能力", exact: true }).click();
  await page.getByRole("button", { name: /H9 打印模板/ }).click();
  await expect(page.getByRole("heading", { name: "H9 打印模板" })).toBeVisible();
  await expect(page.getByRole("button", { name: "ASN 单 asn M2", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "商品标签 product_label M1", exact: true })).toBeVisible();
  fs.mkdirSync(consumerEvidenceDir, { recursive: true });
  await page.screenshot({
    path: path.join(consumerEvidenceDir, "template-type-tree.png"),
    fullPage: false,
  });
  await openSystemDictionary(page);
  await page.getByRole("button", { name: /打印模板类型 print_template_type/ }).click();

  const invalidFieldLibrary = await page.evaluate(async ({ ownerId: currentOwnerId }) => {
    const session = JSON.parse(localStorage.getItem("wms.web-admin.auth-session") ?? "null");
    const response = await fetch("/api/v1/system-dictionaries/print_template_type/items/asn", {
      method: "PUT",
      headers: {
        Authorization: `Bearer ${session.accessToken}`,
        "Content-Type": "application/json",
        "Idempotency-Key": `h9-type-empty-library-${Date.now()}`,
      },
      body: JSON.stringify({
        item_name: "非法空字段库",
        owner_id: currentOwnerId,
        enabled: true,
        sort_order: 10,
        params: {
          field_library_code: " ",
          business_module: "M2",
          business_direction: "inbound",
          paper_type: "a4",
          default_scope: "owner",
        },
        effective_from: null,
        effective_to: null,
      }),
    });
    return { status: response.status, body: await response.json() };
  }, { ownerId });
  expect(invalidFieldLibrary.status).toBe(422);
  expect(invalidFieldLibrary.body.code).toBe("H9_FIELD_LIBRARY_REQUIRED");

  await page.getByRole("button", { name: "新增", exact: true }).click();
  const createDialog = page.getByRole("dialog");
  await createDialog.getByLabel("编码", { exact: true }).fill(ownerOverrideCode);
  await createDialog.getByLabel("名称", { exact: true }).fill("商品标签·货主覆盖");
  await createDialog.getByLabel("货主 ID（可选）", { exact: true }).fill(ownerId);
  await createDialog.getByLabel("排序号", { exact: true }).fill("5");
  await createDialog.getByLabel("字段库编码", { exact: true }).fill("m1_product_label_owner");
  await createDialog.getByLabel("业务模块", { exact: true }).selectOption("M1");
  await createDialog.getByLabel("业务方向", { exact: true }).selectOption("label");
  await createDialog.getByLabel("纸张类型", { exact: true }).selectOption("a4");
  await createDialog.getByLabel("默认作用域", { exact: true }).selectOption("owner");
  const createResponse = page.waitForResponse(
    (response) =>
      response.url().includes(`/api/v1/system-dictionaries/print_template_type/items/${ownerOverrideCode}`) &&
      response.request().method() === "PUT",
  );
  await createDialog.getByRole("button", { name: "保存", exact: true }).click();
  expect((await createResponse).status()).toBe(200);
  await expect(createDialog).toBeHidden();

  await page.getByRole("button", { name: ownerOverrideCode, exact: true }).click();
  await expect(page.getByText("商品标签·货主覆盖", { exact: true })).toBeVisible();
  await expect(page.getByText("m1_product_label_owner", { exact: true })).toBeVisible();
  fs.mkdirSync(evidenceDir, { recursive: true });
  await page.screenshot({
    path: path.join(evidenceDir, "print-template-type-owner-override.png"),
    fullPage: false,
  });

  const ownerItem = page.getByRole("listitem").filter({ hasText: "m1_product_label_owner" });
  await ownerItem.getByRole("button", { name: "更新", exact: true }).click();
  const updateDialog = page.getByRole("dialog");
  await updateDialog.getByLabel("名称", { exact: true }).fill("商品标签·货主覆盖已更新");
  const updateResponse = page.waitForResponse(
    (response) =>
      response.url().includes(`/api/v1/system-dictionaries/print_template_type/items/${ownerOverrideCode}`) &&
      response.request().method() === "PUT",
  );
  await updateDialog.getByRole("button", { name: "保存", exact: true }).click();
  expect((await updateResponse).status()).toBe(200);
  await expect(page.getByText("商品标签·货主覆盖已更新", { exact: true })).toBeVisible();

  await ownerItem.getByRole("button", { name: "停用", exact: true }).click();
  const disableDialog = page.getByRole("dialog");
  await disableDialog.getByLabel("停用原因", { exact: true }).fill("US-H9-001 E2E 清理");
  const disableResponse = page.waitForResponse(
    (response) =>
      response.url().includes(`/api/v1/system-dictionaries/print_template_type/items/${ownerOverrideCode}/disable`) &&
      response.request().method() === "PATCH",
  );
  await disableDialog.getByRole("button", { name: "确认停用", exact: true }).click();
  expect((await disableResponse).status()).toBe(200);
  await expect(page.getByRole("button", { name: ownerOverrideCode, exact: true })).toHaveCount(0);

  const cleanupStatus = await page.evaluate(async ({ code, ownerId: currentOwnerId }) => {
    const session = JSON.parse(localStorage.getItem("wms.web-admin.auth-session") ?? "null");
    return fetch(`/api/v1/system-dictionaries/print_template_type/items/${code}`, {
      method: "PUT",
      headers: {
        Authorization: `Bearer ${session.accessToken}`,
        "Content-Type": "application/json",
        "Idempotency-Key": `h9-type-cleanup-${Date.now()}`,
      },
      body: JSON.stringify({
        item_name: "商品标签",
        owner_id: currentOwnerId,
        enabled: true,
        sort_order: 60,
        params: {
          field_library_code: "m1_product_label",
          business_module: "M1",
          business_direction: "label",
          paper_type: "label",
          default_scope: "global",
        },
        effective_from: null,
        effective_to: null,
      }),
    }).then((response) => response.status);
  }, { code: ownerOverrideCode, ownerId });
  expect(cleanupStatus).toBe(200);

  await login(page, "wh-manager");
  await openSystemDictionary(page);
  await page.getByRole("button", { name: /打印模板类型 print_template_type/ }).click();
  await expect(page.getByRole("button", { name: "新增", exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "asn", exact: true }).click();
  await expect(page.getByRole("button", { name: "更新", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "停用", exact: true })).toHaveCount(0);

  const forbiddenStatus = await page.evaluate(async ({ ownerId: currentOwnerId }) => {
    const session = JSON.parse(localStorage.getItem("wms.web-admin.auth-session") ?? "null");
    return fetch("/api/v1/system-dictionaries/print_template_type/items/asn", {
      method: "PUT",
      headers: {
        Authorization: `Bearer ${session.accessToken}`,
        "Content-Type": "application/json",
        "Idempotency-Key": `h9-type-readonly-${Date.now()}`,
      },
      body: JSON.stringify({
        item_name: "非法修改",
        owner_id: currentOwnerId,
        enabled: true,
        sort_order: 10,
        params: {
          field_library_code: "m2_asn",
          business_module: "M2",
          business_direction: "inbound",
          paper_type: "a4",
          default_scope: "owner",
        },
        effective_from: null,
        effective_to: null,
      }),
    }).then((response) => response.status);
  }, { ownerId });
  expect(forbiddenStatus).toBe(403);
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

async function openSystemDictionary(page: Page) {
  await page.getByRole("button", { name: "基础档案", exact: true }).click();
  const group = page.getByRole("navigation").getByRole("button", { name: "系统配置", exact: true });
  if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  await page.getByRole("navigation").getByRole("button", { name: /M1 系统字典/ }).click();
  await expect(page.getByRole("heading", { name: "M1 系统字典" })).toBeVisible();
}
