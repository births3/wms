import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

interface MatrixInput {
  baseUrl: string;
  artifactsDir: string;
  scenarios: MatrixScenario[];
}

interface MatrixScenario {
  tab: string;
  urlHash: string;
  viewport: { width: number; height: number };
  file: string;
  expectedKeywords: string[];
  minKeywordHitRatio: number;
  requiredSelectors: string[];
  maxHorizontalOverflowPx: number;
  forbidConsoleErrors: boolean;
  detectTextOverflow: boolean;
  detectControlOverlap: boolean;
  detectVerticalCjkTable: boolean;
  clickStrategy: "first-main-button" | "none";
}

interface CheckResult {
  tab: string;
  status: "passed" | "failed";
  issues: string[];
  warnings: string[];
  keywordHits: string[];
  keywordMisses: string[];
  screenshots: string[];
}

const inputPath = process.env.MATRIX_E2E_INPUT;
if (!inputPath) {
  throw new Error("MATRIX_E2E_INPUT is required");
}

const matrixInput = JSON.parse(fs.readFileSync(inputPath, "utf8")) as MatrixInput;
const resultsDir = path.join(matrixInput.artifactsDir, "results");
const screenshotDir = path.join(matrixInput.artifactsDir, "screenshots");
fs.mkdirSync(resultsDir, { recursive: true });
fs.mkdirSync(screenshotDir, { recursive: true });

function normalizeText(value: string) {
  return value
    .replace(/×/g, "x")
    .replace(/\s+/g, " ")
    .trim()
    .toUpperCase();
}

function safeName(tab: string, state: string) {
  return `${tab}.${state}.png`;
}

async function collectDomHealth(page: Page, scenario: MatrixScenario) {
  return page.evaluate((opts) => {
    const issues: string[] = [];
    const warnings: string[] = [];

    const html = document.documentElement;
    const horizontalOverflow = Math.max(0, html.scrollWidth - window.innerWidth);
    if (horizontalOverflow > opts.maxHorizontalOverflowPx) {
      issues.push(`document horizontal overflow ${horizontalOverflow}px > ${opts.maxHorizontalOverflowPx}px`);
    }

    if (opts.detectTextOverflow) {
      const selectors = [
        "main button",
        "main th",
        "main td",
        "main input",
        "main [role='button']",
        "main [data-e2e-check-text]",
      ].join(",");
      const overflowNodes = Array.from(document.querySelectorAll<HTMLElement>(selectors))
        .filter((el) => {
          const rect = el.getBoundingClientRect();
          if (rect.width < 8 || rect.height < 8) return false;
          const style = window.getComputedStyle(el);
          if (style.display === "none" || style.visibility === "hidden") return false;
          if (style.overflowX === "hidden" || style.textOverflow === "ellipsis") return false;
          return el.scrollWidth > el.clientWidth + 2;
        })
        .slice(0, 12)
        .map((el) => `${el.tagName.toLowerCase()} "${(el.textContent ?? "").trim().slice(0, 40)}" scrollWidth=${el.scrollWidth} clientWidth=${el.clientWidth}`);
      if (overflowNodes.length > 0) {
        issues.push(`text overflow: ${overflowNodes.join(" | ")}`);
      }
    }

    if (opts.detectControlOverlap) {
      const controls = Array.from(document.querySelectorAll<HTMLElement>("main button, main a, main input, main select, main textarea, main [role='button']"))
        .map((el) => ({ el, rect: el.getBoundingClientRect(), text: (el.textContent ?? el.getAttribute("aria-label") ?? "").trim() }))
        .filter(({ rect }) => rect.width >= 8 && rect.height >= 8);
      const overlaps: string[] = [];
      for (let i = 0; i < controls.length; i += 1) {
        for (let j = i + 1; j < controls.length; j += 1) {
          const a = controls[i].rect;
          const b = controls[j].rect;
          const x = Math.max(0, Math.min(a.right, b.right) - Math.max(a.left, b.left));
          const y = Math.max(0, Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top));
          const area = x * y;
          if (area <= 0) continue;
          const minArea = Math.min(a.width * a.height, b.width * b.height);
          if (minArea > 0 && area / minArea > 0.5) {
            overlaps.push(`"${controls[i].text.slice(0, 24)}" overlaps "${controls[j].text.slice(0, 24)}"`);
          }
        }
      }
      if (overlaps.length > 0) {
        issues.push(`control overlap: ${overlaps.slice(0, 8).join(" | ")}`);
      }
    }

    if (opts.detectVerticalCjkTable) {
      const badCells = Array.from(document.querySelectorAll<HTMLElement>("main table th, main table td"))
        .map((el) => {
          const text = (el.textContent ?? "").replace(/\s+/g, "");
          const cjkCount = Array.from(text).filter((ch) => /[\u3400-\u9fff]/.test(ch)).length;
          const rect = el.getBoundingClientRect();
          return { text, cjkCount, width: rect.width, height: rect.height };
        })
        .filter((cell) => cell.cjkCount >= 3 && cell.width / cell.cjkCount < 10 && cell.height / cell.cjkCount > 10)
        .slice(0, 12)
        .map((cell) => `"${cell.text.slice(0, 24)}" width=${cell.width.toFixed(1)} cjk=${cell.cjkCount}`);
      if (badCells.length > 0) {
        issues.push(`vertical CJK table text: ${badCells.join(" | ")}`);
      }
    }

    return { issues, warnings };
  }, {
    maxHorizontalOverflowPx: scenario.maxHorizontalOverflowPx,
    detectTextOverflow: scenario.detectTextOverflow,
    detectControlOverlap: scenario.detectControlOverlap,
    detectVerticalCjkTable: scenario.detectVerticalCjkTable,
  });
}

async function maybeClickPrimaryAction(page: Page, scenario: MatrixScenario) {
  if (scenario.clickStrategy === "none") return "skipped";

  const candidates = [
    { scope: "dialog", button: page.locator("[role='dialog'] button:not([disabled])").first() },
    { scope: "main", button: page.locator("main button:not([disabled])").first() },
  ];

  for (const candidate of candidates) {
    if ((await candidate.button.count()) === 0) continue;
    if (!(await candidate.button.isVisible({ timeout: 500 }).catch(() => false))) continue;
    try {
      await candidate.button.click({ timeout: 3_000 });
      await page.waitForTimeout(200);
      return `clicked:${candidate.scope}`;
    } catch (error) {
      const message = error instanceof Error ? error.message.split("\n")[0] : String(error);
      return `click-failed:${candidate.scope}:${message}`;
    }
  }

  return "no-button";
}

for (const scenario of matrixInput.scenarios) {
  test(`${scenario.tab}`, async ({ page }) => {
    const consoleErrors: string[] = [];
    const pageErrors: string[] = [];
    page.on("console", (message) => {
      if (message.type() === "error") consoleErrors.push(message.text());
    });
    page.on("pageerror", (error) => pageErrors.push(error.message));

    const issues: string[] = [];
    const warnings: string[] = [];
    const screenshots: string[] = [];

    await page.setViewportSize(scenario.viewport);
    await page.goto(`${matrixInput.baseUrl}/${scenario.urlHash}`, { waitUntil: "networkidle" });
    await expect(page.locator("main")).toBeVisible();
    await page.waitForTimeout(300);

    for (const selector of scenario.requiredSelectors) {
      await expect(page.locator(selector).first(), `${scenario.tab}: required selector ${selector}`).toBeVisible();
    }

    const bodyText = normalizeText(await page.locator("body").innerText());
    const keywordHits = scenario.expectedKeywords.filter((keyword) => bodyText.includes(normalizeText(keyword)));
    const keywordMisses = scenario.expectedKeywords.filter((keyword) => !bodyText.includes(normalizeText(keyword)));
    const keywordRatio = scenario.expectedKeywords.length === 0 ? 1 : keywordHits.length / scenario.expectedKeywords.length;
    if (keywordRatio < scenario.minKeywordHitRatio) {
      issues.push(`keyword hit ratio ${keywordHits.length}/${scenario.expectedKeywords.length} (${keywordRatio.toFixed(2)}) < ${scenario.minKeywordHitRatio}`);
    }

    const initialPath = path.join(screenshotDir, safeName(scenario.tab, "initial"));
    await page.screenshot({ path: initialPath, fullPage: false });
    screenshots.push(initialPath);

    const domHealth = await collectDomHealth(page, scenario);
    issues.push(...domHealth.issues.map((issue) => `initial ${issue}`));
    warnings.push(...domHealth.warnings.map((warning) => `initial ${warning}`));

    const interaction = await maybeClickPrimaryAction(page, scenario);
    warnings.push(`interaction=${interaction}`);
    if (interaction.startsWith("click-failed:")) {
      issues.push(`interaction ${interaction}`);
    }

    const afterPath = path.join(screenshotDir, safeName(scenario.tab, "after-interaction"));
    await page.screenshot({ path: afterPath, fullPage: false });
    screenshots.push(afterPath);

    const afterDomHealth = await collectDomHealth(page, scenario);
    issues.push(...afterDomHealth.issues.map((issue) => `after-interaction ${issue}`));
    warnings.push(...afterDomHealth.warnings.map((warning) => `after-interaction ${warning}`));

    if (scenario.forbidConsoleErrors) {
      if (consoleErrors.length > 0) issues.push(`console errors: ${consoleErrors.slice(0, 5).join(" | ")}`);
      if (pageErrors.length > 0) issues.push(`page errors: ${pageErrors.slice(0, 5).join(" | ")}`);
    }

    const result: CheckResult = {
      tab: scenario.tab,
      status: issues.length === 0 ? "passed" : "failed",
      issues,
      warnings,
      keywordHits,
      keywordMisses,
      screenshots,
    };
    fs.writeFileSync(path.join(resultsDir, `${scenario.tab}.json`), JSON.stringify(result, null, 2));

    expect(issues, issues.join("\n")).toEqual([]);
  });
}
