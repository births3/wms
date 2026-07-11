#!/usr/bin/env node
/**
 * Capture real apps/web-admin screenshots from the fixed 9002 session
 * into artifacts/screenshot-portal/real-web/<module>/.
 *
 * Usage:
 *   node scripts/governance/capture_web_admin_real_screenshots.mjs
 *   WMS_WEB_ADMIN_BASE_URL=http://127.0.0.1:9002 node scripts/governance/capture_web_admin_real_screenshots.mjs
 *
 * Does not start or restart 9002. Marks data source from the running process.
 */

import { chromium } from "../../node_modules/.pnpm/playwright@1.60.0/node_modules/playwright/index.mjs";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "../..");
const BASE_URL = process.env.WMS_WEB_ADMIN_BASE_URL ?? "http://127.0.0.1:9002";
const OUT_ROOT = path.join(REPO_ROOT, "artifacts/screenshot-portal/real-web");
const VIEWPORT = { width: 1440, height: 900 };
const CHROME = process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE || "/usr/bin/google-chrome";

const LOGIN = {
  owner: process.env.WMS_WEB_ADMIN_OWNER ?? "PY_OWNER",
  username: process.env.WMS_WEB_ADMIN_USERNAME ?? "admin",
  password: process.env.WMS_WEB_ADMIN_PASSWORD ?? "CorrectHorse1!",
};

/** @type {Array<{ module: string; id: string; menuGroup?: string; menu: string | RegExp; title?: string | RegExp; file: string }>} */
const PAGES = [
  { module: "h1-auth", id: "login", menu: "", file: "login.png" },
  { module: "h1-auth", id: "dashboard", menuGroup: "工作台", menu: /运营总览/, title: /运营总览|WMS Web Admin/, file: "dashboard.png" },

  { module: "m1-master-data", id: "m1-products", menuGroup: "基础档案", menu: /M1 商品档案/, title: /M1 商品档案/, file: "m1-products-current.png" },
  { module: "m1-master-data", id: "m1-business-partners", menuGroup: "基础档案", menu: /M1 客商档案/, title: /M1 客商档案/, file: "m1-business-partners-current.png" },
  { module: "m1-master-data", id: "m1-warehouses", menuGroup: "基础档案", menu: /M1 仓库管理/, title: /M1 仓库管理/, file: "m1-warehouses-current.png" },
  { module: "m1-master-data", id: "m1-zones", menuGroup: "基础档案", menu: /M1 库区管理/, title: /M1 库区管理/, file: "m1-zones-current.png" },
  { module: "m1-master-data", id: "m1-locations", menuGroup: "基础档案", menu: /M1 库位管理/, title: /M1 库位管理/, file: "m1-locations-current.png" },
  { module: "m1-master-data", id: "m1-system-dictionary", menuGroup: "基础档案", menu: /M1 系统字典/, title: /M1 系统字典/, file: "m1-system-dictionary-current.png" },
  { module: "m1-master-data", id: "m1-feature-flags", menuGroup: "基础档案", menu: /M1 Feature Flag/, title: /Feature Flag|配置中心/, file: "m1-feature-flags-current.png" },

  { module: "m2-inbound", id: "m2-receiving", menuGroup: "入库业务", menu: /M2 收货管理/, title: /M2 收货管理/, file: "m2-receiving-current.png" },
  { module: "m2-inbound", id: "m2-inspecting", menuGroup: "入库业务", menu: /M2 验收管理/, title: /M2 验收管理/, file: "m2-inspecting-current.png" },
  { module: "m2-inbound", id: "m2-putaway", menuGroup: "入库业务", menu: /M2 上架管理/, title: /M2 上架管理/, file: "m2-putaway-current.png" },

  { module: "m4-outbound", id: "m4-orders", menuGroup: "出库业务", menu: /M4 出库订单管理/, title: /M4 出库订单管理/, file: "m4-order-current.png" },
  { module: "m4-outbound", id: "m4-waves", menuGroup: "出库业务", menu: /M4 波次规划/, title: /M4 波次规划/, file: "m4-wave-current.png" },
  { module: "m4-outbound", id: "m4-review", menuGroup: "出库业务", menu: /M4 复核发货/, title: /M4 复核发货/, file: "m4-review-shipping-current.png" },
  { module: "m4-outbound", id: "m4-returns", menuGroup: "出库业务", menu: /M4 采购退货出库/, title: /M4 采购退货出库/, file: "m4-purchase-return-current.png" },

  { module: "m3-inventory", id: "m3-batches", menuGroup: "库内业务", menu: /M3 批号管理/, title: /M3 批号管理/, file: "m3-batches-current.png" },

  { module: "h-platform", id: "h1-menu-management", menuGroup: "基础能力", menu: /H1 菜单管理/, title: /H1 菜单管理/, file: "h1-menu-management-current.png" },
  { module: "h-platform", id: "h2-audit-trail", menuGroup: "基础能力", menu: /H2 审计追踪/, title: /H2 审计追踪/, file: "h2-audit-trail-current.png" },
  { module: "h-platform", id: "h3-api-contract", menuGroup: "基础能力", menu: /H3 OpenAPI/, title: /H3|OpenAPI/, file: "h3-api-contract-current.png" },
  { module: "h-platform", id: "h4-wechat-settings", menuGroup: "基础能力", menu: /H4 参数设置/, title: /H4|参数设置|企业微信/, file: "h4-wechat-settings-current.png" },
  { module: "h-platform", id: "h4-notify-configs", menuGroup: "基础能力", menu: /H4 通知配置/, title: /H4|通知配置/, file: "h4-notify-configs-current.png" },
  { module: "h-platform", id: "h4-notify-records", menuGroup: "基础能力", menu: /H4 发送记录/, title: /H4|发送记录/, file: "h4-notify-records-current.png" },
  { module: "h-platform", id: "h5-express", menuGroup: "基础能力", menu: /H5 快递对接/, title: /H5|快递/, file: "h5-express-current.png" },
  { module: "h-platform", id: "h9-print-templates", menuGroup: "基础能力", menu: /H9 打印模板/, title: /H9|打印模板/, file: "h9-print-templates-current.png" },
];

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function writeIndex(moduleDir, shots) {
  const items = shots
    .map(
      (s) =>
        `<li><a href="./${s.file}"><img src="./${s.file}" alt="${s.id}" style="max-width:320px;border:1px solid #ddd"/><br/>${s.id} · ${s.file}${s.ok ? "" : " · FAILED"}</a></li>`,
    )
    .join("\n");
  fs.writeFileSync(
    path.join(moduleDir, "index.html"),
    `<!doctype html><html lang="zh-CN"><meta charset="utf-8"/><title>${path.basename(moduleDir)} screenshots</title>
<body style="font-family:sans-serif;padding:16px">
<h1>${path.basename(moduleDir)}</h1>
<ul style="display:flex;flex-wrap:wrap;gap:16px;list-style:none;padding:0">${items}</ul>
</body></html>`,
  );
}

const TOP_SECTIONS = ["工作台", "基础档案", "入库业务", "出库业务", "库内业务", "基础能力"];
const SUBGROUPS = [
  "工作台概览",
  "主数据",
  "仓储资料",
  "系统配置",
  "入库作业",
  "出库作业",
  "库存管理",
  "H1 权限租户",
  "H2 审计能力",
  "H3 契约能力",
  "H4 企业微信",
  "H5 快递能力",
  "H9 打印能力",
];

async function isVisible(locator) {
  try {
    return await locator.isVisible();
  } catch {
    return false;
  }
}

/** Expand section/subgroup only if target menu is not yet visible (avoid toggle-close). */
async function ensureMenuVisible(page, group, menu) {
  const menuBtn = page.getByRole("button", { name: menu }).first();
  if (await isVisible(menuBtn)) return menuBtn;

  if (group) {
    const groupBtn = page.getByRole("button", { name: group, exact: true }).first();
    if ((await groupBtn.count()) > 0 && !(await isVisible(menuBtn))) {
      await groupBtn.click({ timeout: 5_000 });
      await page.waitForTimeout(200);
    }
  }

  for (const sub of SUBGROUPS) {
    if (await isVisible(menuBtn)) break;
    const subBtn = page.getByRole("button", { name: sub, exact: true }).first();
    if ((await subBtn.count()) > 0) {
      await subBtn.click({ timeout: 2_000 }).catch(() => {});
      await page.waitForTimeout(120);
    }
  }

  if (!(await isVisible(menuBtn))) {
    for (const top of TOP_SECTIONS) {
      if (await isVisible(menuBtn)) break;
      const btn = page.getByRole("button", { name: top, exact: true }).first();
      if ((await btn.count()) > 0) {
        await btn.click({ timeout: 2_000 }).catch(() => {});
        await page.waitForTimeout(120);
      }
      for (const sub of SUBGROUPS) {
        if (await isVisible(menuBtn)) break;
        const subBtn = page.getByRole("button", { name: sub, exact: true }).first();
        if ((await subBtn.count()) > 0) {
          await subBtn.click({ timeout: 2_000 }).catch(() => {});
          await page.waitForTimeout(80);
        }
      }
    }
  }

  await menuBtn.waitFor({ state: "visible", timeout: 8_000 });
  return menuBtn;
}

async function login(page) {
  await page.goto(BASE_URL + "/", { waitUntil: "networkidle" });
  await page.getByLabel("货主编码").fill(LOGIN.owner);
  await page.getByLabel("登录账号").fill(LOGIN.username);
  await page.getByLabel("密码").fill(LOGIN.password);
  await page.getByRole("button", { name: "登录" }).click();
  await page.getByRole("button", { name: /退出/ }).waitFor({ timeout: 15_000 });
}

async function captureOne(page, item) {
  const moduleDir = path.join(OUT_ROOT, item.module);
  ensureDir(moduleDir);
  const outFile = path.join(moduleDir, item.file);
  const started = Date.now();

  if (item.id === "login") {
    // force logout-ish by clearing storage then open login
    await page.goto(BASE_URL + "/", { waitUntil: "networkidle" });
    await page.evaluate(() => {
      localStorage.clear();
      sessionStorage.clear();
    });
    await page.goto(BASE_URL + "/", { waitUntil: "networkidle" });
    await page.getByLabel("货主编码").waitFor({ timeout: 10_000 });
    await page.screenshot({ path: outFile, fullPage: false });
    return { ...item, ok: true, path: outFile, ms: Date.now() - started, error: null };
  }

  // ensure logged in shell
  if ((await page.getByRole("button", { name: /退出/ }).count()) === 0) {
    await login(page);
  }

  const menuBtn = await ensureMenuVisible(page, item.menuGroup, item.menu);
  await menuBtn.click({ timeout: 8_000 });
  if (item.title) {
    await page.getByRole("heading", { name: item.title }).first().waitFor({ timeout: 10_000 }).catch(async () => {
      // fallback: wait for menu active / any main content
      await page.waitForTimeout(800);
    });
  } else {
    await page.waitForTimeout(600);
  }
  await page.waitForTimeout(400);
  await page.screenshot({ path: outFile, fullPage: false });
  return { ...item, ok: true, path: outFile, ms: Date.now() - started, error: null };
}

async function main() {
  ensureDir(OUT_ROOT);
  const browser = await chromium.launch({
    executablePath: CHROME,
    headless: true,
    args: ["--no-sandbox", "--disable-dev-shm-usage"],
  });
  const context = await browser.newContext({
    viewport: VIEWPORT,
    deviceScaleFactor: 1,
  });
  const page = await context.newPage();

  /** @type {any[]} */
  const results = [];
  const byModule = new Map();

  // login page first (logged out)
  try {
    const r = await captureOne(page, PAGES[0]);
    results.push(r);
    byModule.set(r.module, [...(byModule.get(r.module) || []), r]);
    console.log(`OK  ${r.module}/${r.file}`);
  } catch (err) {
    const r = { ...PAGES[0], ok: false, path: null, ms: 0, error: String(err) };
    results.push(r);
    byModule.set(r.module, [...(byModule.get(r.module) || []), r]);
    console.error(`FAIL ${PAGES[0].module}/${PAGES[0].file}: ${err}`);
  }

  // login once for remaining pages
  try {
    await login(page);
  } catch (err) {
    console.error("login failed:", err);
    await browser.close();
    process.exit(2);
  }

  for (const item of PAGES.slice(1)) {
    try {
      const r = await captureOne(page, item);
      results.push(r);
      byModule.set(r.module, [...(byModule.get(r.module) || []), r]);
      console.log(`OK  ${r.module}/${r.file} (${r.ms}ms)`);
    } catch (err) {
      const r = { ...item, ok: false, path: null, ms: 0, error: String(err?.message || err) };
      results.push(r);
      byModule.set(r.module, [...(byModule.get(r.module) || []), r]);
      console.error(`FAIL ${item.module}/${item.file}: ${err?.message || err}`);
    }
  }

  for (const [mod, shots] of byModule.entries()) {
    writeIndex(path.join(OUT_ROOT, mod), shots);
  }

  const report = {
    captured_at: new Date().toISOString(),
    base_url: BASE_URL,
    viewport: VIEWPORT,
    data_source: "WMS_WEB_ADMIN_DEV_MOCK (running 9002 session)",
    note: "真实 apps/web-admin 截图；当前 9002 进程启用 DEV_MOCK，非原型。若需真后端证据，请用无 mock 的 9002 + 真实 API 重跑。",
    total: results.length,
    ok: results.filter((r) => r.ok).length,
    failed: results.filter((r) => !r.ok).length,
    results: results.map((r) => ({
      module: r.module,
      id: r.id,
      file: r.file,
      ok: r.ok,
      path: r.path,
      ms: r.ms,
      error: r.error,
    })),
  };

  fs.writeFileSync(path.join(OUT_ROOT, "capture-report.json"), JSON.stringify(report, null, 2));

  // root index
  const modules = [...byModule.keys()];
  fs.writeFileSync(
    path.join(OUT_ROOT, "index.html"),
    `<!doctype html><html lang="zh-CN"><meta charset="utf-8"/><title>real-web screenshots</title>
<body style="font-family:sans-serif;padding:16px">
<h1>apps/web-admin 真实截图</h1>
<p>base: ${BASE_URL} · data_source: ${report.data_source}</p>
<p>ok ${report.ok}/${report.total} · captured_at ${report.captured_at}</p>
<ul>${modules.map((m) => `<li><a href="./${m}/">${m}</a></li>`).join("")}</ul>
</body></html>`,
  );

  await browser.close();
  console.log(JSON.stringify({ ok: report.ok, failed: report.failed, total: report.total, out: OUT_ROOT }, null, 2));
  process.exit(report.failed > 0 ? 1 : 0);
}

main().catch((err) => {
  console.error(err);
  process.exit(2);
});
