import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";

const root = new URL("..", import.meta.url);
const read = (file) => readFileSync(new URL(file, root), "utf8");
const page = read("src/pages/task-engine/TaskTypeConfigPage.tsx");
const queries = read("src/features/task-engine/task-type-queries.ts");

assert.match(page, /<QueryPanel/);
assert.match(page, /<DataGrid[\s\S]*storageKey="mte\.task-types"/);
assert.match(page, /新增任务类型|编辑任务类型/);
for (const field of ["default_priority", "estimated_minutes", "mergeable", "insertable", "enabled"]) assert.match(queries + page, new RegExp(field));
assert.match(queries, /task-engine\/task-types/);
assert.match(queries, /Idempotency-Key/);
assert.match(page, /类型编码必须以字母或数字开头/);
assert.match(page, /读取任务类型失败/);
console.log("TE task type slice self-check passed");
