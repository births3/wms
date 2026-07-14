import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";

const root = new URL("..", import.meta.url);
const read = (file) => readFileSync(new URL(file, root), "utf8");
const dispatch = read("src/pages/task-engine/TaskDispatchPage.tsx");
const groups = read("src/pages/task-engine/TaskGroupConfigPage.tsx");
const taskTypes = read("src/pages/task-engine/TaskTypeConfigPage.tsx");
const queries = read("src/features/task-engine/task-engine-queries.ts");
const renderer = read("src/app-shell/AdminViewRenderer.tsx");
const taskGroupView = "mte-task-groups";
const taskDispatchView = "mte-task-dispatch";
const taskTypeView = "mte-task-types";

for (const page of [dispatch, groups]) {
  assert.match(page, /页面设计契约/);
  assert.match(page, /<QueryPanel/);
  assert.match(page, /<DataGrid/);
}
for (const token of ["mte.task-dispatch", "释放", "分派", "下发", "召回", "处置完成", "手动加急", "priority_factors", "release_due_at"]) assert.match(dispatch, new RegExp(token));
assert.match(dispatch, /确认任务操作/);
for (const token of ["任务优先级规则", "订单加急加分", "等待多少分钟加 1 分", "冷链任务加分", "手动加急加分", "释放策略", "释放间隔", "每批任务数", "conditional", "capacity"]) assert.match(taskTypes, new RegExp(token));
for (const token of [
  "mte.task-groups", "任务类型", "适用库区", "任务组成员", "资格有效期",
  "同时在手上限", "zoneIds", "memberUserIds", "memberQualifications",
]) assert.match(groups, new RegExp(token));
for (const path of ["task-engine/task-groups", "task-engine/tasks", "transitions", "Idempotency-Key"]) assert.match(queries, new RegExp(path));
for (const path of ["task-engine/priority-rule", "Idempotency-Key"]) assert.match(read("src/features/task-engine/task-type-queries.ts"), new RegExp(path));
assert.match(renderer, new RegExp(taskDispatchView));
assert.match(renderer, new RegExp(taskGroupView));
assert.match(renderer, new RegExp(taskTypeView));
console.log("M-TE task execution slice self-check passed");
