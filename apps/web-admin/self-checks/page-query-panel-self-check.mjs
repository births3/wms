import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const queryPanelSource = readFileSync(
  new URL("../../../packages/ui/src/business/QueryPanel/QueryPanel.tsx", import.meta.url),
  "utf8",
);
const m1PageSource = readFileSync(
  new URL("../src/pages/master-data/M1MasterDataPage.tsx", import.meta.url),
  "utf8",
);
const m2PageSource = readFileSync(
  new URL("../src/pages/inbound/M2InboundPage.tsx", import.meta.url),
  "utf8",
);
const m3PageSource = readFileSync(
  new URL("../src/pages/inventory/M3BatchManagementPage.tsx", import.meta.url),
  "utf8",
);
const m4PageSource = readFileSync(
  new URL("../src/pages/outbound/M4OutboundPage.tsx", import.meta.url),
  "utf8",
);
const m2TableSource = readFileSync(
  new URL("../src/pages/inbound/M2InboundOrderTable.tsx", import.meta.url),
  "utf8",
);

assert.match(queryPanelSource, /export type QueryPanelFieldType = "text" \| "select" \| "multiSelect" \| "dateRange" \| "numberRange";/);
assert.match(queryPanelSource, /fields\?: QueryPanelField\[\];/);
assert.match(queryPanelSource, /defaultVisibleFieldKeys\?: string\[\];/);
assert.match(queryPanelSource, /展开/);
assert.match(queryPanelSource, /收起/);
assert.match(queryPanelSource, /buildQueryPanelSummaryItems/);

assert.match(m1PageSource, /const m1QueryFields: QueryPanelField\[\]/);
assert.match(m1PageSource, /usePageQueryState[<(]/);
assert.match(m1PageSource, /\bdraftQuery\b/);
assert.match(m1PageSource, /\bappliedQuery\b/);
assert.match(m1PageSource, /(?:fields|queryFields)=\{m1QueryFields\}/);
assert.match(m1PageSource, /(?:defaultVisibleFieldKeys|coreQueryFieldKeys)=\{m1CoreQueryFieldKeys\}/);
assert.match(m1PageSource, /queryState(?:=\{appliedQuery\}|:\s*appliedQuery)/);
assert.doesNotMatch(m1PageSource, /keyword=\{keyword\}/);

assert.match(m2PageSource, /const m2InboundQueryFields: QueryPanelField\[\]/);
assert.match(m2PageSource, /usePageQueryState[<(]/);
assert.match(m2PageSource, /\bdraftQuery\b/);
assert.match(m2PageSource, /\bappliedQuery\b/);
assert.match(m2PageSource, /(?:fields|queryFields)=\{m2InboundQueryFields\}/);
assert.match(m2PageSource, /(?:defaultVisibleFieldKeys|coreQueryFieldKeys)=\{m2InboundCoreQueryFieldKeys\}/);
assert.match(m2PageSource, /queryState(?:=\{appliedQuery\}|:\s*appliedQuery)/);
assert.doesNotMatch(m2PageSource, /<M2InboundFilterBar/);

assert.match(m2TableSource, /queryState\?: M2InboundQueryValue;/);
assert.match(m2TableSource, /querySummaryItems\?: DataGridQuerySummaryItem\[\];/);
assert.match(m2TableSource, /queryState=\{queryState\}/);

assert.match(m3PageSource, /const m3BatchQueryFields: QueryPanelField\[\]/);
assert.match(m3PageSource, /(?:fields|queryFields)=\{m3BatchQueryFields\}/);
assert.match(m3PageSource, /(?:defaultVisibleFieldKeys|coreQueryFieldKeys)=\{m3BatchCoreQueryFieldKeys\}/);
assert.match(m3PageSource, /queryState(?:=\{appliedQuery\}|:\s*appliedQuery)/);

assert.match(m4PageSource, /const m4OutboundQueryFields: QueryPanelField\[\]/);
assert.match(m4PageSource, /(?:fields|queryFields)=\{m4OutboundQueryFields\}/);
assert.match(m4PageSource, /(?:defaultVisibleFieldKeys|coreQueryFieldKeys)=\{m4OutboundCoreQueryFieldKeys\}/);
assert.match(m4PageSource, /queryState(?:=\{appliedQuery\}|:\s*appliedQuery)/);
assert.doesNotMatch(m4PageSource, /function FilterBar/);
