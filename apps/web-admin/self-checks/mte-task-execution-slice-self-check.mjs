import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";

const root = new URL("..", import.meta.url);
const read = (file) => readFileSync(new URL(file, root), "utf8");
const dispatch = read("src/pages/task-engine/TaskDispatchPage.tsx");
const groups = read("src/pages/task-engine/TaskGroupConfigPage.tsx");
const queries = read("src/features/task-engine/task-engine-queries.ts");
const renderer = read("src/app-shell/AdminViewRenderer.tsx");

for (const page of [dispatch, groups]) {
  assert.match(page, /页面设计契约/);
  assert.match(page, /<QueryPanel/);
  assert.match(page, /<DataGrid/);
}
for (const token of ["mte.task-dispatch", "分派", "下发", "召回", "处置完成"]) assert.match(dispatch, new RegExp(token));
assert.match(dispatch, /确认任务操作/);
for (const token of ["mte.task-groups", "任务类型", "适用库区", "任务组成员", "zoneIds", "memberUserIds"]) assert.match(groups, new RegExp(token));
for (const path of ["task-engine/task-groups", "task-engine/tasks", "transitions", "Idempotency-Key"]) assert.match(queries, new RegExp(path));
assert.match(renderer, /mte-task-dispatch/);
assert.match(renderer, /mte-task-groups/);
console.log("M-TE task execution slice self-check passed");
