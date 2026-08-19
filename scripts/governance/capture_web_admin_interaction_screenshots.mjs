#!/usr/bin/env node
/**
 * Capture dialog / empty-state screenshots from apps/web-admin on 9002.
 * Complements capture_web_admin_real_screenshots.mjs (first-screen only).
 */

import {
  BASE_URL,
  CHROME,
  OUT_ROOT,
  VIEWPORT,
  chromium,
  clearFilter,
  clickContentButton,
  clickToolbar,
  closeDialog,
  ensureDir,
  filterEmpty,
  firstVisible,
  fs,
  login,
  openPage,
  path,
  selectFirstDataRow,
  shot,
} from "./capture_web_admin_interaction_helpers.mjs";

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
      // 工具栏「批量」/「批量新增」
      const batch = await firstVisible(page.getByRole("button", { name: /批量/ }));
      if (batch && (await batch.isEnabled())) {
        await batch.click({ timeout: 5_000 });
      } else {
        await clickToolbar(page, "新增");
      }
      await page.getByRole("dialog").waitFor({ timeout: 8_000 });
    },
  },
  {
    id: "m1-dictionary-create",
    module: "m1-master-data",
    file: "m1-system-dictionary-create-dialog-current.png",
    async run(page) {
      await openPage(page, "基础档案", /M1 系统字典/);
      // 默认已选 special_drug_category；直接点内容区「新增」
      await clickContentButton(page, "新增");
      await page.getByRole("dialog").waitFor({ timeout: 8_000 });
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
      // 验收动作可能与状态筛选标签同名，优先点工具栏 enabled 的「验收」
      await clickToolbar(page, "验收");
      await page.getByRole("dialog").waitFor({ timeout: 8_000 });
    },
  },
  {
    id: "m2-inspecting-sign",
    module: "m2-inbound",
    file: "m2-inspecting-sign-dialog-current.png",
    async run(page) {
      await openPage(page, "入库业务", /M2 验收管理/);
      await selectFirstDataRow(page);
      await clickToolbar(page, "验收");
      const dialog = page.getByRole("dialog");
      await dialog.waitFor({ timeout: 8_000 });
      // 签字字段已内嵌在验收弹窗；截到含双人签字区即可
      await dialog.getByText(/第一签字人|第二人账号/).first().waitFor({ timeout: 5_000 });
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
      // 双击单号或点详情
      const orderLink = page.locator("tbody .text-primary, tbody td").filter({ hasText: /SO-/ }).first();
      if (await orderLink.isVisible().catch(() => false)) {
        await orderLink.dblclick({ timeout: 5_000 }).catch(async () => {
          await clickToolbar(page, "详情");
        });
      } else {
        await clickToolbar(page, "详情");
      }
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
      try {
        await clickToolbar(page, "修改");
      } catch {
        await clickToolbar(page, "新增");
      }
      await page.getByRole("dialog").waitFor({ timeout: 8_000 });
    },
  },
  {
    id: "h5-express-create",
    module: "h-platform",
    file: "h5-express-dialog-current.png",
    async run(page) {
      await openPage(page, "基础能力", /H5 快递对接/);
      // 快递商配置区「新增」（页面有承运商/规则两处新增，取内容区第一个）
      await clickContentButton(page, "新增");
      await page.getByRole("dialog").waitFor({ timeout: 8_000 });
    },
  },
  {
    id: "h9-template-create",
    module: "h-platform",
    file: "h9-print-template-create-dialog-current.png",
    async run(page) {
      await openPage(page, "基础能力", /H9 打印模板/);
      // createAction 在 types/libraries pending 时 disabled — 等到可点
      await clickContentButton(page, "新增", { waitEnabledMs: 15_000 });
      await page.getByRole("dialog").waitFor({ timeout: 15_000 });
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
  const results = [];

  const scenes = ONLY.size ? SCENES.filter((s) => ONLY.has(s.id)) : SCENES;
  for (const scene of scenes) {
    const started = Date.now();
    const context = await browser.newContext({ viewport: VIEWPORT, deviceScaleFactor: 1 });
    const page = await context.newPage();
    page.setDefaultTimeout(8_000);
    page.setDefaultNavigationTimeout(20_000);
    try {
      await login(page);
      // per-scene hard timeout
      await Promise.race([
        scene.run(page),
        new Promise((_, reject) => setTimeout(() => reject(new Error("scene timeout 60s")), 60_000)),
      ]);
      const out = await shot(page, scene.module, scene.file);
      await closeDialog(page);
      await clearFilter(page).catch(() => {});
      results.push({ id: scene.id, module: scene.module, file: scene.file, ok: true, path: out, ms: Date.now() - started, error: null });
      console.log(`OK  ${scene.module}/${scene.file} (${Date.now() - started}ms)`);
    } catch (err) {
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
    } finally {
      await context.close();
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
