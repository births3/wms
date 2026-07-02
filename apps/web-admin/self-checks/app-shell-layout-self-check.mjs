import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const appShell = readFileSync(resolve(__dirname, "../src/App.tsx"), "utf8");

assert.match(appShell, /lg:grid-cols-\[14rem_1fr\]/, "桌面菜单栏宽度应为 14rem");
assert.doesNotMatch(appShell, /lg:grid-cols-\[16rem_1fr\]/, "桌面菜单栏不能回退到 16rem");

console.log("app shell layout self-check passed");
