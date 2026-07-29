import crypto from "node:crypto";
import fs from "node:fs";
import http from "node:http";
import { createRequire } from "node:module";
import path from "node:path";
import { pathToFileURL } from "node:url";

import { chromium } from "playwright-core";

const DEFAULT_MAX_BODY_BYTES = 2 * 1024 * 1024;
const DEFAULT_MAX_CONCURRENCY = 2;
const DEFAULT_RENDER_TIMEOUT_MS = 30_000;
const require = createRequire(import.meta.url);
const hiprintMain = require.resolve("hiprint");
const hiprintRequire = createRequire(hiprintMain);
const assets = resolveBrowserAssets();

let browserPromise;

export async function renderPdf(input, options = {}) {
  validateRenderInput(input);
  const timeoutMs = positiveInteger(
    options.timeoutMs ?? process.env.WMS_H9_RENDER_TIMEOUT_MS,
    DEFAULT_RENDER_TIMEOUT_MS,
  );
  const browser = await getBrowser();
  const page = await browser.newPage();
  try {
    page.setDefaultTimeout(timeoutMs);
    await page.route("**/*", async (route) => {
      const url = route.request().url();
      if (url === "http://render-worker.internal/") {
        await route.fulfill({
          body: "<!doctype html><html><head><meta charset=\"utf-8\"></head><body></body></html>",
          contentType: "text/html; charset=utf-8",
          status: 200,
        });
        return;
      }
      const protocol = new URL(url).protocol;
      if (["about:", "blob:", "data:"].includes(protocol)) {
        await route.continue();
      } else {
        await route.abort("blockedbyclient");
      }
    });
    await page.goto("http://render-worker.internal/");
    await page.addStyleTag({ path: assets.css });
    for (const script of assets.scripts) {
      await page.addScriptTag({ path: script });
    }
    const base64 = await withTimeout(
      page.evaluate(async ({ template, data }) => {
        const module = window["vue-plugin-hiprint"];
        if (!module?.hiprint || !module.defaultElementTypeProvider) {
          throw new Error("hiprint browser bundle did not initialize");
        }
        module.disAutoConnect();
        module.hiprint.init({
          providers: [new module.defaultElementTypeProvider()],
        });
        const printTemplate = new module.hiprint.PrintTemplate({ template });
        const blob = await new Promise((resolve, reject) => {
          const operation = printTemplate.toPdf(data, "wms-category", {
            isDownload: false,
            scale: 2,
          });
          operation.then(resolve);
          operation.fail(reject);
        });
        return await new Promise((resolve, reject) => {
          const reader = new FileReader();
          reader.onload = () => resolve(String(reader.result).split(",", 2)[1]);
          reader.onerror = () => reject(reader.error);
          reader.readAsDataURL(blob);
        });
      }, input),
      timeoutMs,
    );
    const pdf = Buffer.from(base64, "base64");
    if (!pdf.subarray(0, 5).equals(Buffer.from("%PDF-"))) {
      throw new Error("renderer returned an invalid PDF");
    }
    return pdf;
  } finally {
    await page.close();
  }
}

export function createRenderServer(options = {}) {
  const token = options.token ?? process.env.WMS_H9_RENDER_TOKEN;
  if (!token?.trim()) {
    throw new Error("WMS_H9_RENDER_TOKEN is required");
  }
  const maxBodyBytes = positiveInteger(
    options.maxBodyBytes ?? process.env.WMS_H9_RENDER_MAX_BODY_BYTES,
    DEFAULT_MAX_BODY_BYTES,
  );
  const maxConcurrency = positiveInteger(
    options.maxConcurrency ?? process.env.WMS_H9_RENDER_MAX_CONCURRENCY,
    DEFAULT_MAX_CONCURRENCY,
  );
  let activeRenders = 0;

  return http.createServer(async (request, response) => {
    if (request.method === "GET" && request.url === "/healthz") {
      try {
        await getBrowser();
        sendJson(response, 200, { status: "ok" });
      } catch {
        sendJson(response, 503, { status: "unavailable" });
      }
      return;
    }
    if (request.method !== "POST" || request.url !== "/render") {
      sendJson(response, 404, { code: "NOT_FOUND" });
      return;
    }
    if (!authorized(request.headers.authorization, token)) {
      sendJson(response, 401, { code: "UNAUTHORIZED" });
      return;
    }
    if (!request.headers["content-type"]?.startsWith("application/json")) {
      sendJson(response, 415, { code: "JSON_REQUIRED" });
      return;
    }
    if (activeRenders >= maxConcurrency) {
      sendJson(response, 429, { code: "RENDER_CAPACITY_EXCEEDED" });
      return;
    }

    activeRenders += 1;
    try {
      const body = await readJsonBody(request, maxBodyBytes);
      const pdf = await renderPdf(body, options);
      response.writeHead(200, {
        "cache-control": "no-store",
        "content-length": pdf.length,
        "content-type": "application/pdf",
        "x-content-type-options": "nosniff",
        "x-wms-render-engine": "hiprint-chromium",
      });
      response.end(pdf);
    } catch (error) {
      const status = error instanceof RequestError ? error.status : 502;
      const code = error instanceof RequestError
        ? error.code
        : "RENDER_FAILED";
      sendJson(response, status, { code });
    } finally {
      activeRenders -= 1;
    }
  });
}

export async function closeRenderBrowser() {
  if (!browserPromise) return;
  const browser = await browserPromise;
  browserPromise = undefined;
  await browser.close();
}

async function getBrowser() {
  if (!browserPromise) {
    const executablePath =
      process.env.WMS_H9_RENDER_CHROMIUM_EXECUTABLE
      ?? (fs.existsSync("/usr/bin/google-chrome")
        ? "/usr/bin/google-chrome"
        : fs.existsSync("/usr/bin/chromium")
          ? "/usr/bin/chromium"
          : undefined);
    browserPromise = chromium
      .launch({
        executablePath,
        headless: true,
        args: ["--disable-dev-shm-usage"],
      })
      .then((browser) => {
        browser.once("disconnected", () => {
          browserPromise = undefined;
        });
        return browser;
      });
  }
  try {
    return await browserPromise;
  } catch (error) {
    browserPromise = undefined;
    throw error;
  }
}

function validateRenderInput(input) {
  if (!isPlainObject(input) || !isPlainObject(input.template) || !isPlainObject(input.data)) {
    throw new RequestError(422, "INVALID_RENDER_INPUT");
  }
  const panels = input.template.panels;
  if (!Array.isArray(panels) || panels.length === 0 || panels.length > 20) {
    throw new RequestError(422, "INVALID_TEMPLATE");
  }
  rejectUnsafeValues(input.template);
}

function rejectUnsafeValues(value) {
  if (typeof value === "string") {
    const normalized = value.trim().toLowerCase();
    if (
      normalized.startsWith("http:")
      || normalized.startsWith("https:")
      || normalized.startsWith("file:")
      || normalized.startsWith("//")
      || normalized.startsWith("javascript:")
    ) {
      throw new RequestError(422, "EXTERNAL_RESOURCE_FORBIDDEN");
    }
    return;
  }
  if (!value || typeof value !== "object") return;
  for (const [childKey, child] of Object.entries(value)) {
    if (["__proto__", "constructor", "prototype"].includes(childKey)) {
      throw new RequestError(422, "UNSAFE_TEMPLATE_KEY");
    }
    rejectUnsafeValues(child);
  }
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

async function readJsonBody(request, maxBodyBytes) {
  const declaredLength = Number(request.headers["content-length"] ?? 0);
  if (declaredLength > maxBodyBytes) {
    throw new RequestError(413, "REQUEST_TOO_LARGE");
  }
  let size = 0;
  const chunks = [];
  for await (const chunk of request) {
    size += chunk.length;
    if (size > maxBodyBytes) {
      throw new RequestError(413, "REQUEST_TOO_LARGE");
    }
    chunks.push(chunk);
  }
  try {
    return JSON.parse(Buffer.concat(chunks).toString("utf8"));
  } catch {
    throw new RequestError(400, "INVALID_JSON");
  }
}

function authorized(value, token) {
  if (!value?.startsWith("Bearer ")) return false;
  const supplied = Buffer.from(value.slice("Bearer ".length));
  const expected = Buffer.from(token);
  return supplied.length === expected.length && crypto.timingSafeEqual(supplied, expected);
}

function positiveInteger(value, fallback) {
  const parsed = Number(value);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : fallback;
}

async function withTimeout(operation, timeoutMs) {
  let timeout;
  try {
    return await Promise.race([
      operation,
      new Promise((_, reject) => {
        timeout = setTimeout(() => reject(new Error("render timeout")), timeoutMs);
      }),
    ]);
  } finally {
    clearTimeout(timeout);
  }
}

function sendJson(response, status, value) {
  const body = Buffer.from(JSON.stringify(value));
  response.writeHead(status, {
    "cache-control": "no-store",
    "content-length": body.length,
    "content-type": "application/json; charset=utf-8",
    "x-content-type-options": "nosniff",
  });
  response.end(body);
}

function resolveBrowserAssets() {
  const socketPackage = path.dirname(
    hiprintRequire.resolve("socket.io-client/package.json"),
  );
  const canvgMain = hiprintRequire.resolve("canvg");
  return {
    css: path.join(path.dirname(hiprintMain), "print-lock.css"),
    scripts: [
      hiprintRequire.resolve("jquery/dist/jquery.min.js"),
      hiprintRequire.resolve("jsbarcode/dist/JsBarcode.all.min.js"),
      path.join(socketPackage, "dist/socket.io.min.js"),
      hiprintRequire.resolve("jspdf/dist/jspdf.umd.min.js"),
      hiprintRequire.resolve("html2canvas/dist/html2canvas.min.js"),
      path.join(path.dirname(canvgMain), "umd.js"),
      hiprintMain,
    ],
  };
}

class RequestError extends Error {
  constructor(status, code) {
    super(code);
    this.status = status;
    this.code = code;
  }
}

async function start() {
  const host = process.env.WMS_H9_RENDER_HOST ?? "127.0.0.1";
  const port = positiveInteger(process.env.WMS_H9_RENDER_PORT, 18090);
  const server = createRenderServer();
  server.listen(port, host, () => {
    process.stdout.write(`H9 Render Worker listening on ${host}:${port}\n`);
  });
  const stop = async () => {
    server.close();
    await closeRenderBrowser();
  };
  process.once("SIGINT", stop);
  process.once("SIGTERM", stop);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await start();
}
