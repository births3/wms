import assert from "node:assert/strict";

import { maskSensitiveDisplayValue } from "./mask-sensitive.ts";

assert.equal(maskSensitiveDisplayValue(null), undefined);
assert.equal(maskSensitiveDisplayValue(""), undefined);
assert.equal(maskSensitiveDisplayValue("   "), undefined);
assert.equal(maskSensitiveDisplayValue("1234567"), "*******");
assert.equal(maskSensitiveDisplayValue("13800000000"), "138****0000");
assert.equal(maskSensitiveDisplayValue("320101199001011234"), "320***********1234");
assert.equal(maskSensitiveDisplayValue("138****0000"), "138****0000");
