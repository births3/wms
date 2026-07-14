import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const app = fs.readFileSync(path.join(root, "src/App.tsx"), "utf8");
const renderer = fs.readFileSync(path.join(root, "src/app-shell/AdminViewRenderer.tsx"), "utf8");

if (!app.includes('id: "dock-management"') || !app.includes('title: "M1 月台管理"')) {
  throw new Error("M1 月台管理未登记到管理端菜单");
}
if (!renderer.includes('view === "dock-management"')) {
  throw new Error("M1 月台管理菜单未挂载页面渲染器");
}

console.log("m1-dock-management-navigation-self-check: passed");
