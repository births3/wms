import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const apiURL = process.env.WMS_WEB_ADMIN_E2E_API_URL ?? "http://127.0.0.1:19189";
const artifactsDir = path.resolve("../artifacts/screenshot-portal/real-web/m9-billing-rules");
const uuidPattern = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

type BillingContract = { id: string; account_id: string; contract_no: string };
type BillingRule = {
  id: string;
  contract_id: string;
  charge_item: string;
  unit: string;
  billing_cycle: string;
  unit_price_cents: number;
  effective_from: string;
  effective_to: string;
};
type ApiBody = { id?: string; code?: string; message?: string };

test("US-M9-001 真实后端创建计费规则并保留浏览器证据", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });
  await login(page);
  await openBillingPage(page);
  await expect(page.getByRole("heading", { name: "M9 计费规则配置" })).toBeVisible();
  for (const id of [
    "billing-charge-item",
    "billing-unit",
    "billing-cycle",
    "billing-price",
    "billing-effective-from",
    "billing-effective-to",
    "billing-contract-id",
  ]) {
    await expect(page.locator(`#${id}`)).toBeVisible();
  }
  await page.screenshot({ path: path.join(artifactsDir, "page-loaded.png"), fullPage: false });

  const contract = await createContract(page);
  await page.locator("#billing-contract-id").fill(contract.id);
  await page.locator("#billing-effective-from").fill("2026-07-01");
  await page.locator("#billing-effective-to").fill("2026-07-31");

  await page.locator("#billing-price").fill("9007199254740992");
  await page.getByRole("button", { name: "保存计费规则", exact: true }).click();
  await expect(page.getByRole("alert")).toHaveText("单价必须是非负整数（分）");
  await page.screenshot({ path: path.join(artifactsDir, "invalid-rate.png"), fullPage: false });

  await page.locator("#billing-price").fill("125");
  await page.locator("#billing-effective-from").fill("2026-07-31");
  await page.locator("#billing-effective-to").fill("2026-07-01");
  await page.getByRole("button", { name: "保存计费规则", exact: true }).click();
  await expect(page.getByRole("alert")).toHaveText("生效止日不能早于生效起日");
  await page.screenshot({ path: path.join(artifactsDir, "invalid-date-window.png"), fullPage: false });

  await page.locator("#billing-effective-from").fill("2026-07-01");
  await page.locator("#billing-effective-to").fill("2026-07-31");
  const ruleResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/billing/rules") && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "保存计费规则", exact: true }).click();
  const ruleResponse = await ruleResponsePromise;
  expect(ruleResponse.status(), await ruleResponse.text()).toBe(200);
  const rule = (await ruleResponse.json()) as BillingRule;
  expect(rule).toMatchObject({
    contract_id: contract.id,
    charge_item: "storage",
    unit: "pallet_day",
    billing_cycle: "monthly",
    unit_price_cents: 125,
    effective_from: "2026-07-01",
    effective_to: "2026-07-31",
  });
  expect(rule.id).toMatch(uuidPattern);
  await expect(page.getByRole("status")).toContainText(`计费规则 ${rule.id} 已创建`);
  await expect(page.getByText(contract.id, { exact: true })).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "rule-created.png"), fullPage: false });
});

async function login(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  const loginResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/auth/login") && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "登录", exact: true }).click();
  const loginResponse = await loginResponsePromise;
  expect(loginResponse.status(), await loginResponse.text()).toBe(200);
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
  const token = await page.evaluate(() => {
    const session = JSON.parse(localStorage.getItem("wms.web-admin.auth-session") ?? "null") as { accessToken?: string } | null;
    return session?.accessToken ?? "";
  });
  expect(token.split(".")).toHaveLength(3);
}

async function openBillingPage(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const target = navigation.getByRole("button", { name: /M9 计费规则/ });
  if (!(await target.isVisible())) {
    const section = navigation.getByRole("button", { name: "增值业务", exact: true });
    if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
    const group = navigation.getByRole("button", { name: "增值作业", exact: true });
    if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  }
  await expect(target).toBeVisible();
  await target.click();
}

async function createContract(page: import("@playwright/test").Page): Promise<BillingContract> {
  const token = await page.evaluate(() => {
    const session = JSON.parse(localStorage.getItem("wms.web-admin.auth-session") ?? "null") as { accessToken?: string } | null;
    return session?.accessToken ?? "";
  });
  expect(token).toMatch(/\S+/);
  const headers = { Authorization: `Bearer ${token}`, "Content-Type": "application/json" };
  const runId = Date.now().toString();
  const accountResponse = await page.request.post(`${apiURL}/api/v1/billing/accounts`, {
    headers,
    data: { account_code: `M9-E2E-${runId}`, account_name: `M9 E2E 计费账户 ${runId}` },
  });
  const account = await successfulBody(accountResponse, "/api/v1/billing/accounts");
  expect(account.id).toMatch(uuidPattern);

  const contractResponse = await page.request.post(`${apiURL}/api/v1/billing/contracts`, {
    headers: { ...headers, "Idempotency-Key": `m9-e2e-contract-${runId}` },
    data: {
      account_id: account.id,
      contract_no: `M9-E2E-CONTRACT-${runId}`,
      valid_from: "2026-01-01",
      valid_to: "2027-12-31",
    },
  });
  const contract = await successfulBody(contractResponse, "/api/v1/billing/contracts");
  expect(contract.id).toMatch(uuidPattern);
  return contract as BillingContract;
}

async function successfulBody(response: import("@playwright/test").APIResponse, endpoint: string): Promise<ApiBody> {
  const body = (await response.json()) as ApiBody;
  if (!response.ok()) {
    throw new Error(`${endpoint} blocked: HTTP ${response.status()} ${body.code ?? "UNKNOWN"}: ${body.message ?? "no message"}; requires a seeded m9.write permission for the real E2E admin`);
  }
  return body;
}
