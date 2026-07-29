import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const source = readFileSync(
  new URL("../src/business/QueryPanel/QueryPanel.tsx", import.meta.url),
  "utf8",
);

assert.match(source, /export interface QueryPanelQuickFilter/);
assert.match(source, /quickFilters\?: QueryPanelQuickFilter\[\]/);
assert.match(source, /aria-label=\{quickFiltersAriaLabel\}/);
assert.match(source, /aria-pressed=\{filter\.active === true\}/);
assert.match(source, /rounded-full/);
assert.match(source, /onQuickFilterClick\?\.\(filter\.key\)/);
