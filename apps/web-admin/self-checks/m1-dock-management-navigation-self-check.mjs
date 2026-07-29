import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const app = fs.readFileSync(path.join(root, "src/App.tsx"), "utf8");
const renderer = fs.readFileSync(path.join(root, "src/app-shell/AdminViewRenderer.tsx"), "utf8");
const page = fs.readFileSync(path.join(root, "src/pages/dock/DockManagementPage.tsx"), "utf8");

if (!app.includes('id: "dock-management"') || !app.includes('title: "M1 月台管理"')) {
  throw new Error("M1 月台管理未登记到管理端菜单");
}
if (!renderer.includes('view === "dock-management"')) {
  throw new Error("M1 月台管理菜单未挂载页面渲染器");
}
if (!page.includes('accept=".xlsx,.csv"')) {
  throw new Error("M1 月台导入只应声明真实支持的 .xlsx/.csv");
}
if (/accept="[^"]*\.xls,/.test(page)) {
  throw new Error("M1 月台导入不得继续声明不支持的二进制 .xls");
}

console.log("m1-dock-management-navigation-self-check: passed");
