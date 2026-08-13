import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const scrollBarSource = readFileSync(new URL("../src/ui/scroll-bar.tsx", import.meta.url), "utf8");
const dataTableSource = readFileSync(new URL("../src/business/DataTable/DataTable.tsx", import.meta.url), "utf8");
const dataGridContentSource = readFileSync(new URL("../src/business/DataGrid/DataGridContent.tsx", import.meta.url), "utf8");
const dataGridTypesSource = readFileSync(new URL("../src/business/DataGrid/data-grid-types.ts", import.meta.url), "utf8");

// ScrollBar：可访问性语义（role + aria 三值 + 键盘）
assert.match(scrollBarSource, /role="scrollbar"/);
assert.match(scrollBarSource, /aria-orientation="horizontal"/);
assert.match(scrollBarSource, /aria-valuemin=\{0\}/);
assert.match(scrollBarSource, /aria-valuemax=\{view\.maxLeft\}/);
assert.match(scrollBarSource, /aria-valuenow=\{Math\.round\(Math\.min\(view\.left, view\.maxLeft\)\)\}/);
assert.match(scrollBarSource, /tabIndex=\{0\}/);
assert.match(scrollBarSource, /onKeyDown=/);

// ScrollBar：指针交互三件套 + 常驻渲染（hidden 切换，避免条件挂载鸡生蛋）
assert.match(scrollBarSource, /onPointerDown=/);
assert.match(scrollBarSource, /onPointerMove=/);
assert.match(scrollBarSource, /onPointerUp=/);
assert.match(scrollBarSource, /!view\.scrollable && "hidden"/);

// ScrollBar：双观察修复滑块刷新盲区（容器 clientWidth + 内容 scrollWidth）
assert.match(scrollBarSource, /observer\.observe\(container\)/);
assert.match(scrollBarSource, /observer\.observe\(contentRef\.current\)/);

// ScrollBar：计算委托纯函数模块
assert.match(scrollBarSource, /from "\.\/scroll-bar-math"/);

// DataTable：旧私有横向滚动条与 sticky 页脚技巧彻底移除
assert.doesNotMatch(dataTableSource, /DataTableHScrollBar/);
assert.doesNotMatch(dataTableSource, /-mt-12/);
assert.doesNotMatch(dataTableSource, /className="h-12"/);
assert.doesNotMatch(dataTableSource, /calc\(100vh/);

// DataTable：列表自管滚动结构 —— flex column 根 + 双轴滚动容器 + 常驻底部栏
assert.match(dataTableSource, /flex min-h-0 flex-col/);
assert.match(dataTableSource, /overflow-auto overscroll-contain/);
assert.match(dataTableSource, /\[&::-webkit-scrollbar:horizontal\]:hidden/);
assert.match(dataTableSource, /scrollbar-width:none/);
assert.match(dataTableSource, /<ScrollBar container=\{scrollAreaNode\}/);
assert.match(dataTableSource, /contentRef=\{tableRef\}/);

// DataTable：视口测量高度契约（maxHeight prop + hook 接入）
assert.match(dataTableSource, /maxHeight\?: string \| number/);
assert.match(dataTableSource, /useScrollAreaMaxHeight/);

// DataTable：内容少时滚动区按视口剩余撑满（minHeight 与 maxHeight 同源测量，防回退为纯上限）
assert.match(dataTableSource, /minHeight: effectiveMinHeight/);
assert.match(dataTableSource, /maxHeight: effectiveMaxHeight/);

// DataGridContent：外层滚动 div 与魔法数移除，DataTable 改为 flex 子项
assert.doesNotMatch(dataGridContentSource, /23rem/);
assert.doesNotMatch(dataGridContentSource, /overflow-visible/);
assert.match(dataGridContentSource, /className="flex-1"/);
assert.match(dataGridContentSource, /maxHeight=\{maxHeight\}/);

// data-grid-types：maxHeight 注释不再引用魔法数
assert.doesNotMatch(dataGridTypesSource, /15rem/);
