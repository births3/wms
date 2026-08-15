import { describe, expect, it } from "vitest";
import {
  createDateColumn,
  createMonoColumn,
  createNumericColumn,
  createStatusColumn,
} from "../src/business/DataGrid/data-grid-column-factory";

interface MockRow {
  id: string;
  orderNo: string;
  status: "pending" | "done";
  qty: number;
  createdAt: string;
}

describe("data-grid-column-factory", () => {
  it("creates status column with mapping", () => {
    const col = createStatusColumn<MockRow>({
      key: "status",
      header: "单据状态",
      statusMap: {
        pending: { label: "处理中", status: "pending" },
        done: { label: "已完成", status: "completed" },
      },
    });

    expect(col.key).toBe("status");
    expect(col.header).toBe("单据状态");
    expect(col.sortable).toBe(true);
    expect(col.copyValue?.({ id: "1", orderNo: "O1", status: "pending", qty: 10, createdAt: "2026-08-15" })).toBe("处理中");
  });

  it("creates mono column with copyable enabled", () => {
    const col = createMonoColumn<MockRow>({
      key: "orderNo",
      header: "出库单号",
    });

    expect(col.mono).toBe(true);
    expect(col.copyable).toBe(true);
    expect(col.copyValue?.({ id: "1", orderNo: "OUT-001", status: "done", qty: 5, createdAt: "2026-08-15" })).toBe("OUT-001");
  });

  it("creates date column with custom formatter", () => {
    const col = createDateColumn<MockRow>({
      key: "createdAt",
      header: "创建时间",
      formatter: (val) => `[${val}]`,
    });

    expect(col.header).toBe("创建时间");
    expect(col.copyValue?.({ id: "1", orderNo: "OUT-001", status: "done", qty: 5, createdAt: "2026-08-15" })).toBe("[2026-08-15]");
  });

  it("creates numeric column with alignment and formatting", () => {
    const col = createNumericColumn<MockRow>({
      key: "qty",
      header: "件数",
      unit: "件",
    });

    expect(col.align).toBe("right");
    expect(col.sortValue?.({ id: "1", orderNo: "OUT-001", status: "done", qty: 1200, createdAt: "2026-08-15" })).toBe(1200);
    expect(col.copyValue?.({ id: "1", orderNo: "OUT-001", status: "done", qty: 1200, createdAt: "2026-08-15" })).toBe("1200 件");
  });
});
