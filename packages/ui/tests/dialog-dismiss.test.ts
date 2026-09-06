// 本目录为 node:test 直跑脚本，类型解析见同目录 tsconfig.json
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const dialogSource = readFileSync(new URL("../src/ui/dialog.tsx", import.meta.url), "utf8");

assert.match(dialogSource, /export const DialogOverlay = React\.forwardRef/);
assert.doesNotMatch(dialogSource, /<DialogPrimitive\.Close asChild>[\s\S]*<DialogPrimitive\.Overlay/);
assert.match(dialogSource, /pointer-events-none/);
assert.match(dialogSource, /<DialogOverlay \/>[\s\S]*<DialogPrimitive\.Content/);
assert.match(dialogSource, /<DialogPrimitive\.Close className="absolute right-4 top-4/);
// 中文系统的图标关闭按钮使用独立可访问名称，避免与页脚“关闭”动作重名。
assert.match(dialogSource, /<span className="sr-only">关闭弹窗<\/span>/);
