import assert from "node:assert/strict";
import { once } from "node:events";
import { test } from "node:test";

import {
  closeRenderBrowser,
  createRenderServer,
  renderPdf,
} from "../src/server.mjs";

const template = {
  panels: [
    {
      index: 0,
      paperType: "A4",
      width: 210,
      height: 297,
      printElements: [
        {
          options: {
            field: "wms_order_no",
            title: "出库单号",
            left: 20,
            top: 20,
            width: 260,
            height: 24,
            fontSize: 18,
          },
          printElementType: { type: "text" },
        },
      ],
    },
  ],
};

test("hiprint 在 Chromium 中渲染真实 PDF", async (t) => {
  t.after(closeRenderBrowser);
  const pdf = await renderPdf({
    template,
    data: { wms_order_no: "OUT-H9-WORKER-001" },
  });

  assert.equal(pdf.subarray(0, 5).toString(), "%PDF-");
  assert.ok(pdf.length > 5_000, "浏览器渲染结果不能是文本占位 PDF");
});

test("HTTP 端点拒绝缺失令牌和非法模板", async (t) => {
  const server = createRenderServer({
    token: "render-worker-test-token",
    host: "127.0.0.1",
    port: 0,
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  t.after(() => server.close());

  const address = server.address();
  assert.ok(address && typeof address === "object");
  const endpoint = `http://127.0.0.1:${address.port}/render`;
  const unauthorized = await fetch(endpoint, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ template, data: {} }),
  });
  assert.equal(unauthorized.status, 401);

  const invalid = await fetch(endpoint, {
    method: "POST",
    headers: {
      authorization: "Bearer render-worker-test-token",
      "content-type": "application/json",
    },
    body: JSON.stringify({ template: { panels: [] }, data: {} }),
  });
  assert.equal(invalid.status, 422);
});
