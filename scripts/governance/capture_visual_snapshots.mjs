import fs from "node:fs/promises";
import process from "node:process";
import { createRequire } from "node:module";

const require = createRequire(new URL("../../prototypes/package.json", import.meta.url));
const { chromium } = require("@playwright/test");

async function readStdin() {
  let input = "";
  process.stdin.setEncoding("utf8");
  for await (const chunk of process.stdin) input += chunk;
  return JSON.parse(input);
}

async function main() {
  const payload = await readStdin();
  const browser = await chromium.launch({
    executablePath: payload.chrome,
    headless: true,
    args: ["--no-sandbox", "--hide-scrollbars"],
  });
  const results = [];

  try {
    for (const job of payload.jobs) {
      await fs.rm(job.out_file, { force: true });
      const context = await browser.newContext({
        viewport: { width: job.width, height: job.viewport_height },
      });
      try {
        const page = await context.newPage();
        await page.goto(`${payload.base_url}${job.url_hash}`, {
          waitUntil: "load",
          timeout: 30_000,
        });
        await page.waitForTimeout(2_000);
        await page.screenshot({ path: job.out_file, fullPage: false });
        results.push({ tab: job.tab, ok: true, error: "" });
      } catch (error) {
        await fs.rm(job.out_file, { force: true });
        results.push({
          tab: job.tab,
          ok: false,
          error: String(error instanceof Error ? error.message : error).slice(0, 400),
        });
      } finally {
        await context.close();
      }
    }
  } finally {
    await browser.close();
  }

  process.stdout.write(JSON.stringify({ results }));
}

main().catch((error) => {
  process.stderr.write(`${String(error instanceof Error ? error.stack : error)}\n`);
  process.exitCode = 1;
});
