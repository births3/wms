/**
 * Shared helpers for capture_web_admin_interaction_screenshots.mjs
 * (login / nav / toolbar / filter / screenshot).
 */

import { chromium } from "../../node_modules/.pnpm/playwright@1.60.0/node_modules/playwright/index.mjs";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "../..");
export const BASE_URL = process.env.WMS_WEB_ADMIN_BASE_URL ?? "http://127.0.0.1:9002";
export const OUT_ROOT = path.join(REPO_ROOT, "artifacts/screenshot-portal/real-web");
export const VIEWPORT = { width: 1440, height: 900 };
export const CHROME = process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE || "/usr/bin/google-chrome";
export const LOGIN = {
  owner: process.env.WMS_WEB_ADMIN_OWNER ?? "PY_OWNER",
  username: process.env.WMS_WEB_ADMIN_USERNAME ?? "admin",
  password: process.env.WMS_WEB_ADMIN_PASSWORD ?? "CorrectHorse1!",
};

export { chromium, fs, path };

const TOP_SECTIONS = ["工作台", "基础档案", "入库业务", "出库业务", "库内业务", "基础能力"];
const SUBGROUPS = [
  "工作台概览", "主数据", "仓储资料", "系统配置", "入库作业", "入库资料", "出库作业", "库存管理",
  "增值作业", "H1 权限租户", "H2 审计能力", "H3 契约能力", "H4 企业微信", "H5 快递能力", "H-AL 告警能力",
  "H8 集成中心", "H9 打印能力", "M-CG 编码能力",
];

/** menu regex source → preferred subgroup to expand (avoid toggle thrash). */
const MENU_SUBGROUP = [
  [/M1 商品档案|M1 客商档案/, "主数据"],
  [/M1 仓库管理|M1 库区管理|M1 库位管理|月台/, "仓储资料"],
  [/M1 系统字典|M1 Feature Flag/, "系统配置"],
  [/M2 收货|M2 验收|M2 上架/, "入库作业"],
  [/药检|随货同行单/, "入库资料"],
  [/M4 出库订单|M4 波次|M4 复核|M4 采购退货/, "出库作业"],
  [/M3 批号|库位历史|状态规则|盘点|在库养护|移库|对账|任务/, "库存管理"],
  [/计费|路径规划/, "增值作业"],
  [/H1 菜单|H1 角色|H1 会话|H1 API/, "H1 权限租户"],
  [/H2 审计/, "H2 审计能力"],
  [/H3 OpenAPI|H3 契约/, "H3 契约能力"],
  [/H4 参数|H4 通知|H4 发送/, "H4 企业微信"],
  [/H5 快递/, "H5 快递能力"],
  [/H-AL|告警|升级规则/, "H-AL 告警能力"],
  [/H8 ERP|接口表/, "H8 集成中心"],
  [/H9 打印|设备·Print/, "H9 打印能力"],
  [/单据号规则/, "M-CG 编码能力"],
];

export function ensureDir(d) {
  fs.mkdirSync(d, { recursive: true });
}

async function visible(loc) {
  try {
    return await loc.isVisible();
  } catch {
    return false;
  }
}

export async function login(page) {
  await page.goto(BASE_URL + "/", { waitUntil: "networkidle" });
  await page.locator("#owner-code").fill(LOGIN.owner);
  await page.locator("#username").fill(LOGIN.username);
  // 避免 getByLabel('密码') 命中「显示密码」按钮
  await page.locator("#password").fill(LOGIN.password);
  await page.getByRole("button", { name: "登录", exact: true }).click();
  await page.getByRole("button", { name: /退出/ }).waitFor({ timeout: 15_000 });
}

/** Only expand when aria-expanded="false"; never toggle-close open sections. */
async function expandIfCollapsed(page, name) {
  const btn = page.getByRole("button", { name, exact: true }).first();
  if (!(await btn.count()) || !(await visible(btn))) return;
  const exp = await btn.getAttribute("aria-expanded").catch(() => null);
  if (exp === "true") return;
  if (exp !== "false") return; // unknown: do not toggle
  await btn.click({ timeout: 2_000 }).catch(() => {});
  await page.waitForTimeout(120);
}

function preferredSubgroup(menu) {
  const source = menu instanceof RegExp ? menu.source : String(menu);
  for (const [re, sub] of MENU_SUBGROUP) {
    if (re.test(source) || (typeof menu === "string" && re.test(menu))) return sub;
  }
  return null;
}

async function menuIfVisible(menuBtn) {
  if (!(await menuBtn.count())) return false;
  // Sidebar is scrollable — bring into view before visibility check
  await menuBtn.scrollIntoViewIfNeeded().catch(() => {});
  return visible(menuBtn);
}

async function ensureMenuVisible(page, group, menu) {
  const menuBtn = page.getByRole("button", { name: menu }).first();
  if (await menuIfVisible(menuBtn)) return menuBtn;

  if (group) await expandIfCollapsed(page, group);
  if (await menuIfVisible(menuBtn)) return menuBtn;

  const preferred = preferredSubgroup(menu);
  if (preferred) {
    await expandIfCollapsed(page, preferred);
    if (await menuIfVisible(menuBtn)) return menuBtn;
  }

  for (const sub of SUBGROUPS) {
    if (await menuIfVisible(menuBtn)) break;
    const s = page.getByRole("button", { name: sub, exact: true }).first();
    if (!(await s.count()) || !(await visible(s))) continue;
    const exp = await s.getAttribute("aria-expanded").catch(() => null);
    if (exp === "true") continue;
    await s.click({ timeout: 1_500 }).catch(() => {});
    await page.waitForTimeout(80);
  }

  if (!(await menuIfVisible(menuBtn))) {
    for (const top of TOP_SECTIONS) {
      if (await menuIfVisible(menuBtn)) break;
      await expandIfCollapsed(page, top);
      for (const sub of SUBGROUPS) {
        if (await menuIfVisible(menuBtn)) break;
        const s = page.getByRole("button", { name: sub, exact: true }).first();
        if (!(await s.count()) || !(await visible(s))) continue;
        const exp = await s.getAttribute("aria-expanded").catch(() => null);
        if (exp === "true") continue;
        await s.click({ timeout: 1_500 }).catch(() => {});
        await page.waitForTimeout(60);
      }
    }
  }

  await menuBtn.scrollIntoViewIfNeeded().catch(() => {});
  await menuBtn.waitFor({ state: "visible", timeout: 10_000 });
  return menuBtn;
}

export async function openPage(page, group, menu) {
  const btn = await ensureMenuVisible(page, group, menu);
  await btn.click({ timeout: 5_000 });
  await page.waitForTimeout(700);
}

export async function closeDialog(page) {
  if ((await page.getByRole("dialog").count()) === 0) return;
  await page.keyboard.press("Escape");
  await page.waitForTimeout(250);
  if ((await page.getByRole("dialog").count()) > 0) {
    await page.getByRole("button", { name: /取消|关闭/ }).first().click().catch(() => {});
    await page.waitForTimeout(200);
  }
  if ((await page.getByRole("dialog").count()) > 0) {
    await page.keyboard.press("Escape");
    await page.waitForTimeout(200);
  }
}

export async function firstVisible(locator) {
  const n = await locator.count();
  for (let i = 0; i < n; i++) {
    const item = locator.nth(i);
    if (await item.isVisible().catch(() => false)) return item;
  }
  return null;
}

/** Prefer enabled toolbar buttons in the main content area (x > sidebar). */
export async function firstContentButton(page, name, { exact = true } = {}) {
  const buttons = page.getByRole("button", { name, exact });
  let fallback = null;
  for (let i = 0; i < (await buttons.count()); i++) {
    const btn = buttons.nth(i);
    if (!(await btn.isVisible().catch(() => false))) continue;
    const box = await btn.boundingBox().catch(() => null);
    if (box && box.x < 280) continue; // sidebar
    if (!(await btn.isEnabled().catch(() => false))) {
      fallback = fallback ?? btn;
      continue;
    }
    return btn;
  }
  return fallback;
}

export async function clickContentButton(page, name, { exact = true, waitEnabledMs = 8_000 } = {}) {
  const deadline = Date.now() + waitEnabledMs;
  let btn = null;
  while (Date.now() < deadline) {
    btn = await firstContentButton(page, name, { exact });
    if (btn && (await btn.isEnabled().catch(() => false))) break;
    await page.waitForTimeout(200);
  }
  if (!btn) throw new Error(`content button not found: ${name}`);
  if (!(await btn.isEnabled().catch(() => false))) throw new Error(`content button disabled: ${name}`);
  await btn.click({ timeout: 5_000 });
  await page.waitForTimeout(400);
}

/** Prefer checkboxes inside the active main panel; skip hidden workspace tabs. */
export async function selectFirstDataRow(page) {
  // Prefer tbody row checkboxes (skip header "select all")
  const bodyBoxes = page.locator('tbody [role="checkbox"], tbody button[role="checkbox"]');
  const body = await firstVisible(bodyBoxes);
  if (body) {
    const state = await body.getAttribute("data-state").catch(() => null);
    if (state !== "checked") {
      await body.click({ force: true, timeout: 5_000 });
      await page.waitForTimeout(250);
    }
    return true;
  }

  const allBoxes = page.getByRole("checkbox");
  const visible = [];
  for (let i = 0; i < (await allBoxes.count()); i++) {
    const box = allBoxes.nth(i);
    if (await box.isVisible().catch(() => false)) visible.push(box);
  }
  // index 0 is usually header; pick first data checkbox
  if (visible.length >= 2) {
    const box = visible[1];
    const state = await box.getAttribute("data-state").catch(() => null);
    if (state !== "checked") {
      await box.click({ force: true, timeout: 5_000 });
      await page.waitForTimeout(250);
    }
    return true;
  }

  const cell = await firstVisible(page.locator("tbody tr"));
  if (cell) {
    await cell.click({ timeout: 5_000 });
    await page.waitForTimeout(250);
    return true;
  }
  return false;
}

export async function clickToolbar(page, name, { exact = true } = {}) {
  // Prefer enabled visible toolbar buttons (avoid status filter chips named similarly)
  const buttons = page.getByRole("button", { name, exact });
  let btn = null;
  for (let i = 0; i < (await buttons.count()); i++) {
    const candidate = buttons.nth(i);
    if (!(await candidate.isVisible().catch(() => false))) continue;
    btn = candidate;
    if (await candidate.isEnabled()) break;
  }
  if (!btn) throw new Error(`button not found: ${name}`);
  for (let i = 0; i < 20; i++) {
    if (await btn.isEnabled()) break;
    await page.waitForTimeout(150);
  }
  if (!(await btn.isEnabled())) {
    throw new Error(`button disabled: ${name}`);
  }
  await btn.click({ timeout: 5_000 });
  await page.waitForTimeout(500);
}

export async function shot(page, module, file) {
  const dir = path.join(OUT_ROOT, module);
  ensureDir(dir);
  const out = path.join(dir, file);
  await page.screenshot({ path: out, fullPage: false });
  return out;
}

export async function filterEmpty(page, keyword = "NO-SUCH-RECORD-XYZ") {
  // QueryPanel may collapse core fields — expand all visible expanders
  const expands = page.getByRole("button", { name: /展开|更多/ });
  for (let i = 0; i < (await expands.count()); i++) {
    const expand = expands.nth(i);
    if (await expand.isVisible().catch(() => false)) {
      await expand.click({ timeout: 1_500 }).catch(() => {});
      await page.waitForTimeout(150);
    }
  }

  // Prefer accessible name / placeholder APIs (QueryPanel uses aria-label = label)
  const candidates = [
    page.getByRole("textbox", { name: "关键字" }),
    page.getByRole("textbox", { name: /关键字|批号|单号|搜索/ }),
    page.getByPlaceholder(/批号|单号|关键字|搜索|编码|商品/),
    page.locator(
      'input[aria-label="关键字"], input[placeholder*="批号"], input[placeholder*="单号"], input[placeholder*="关键字"], input[placeholder*="搜索"], input[placeholder*="编码"], input[placeholder*="商品"]',
    ),
  ];
  let filled = false;
  for (const loc of candidates) {
    const target = await firstVisible(loc);
    if (!target) continue;
    await target.fill(keyword, { timeout: 3_000 });
    filled = true;
    break;
  }
  if (!filled) throw new Error("no visible keyword field");

  const queryBtn = await firstContentButton(page, "查询", { exact: true });
  if (!queryBtn) throw new Error("no visible query button");
  await queryBtn.click({ timeout: 5_000 });
  await page.waitForTimeout(600);
}

export async function clearFilter(page) {
  const reset = page.getByRole("button", { name: /重置|清除查询/ }).first();
  if (await reset.count()) {
    await reset.click().catch(() => {});
    await page.waitForTimeout(400);
  }
}
