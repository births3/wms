// 本目录为 node:test 直跑脚本，类型解析见同目录 tsconfig.json
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const dialogSource = readFileSync(new URL("../src/ui/dialog.tsx", import.meta.url), "utf8");

assert.match(dialogSource, /export const DialogOverlay = React\.forwardRef/);
assert.doesNotMatch(dialogSource, /<DialogPrimitive\.Close asChild>[\s\S]*<DialogPrimitive\.Overlay/);
assert.match(dialogSource, /pointer-events-none/);
assert.match(dialogSource, /<DialogOverlay \/>[\s\S]*<DialogPrimitive\.Content/);
assert.match(dialogSource, /<DialogPrimitive\.Close className="absolute right-4 top-4/);
// 中文系统的屏幕阅读器必须读到中文关闭说明
assert.match(dialogSource, /<span className="sr-only">关闭<\/span>/);
