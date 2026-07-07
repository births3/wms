import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const dialogSource = readFileSync(new URL("../src/ui/dialog.tsx", import.meta.url), "utf8");

assert.match(dialogSource, /export const DialogOverlay = React\.forwardRef/);
assert.doesNotMatch(dialogSource, /<DialogPrimitive\.Close asChild>[\s\S]*<DialogPrimitive\.Overlay/);
assert.match(dialogSource, /pointer-events-none/);
assert.match(dialogSource, /<DialogOverlay \/>[\s\S]*<DialogPrimitive\.Content/);
assert.match(dialogSource, /<DialogPrimitive\.Close className="absolute right-4 top-4/);
