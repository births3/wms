#!/usr/bin/env node
/**
 * Capture dialog / empty-state screenshots from apps/web-admin on 9002.
 * Complements capture_web_admin_real_screenshots.mjs (first-screen only).
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

const TOP_SECTIONS = ["工作台", "基础档案", "入库业务", "出库业务", "库内业务", "基础能力"];
const SUBGROUPS = [
  "工作台概览", "主数据", "仓储资料", "系统配置", "入库作业", "出库作业", "库存管理",
  "H1 权限租户", "H2 审计能力", "H3 契约能力", "H4 企业微信", "H5 快递能力", "H9 打印能力",
];

function ensureDir(d) {
  fs.mkdirSync(d, { recursive: true });
}

async function visible(loc) {
  try {
    return await loc.isVisible();
  } catch {
    return false;
  }
}

async function login(page) {
  await page.goto(BASE_URL + "/", { waitUntil: "networkidle" });
  await page.locator("#owner-code").fill(LOGIN.owner);
  await page.locator("#username").fill(LOGIN.username);
  // 避免 getByLabel('密码') 命中「显示密码」按钮
  await page.locator("#password").fill(LOGIN.password);
  await page.getByRole("button", { name: "登录", exact: true }).click();
  await page.getByRole("button", { name: /退出/ }).waitFor({ timeout: 15_000 });
}

async function ensureMenuVisible(page, group, menu) {
  const menuBtn = page.getByRole("button", { name: menu }).first();
  if (await visible(menuBtn)) return menuBtn;
  if (group) {
    const g = page.getByRole("button", { name: group, exact: true }).first();
    if ((await g.count()) && !(await visible(menuBtn))) {
      await g.click().catch(() => {});
      await page.waitForTimeout(150);
    }
  }
  for (const sub of SUBGROUPS) {
    if (await visible(menuBtn)) break;
    const s = page.getByRole("button", { name: sub, exact: true }).first();
    if (await s.count()) {
      await s.click().catch(() => {});
      await page.waitForTimeout(100);
    }
  }
  if (!(await visible(menuBtn))) {
    for (const top of TOP_SECTIONS) {
      if (await visible(menuBtn)) break;
      await page.getByRole("button", { name: top, exact: true }).first().click().catch(() => {});
      await page.waitForTimeout(80);
      for (const sub of SUBGROUPS) {
        if (await visible(menuBtn)) break;
        await page.getByRole("button", { name: sub, exact: true }).first().click().catch(() => {});
        await page.waitForTimeout(60);
      }
    }
  }
  await menuBtn.waitFor({ state: "visible", timeout: 10_000 });
  return menuBtn;
}

async function openPage(page, group, menu) {
  const btn = await ensureMenuVisible(page, group, menu);
  await btn.click();
  await page.waitForTimeout(700);
}

async function closeDialog(page) {
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

async function selectFirstDataRow(page) {
  const boxes = page.getByRole("checkbox");
  const n = await boxes.count();
  if (n >= 2) {
    const box = boxes.nth(1);
    const state = await box.getAttribute("data-state").catch(() => null);
    if (state !== "checked") {
      await box.click({ force: true });
      await page.waitForTimeout(200);
    }
    return true;
  }
  // fallback: click first data cell-ish text
  const cell = page.locator("tbody tr").first();
  if (await cell.count()) {
    await cell.click();
    await page.waitForTimeout(200);
    return true;
  }
  return false;
}

async function clickToolbar(page, name, { exact = true } = {}) {
  const btn = page.getByRole("button", { name, exact }).first();
  await btn.waitFor({ state: "visible", timeout: 5_000 });
  // wait enable (max ~3s)
  for (let i = 0; i < 12; i++) {
    if (await btn.isEnabled()) break;
    await page.waitForTimeout(150);
  }
  if (!(await btn.isEnabled())) {
    throw new Error(`button disabled: ${name}`);
  }
  await btn.click({ timeout: 5_000 });
  await page.waitForTimeout(500);
}

async function shot(page, module, file) {
  const dir = path.join(OUT_ROOT, module);
  ensureDir(dir);
  const out = path.join(dir, file);
  await page.screenshot({ path: out, fullPage: false });
  return out;
}

async function filterEmpty(page, keyword = "NO-SUCH-RECORD-XYZ") {
  // QueryPanel may collapse core fields — expand first
  const expand = page.getByRole("button", { name: /展开|更多/ }).first();
  if ((await expand.count()) && (await expand.isVisible().catch(() => false))) {
    await expand.click().catch(() => {});
    await page.waitForTimeout(200);
  }

  const candidates = [
    page.getByLabel("关键字"),
    page.getByPlaceholder(/单号|关键字|搜索|编码|名称|ASN|商品|批号/),
    page.locator('input[aria-label="关键字"]'),
    page.locator('main input[type="text"], section input[type="text"]').first(),
  ];
  let filled = false;
  for (const loc of candidates) {
    const target = loc.first();
    if (!(await target.count())) continue;
    if (!(await target.isVisible().catch(() => false))) continue;
    await target.fill(keyword, { timeout: 3_000 });
    filled = true;
    break;
  }
  if (!filled) throw new Error("no visible keyword field");
  await page.getByRole("button", { name: "查询" }).first().click({ timeout: 5_000 });
  await page.waitForTimeout(600);
}

async function clearFilter(page) {
  const reset = page.getByRole("button", { name: /重置|清除查询/ }).first();
  if (await reset.count()) {
    await reset.click().catch(() => {});
    await page.waitForTimeout(400);
  }
}

/** @type {Array<{id:string, module:string, file:string, run:(page:import('playwright').Page)=>Promise<void>}>} */
const ONLY = new Set(
  (process.env.WMS_CAPTURE_ONLY || "")
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean),
);

const SCENES = [
  // ---- M1 ----
  {
    id: "m1-products-create",
    module: "m1-master-data",
    file: "m1-products-create-dialog-current.png",
    async run(page) {
      await openPage(page, "基础档案", /M1 商品档案/);
      await clickToolbar(page, "新增");
      await page.getByRole("dialog").waitFor({ timeout: 8_000 });
    },
  },
  {
    id: "m1-products-edit",
    module: "m1-master-data",
    file: "m1-products-edit-dialog-current.png",
    async run(page) {
      await openPage(page, "基础档案", /M1 商品档案/);
      await selectFirstDataRow(page);
      await clickToolbar(page, "修改");
      await page.getByRole("dialog").waitFor({ timeout: 8_000 });
    },
  },
  {
    id: "m1-products-empty",
    module: "m1-master-data",
    file: "m1-products-filter-empty-current.png",
    async run(page) {
      await openPage(page, "基础档案", /M1 商品档案/);
      await filterEmpty(page, "NO-SUCH-PRODUCT-XYZ");
    },
  },
  {
    id: "m1-locations-batch",
    module: "m1-master-data",
    file: "m1-locations-batch-dialog-current.png",
    async run(page) {
      await openPage(page, "基础档案", /M1 库位管理/);
      const batch = page.getByRole("button", { name: /批量新增|批量/ }).first();
      if (!(await batch.count()) || !(await batch.isEnabled())) {
        // fallback: 新增 single location dialog if batch not present
        const add = page.getByRole("button", { name: "新增", exact: true }).first();
        if (!(await add.count()) || !(await add.isEnabled())) throw new Error("no location create/batch");
        await add.click({ timeout: 5_000 });
      } else {
        await batch.click({ timeout: 5_000 });
      }
      await page.getByRole("dialog").waitFor({ timeout: 5_000 });
    },
  },
  {
    id: "m1-dictionary-create",
    module: "m1-master-data",
    file: "m1-system-dictionary-create-dialog-current.png",
    async run(page) {
      await openPage(page, "基础档案", /M1 系统字典/);
      // dictionary may need type selected first
      const treeItem = page.getByRole("button", { name: /单据类型|document_type|特殊药品/ }).first();
      if (await treeItem.count()) {
        await treeItem.click({ timeout: 3_000 }).catch(() => {});
        await page.waitForTimeout(300);
      }
      const add = page.getByRole("button", { name: /新增/ }).first();
      if (!(await add.count()) || !(await add.isEnabled())) throw new Error("dictionary add disabled");
      await add.click({ timeout: 5_000 });
      await page.getByRole("dialog").waitFor({ timeout: 5_000 });
    },
  },

  // ---- M2 ----
  {
    id: "m2-receiving-detail",
    module: "m2-inbound",
    file: "m2-receiving-detail-current.png",
    async run(page) {
      await openPage(page, "入库业务", /M2 收货管理/);
      await selectFirstDataRow(page).catch(() => {});
      await clickToolbar(page, "详情");
      await page.getByRole("dialog").waitFor({ timeout: 8_000 });
    },
  },
  {
    id: "m2-receiving-dialog",
    module: "m2-inbound",
    file: "m2-receiving-dialog-current.png",
    async run(page) {
      await openPage(page, "入库业务", /M2 收货管理/);
      await selectFirstDataRow(page).catch(() => {});
      await clickToolbar(page, "收货");
      await page.getByRole("dialog").waitFor({ timeout: 8_000 });
    },
  },
  {
    id: "m2-receiving-reject",
    module: "m2-inbound",
    file: "m2-receiving-reject-dialog-current.png",
    async run(page) {
      await openPage(page, "入库业务", /M2 收货管理/);
      await selectFirstDataRow(page).catch(() => {});
      await clickToolbar(page, "收货");
      const dialog = page.getByRole("dialog");
      await dialog.waitFor({ timeout: 5_000 });
      // enable reject by filling reason if needed
      const reason = dialog.getByLabel(/拒收原因|原因/).first();
      if (await reason.count()) {
        await reason.fill("外包装破损，整单拒收", { timeout: 3_000 }).catch(() => {});
      } else {
        const ta = dialog.locator("textarea").first();
        if (await ta.count()) await ta.fill("外包装破损，整单拒收").catch(() => {});
      }
      const reject = dialog.getByRole("button", { name: /整单拒收/ }).first();
      if ((await reject.count()) && (await reject.isEnabled())) {
        // do not confirm reject (destructive) — just show enabled state by screenshot after fill
        await page.waitForTimeout(200);
      }
      // if still disabled, screenshot receive dialog with reject section visible
    },
  },
  {
    id: "m2-receiving-empty",
    module: "m2-inbound",
    file: "m2-receiving-filter-empty-current.png",
    async run(page) {
      await openPage(page, "入库业务", /M2 收货管理/);
      await filterEmpty(page, "NO-SUCH-ASN-XYZ");
    },
  },
  {
    id: "m2-inspecting-dialog",
    module: "m2-inbound",
    file: "m2-inspecting-dialog-current.png",
    async run(page) {
      await openPage(page, "入库业务", /M2 验收管理/);
      await selectFirstDataRow(page);
      const candidates = ["验收", "详情", "修改"];
      let opened = false;
      for (const name of candidates) {
        const btn = page.getByRole("button", { name, exact: true }).first();
        if ((await btn.count()) && (await btn.isEnabled())) {
          await btn.click({ timeout: 5_000 });
          opened = true;
          break;
        }
      }
      if (!opened) throw new Error("no enabled inspect action");
      await page.getByRole("dialog").waitFor({ timeout: 5_000 });
    },
  },
  {
    id: "m2-inspecting-sign",
    module: "m2-inbound",
    file: "m2-inspecting-sign-dialog-current.png",
    async run(page) {
      await openPage(page, "入库业务", /M2 验收管理/);
      await selectFirstDataRow(page);
      for (const name of ["双人签字", "签字", "验收"]) {
        const btn = page.getByRole("button", { name: new RegExp(`^${name}$`) }).first();
        if ((await btn.count()) && (await btn.isEnabled())) {
          await btn.click({ timeout: 5_000 });
          await page.waitForTimeout(400);
          if ((await page.getByRole("dialog").count()) > 0) return;
        }
      }
      throw new Error("no sign dialog entry");
    },
  },
  {
    id: "m2-putaway-dialog",
    module: "m2-inbound",
    file: "m2-putaway-dialog-current.png",
    async run(page) {
      await openPage(page, "入库业务", /M2 上架管理/);
      await selectFirstDataRow(page);
      for (const name of ["上架", "详情"]) {
        const btn = page.getByRole("button", { name, exact: true }).first();
        if ((await btn.count()) && (await btn.isEnabled())) {
          await btn.click({ timeout: 5_000 });
          break;
        }
      }
      await page.getByRole("dialog").waitFor({ timeout: 5_000 });
    },
  },

  // ---- M4 ----
  {
    id: "m4-order-detail",
    module: "m4-outbound",
    file: "m4-order-detail-current.png",
    async run(page) {
      await openPage(page, "出库业务", /M4 出库订单管理/);
      await selectFirstDataRow(page);
      await clickToolbar(page, "详情");
      await page.getByRole("dialog").waitFor({ timeout: 8_000 });
    },
  },
  {
    id: "m4-order-validate",
    module: "m4-outbound",
    file: "m4-order-action-dialog-current.png",
    async run(page) {
      await openPage(page, "出库业务", /M4 出库订单管理/);
      await selectFirstDataRow(page);
      const validate = page.getByRole("button", { name: /校验|重新校验/ }).first();
      if (!(await validate.count()) || !(await validate.isEnabled())) throw new Error("validate disabled");
      await validate.click();
      await page.getByRole("dialog").waitFor({ timeout: 8_000 });
    },
  },
  {
    id: "m4-order-empty",
    module: "m4-outbound",
    file: "m4-order-filter-empty-current.png",
    async run(page) {
      await openPage(page, "出库业务", /M4 出库订单管理/);
      await filterEmpty(page, "NO-SUCH-SO-XYZ");
    },
  },
  {
    id: "m4-wave-detail",
    module: "m4-outbound",
    file: "m4-wave-detail-current.png",
    async run(page) {
      await openPage(page, "出库业务", /M4 波次规划/);
      await selectFirstDataRow(page);
      await clickToolbar(page, "详情");
      await page.getByRole("dialog").waitFor({ timeout: 8_000 });
    },
  },
  {
    id: "m4-review-ship",
    module: "m4-outbound",
    file: "m4-review-ship-dialog-current.png",
    async run(page) {
      await openPage(page, "出库业务", /M4 复核发货/);
      await selectFirstDataRow(page);
      for (const name of ["发货交接", "发货", "复核", "详情"]) {
        const btn = page.getByRole("button", { name: new RegExp(name) }).first();
        if ((await btn.count()) && (await btn.isEnabled())) {
          await btn.click({ timeout: 5_000 });
          if ((await page.getByRole("dialog").count()) > 0) return;
        }
      }
      await page.getByRole("dialog").waitFor({ timeout: 5_000 });
    },
  },
  {
    id: "m4-return-dialog",
    module: "m4-outbound",
    file: "m4-purchase-return-dialog-current.png",
    async run(page) {
      await openPage(page, "出库业务", /M4 采购退货出库/);
      await selectFirstDataRow(page);
      const approve = page.getByRole("button", { name: /审批|详情/ }).first();
      if (!(await approve.count()) || !(await approve.isEnabled())) throw new Error("return action disabled");
      await approve.click();
      await page.getByRole("dialog").waitFor({ timeout: 8_000 });
    },
  },

  // ---- M3 ----
  {
    id: "m3-batches-empty",
    module: "m3-inventory",
    file: "m3-batches-filter-empty-current.png",
    async run(page) {
      await openPage(page, "库内业务", /M3 批号管理/);
      await filterEmpty(page, "NO-SUCH-BATCH-XYZ");
    },
  },

  // ---- H platform ----
  {
    id: "h1-menu-create",
    module: "h-platform",
    file: "h1-menu-create-dialog-current.png",
    async run(page) {
      await openPage(page, "基础能力", /H1 菜单管理/);
      const add = page.getByRole("button", { name: /新增/ }).first();
      if (!(await add.count()) || !(await add.isEnabled())) throw new Error("h1 add disabled");
      await add.click();
      await page.getByRole("dialog").waitFor({ timeout: 8_000 });
    },
  },
  {
    id: "h4-settings-edit",
    module: "h-platform",
    file: "h4-wechat-settings-dialog-current.png",
    async run(page) {
      await openPage(page, "基础能力", /H4 参数设置/);
      await selectFirstDataRow(page);
      for (const name of ["修改", "新增", "详情"]) {
        const btn = page.getByRole("button", { name, exact: true }).first();
        if ((await btn.count()) && (await btn.isEnabled())) {
          await btn.click({ timeout: 5_000 });
          break;
        }
      }
      await page.getByRole("dialog").waitFor({ timeout: 5_000 });
    },
  },
  {
    id: "h5-express-create",
    module: "h-platform",
    file: "h5-express-dialog-current.png",
    async run(page) {
      await openPage(page, "基础能力", /H5 快递对接/);
      await selectFirstDataRow(page).catch(() => {});
      for (const name of ["新增", "修改", "详情"]) {
        const btn = page.getByRole("button", { name, exact: true }).first();
        if ((await btn.count()) && (await btn.isEnabled())) {
          await btn.click({ timeout: 5_000 });
          break;
        }
      }
      await page.getByRole("dialog").waitFor({ timeout: 5_000 });
    },
  },
  {
    id: "h9-template-create",
    module: "h-platform",
    file: "h9-print-template-create-dialog-current.png",
    async run(page) {
      await openPage(page, "基础能力", /H9 打印模板/);
      const add = page.getByRole("button", { name: /新增/ }).first();
      if (!(await add.count()) || !(await add.isEnabled())) throw new Error("h9 add disabled");
      await add.click();
      await page.getByRole("dialog").waitFor({ timeout: 8_000 });
    },
  },
];

async function main() {
  ensureDir(OUT_ROOT);
  const browser = await chromium.launch({
    executablePath: CHROME,
    headless: true,
    args: ["--no-sandbox", "--disable-dev-shm-usage"],
  });
  const context = await browser.newContext({ viewport: VIEWPORT, deviceScaleFactor: 1 });
  const page = await context.newPage();
  await login(page);

  const results = [];
  page.setDefaultTimeout(8_000);
  page.setDefaultNavigationTimeout(20_000);

  const scenes = ONLY.size ? SCENES.filter((s) => ONLY.has(s.id)) : SCENES;
  for (const scene of scenes) {
    const started = Date.now();
    try {
      await closeDialog(page);
      await clearFilter(page).catch(() => {});
      // per-scene hard timeout
      await Promise.race([
        scene.run(page),
        new Promise((_, reject) => setTimeout(() => reject(new Error("scene timeout 45s")), 45_000)),
      ]);
      const out = await shot(page, scene.module, scene.file);
      await closeDialog(page);
      await clearFilter(page).catch(() => {});
      results.push({ id: scene.id, module: scene.module, file: scene.file, ok: true, path: out, ms: Date.now() - started, error: null });
      console.log(`OK  ${scene.module}/${scene.file} (${Date.now() - started}ms)`);
    } catch (err) {
      await closeDialog(page).catch(() => {});
      // recover stuck page
      try {
        await page.goto(BASE_URL + "/", { waitUntil: "domcontentloaded", timeout: 15_000 });
        if ((await page.getByRole("button", { name: /退出/ }).count()) === 0) {
          await login(page);
        }
      } catch {
        await login(page).catch(() => {});
      }
      results.push({
        id: scene.id,
        module: scene.module,
        file: scene.file,
        ok: false,
        path: null,
        ms: Date.now() - started,
        error: String(err?.message || err),
      });
      console.error(`FAIL ${scene.module}/${scene.file}: ${err?.message || err}`);
    }
  }

  const report = {
    captured_at: new Date().toISOString(),
    base_url: BASE_URL,
    viewport: VIEWPORT,
    data_source: "WMS_WEB_ADMIN_DEV_MOCK (running 9002 session)",
    kind: "interaction",
    total: results.length,
    ok: results.filter((r) => r.ok).length,
    failed: results.filter((r) => !r.ok).length,
    results,
  };
  fs.writeFileSync(path.join(OUT_ROOT, "capture-interaction-report.json"), JSON.stringify(report, null, 2));

  // merge note into main report if present
  const mainReportPath = path.join(OUT_ROOT, "capture-report.json");
  if (fs.existsSync(mainReportPath)) {
    try {
      const main = JSON.parse(fs.readFileSync(mainReportPath, "utf8"));
      main.interaction = { ok: report.ok, failed: report.failed, total: report.total, captured_at: report.captured_at };
      fs.writeFileSync(mainReportPath, JSON.stringify(main, null, 2));
    } catch {
      /* ignore */
    }
  }

  await browser.close();
  console.log(JSON.stringify({ ok: report.ok, failed: report.failed, total: report.total }, null, 2));
  process.exit(report.failed > 0 ? 1 : 0);
}

main().catch((e) => {
  console.error(e);
  process.exit(2);
});
