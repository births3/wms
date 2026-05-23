import type { Meta, StoryObj } from "@storybook/react";
import { DataTable } from "./DataTable";
import { StatusBadge, type StatusKey } from "../StatusBadge";

type Row = {
  code: string;
  name: string;
  category: string;
  status: StatusKey;
};

const ROWS: Row[] = [
  { code: "P-001234", name: "葡萄糖注射液", category: "普药", status: "qualified" },
  { code: "P-001235", name: "氯化钠注射液", category: "普药", status: "qualified" },
  { code: "P-002001", name: "盐酸吗啡注射液", category: "麻精", status: "isolated" },
  { code: "P-003045", name: "辉瑞牌阿托伐他汀", category: "近效期", status: "near_expiry" },
];

const meta: Meta<typeof DataTable<Row>> = {
  title: "Components/DataTable",
  component: DataTable<Row>,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "管理页通用表格 · 列定义 + 行选中 + caption + footer + 空态。覆盖 M1/M3/M6 全部 PC 列表场景。",
      },
    },
  },
};
export default meta;
type Story = StoryObj<typeof DataTable<Row>>;

export const M1ItemsList: Story = {
  name: "M1-001 商品列表",
  args: {
    rowKey: (r) => r.code,
    caption: "共 4 条 · 按商品编码升序",
    columns: [
      { key: "code", header: "商品编码", mono: true, width: 140 },
      { key: "name", header: "品名" },
      { key: "category", header: "类别", width: 100 },
      {
        key: "status",
        header: "状态",
        width: 110,
        render: (r) => <StatusBadge status={r.status} size="sm" />,
      },
    ],
    data: ROWS,
  },
};

export const SelectedRow: Story = {
  name: "选中态（点击 P-002001）",
  args: {
    rowKey: (r) => r.code,
    selectedKey: "P-002001",
    columns: [
      { key: "code", header: "编码", mono: true, width: 140 },
      { key: "name", header: "品名" },
      {
        key: "status",
        header: "状态",
        width: 110,
        render: (r) => <StatusBadge status={r.status} size="sm" />,
      },
    ],
    data: ROWS,
  },
};

export const Empty: Story = {
  name: "空态（自定义提示）",
  args: {
    rowKey: (r) => r.code,
    columns: [
      { key: "code", header: "编码", width: 140 },
      { key: "name", header: "品名" },
    ],
    data: [],
    emptyTitle: "暂无商品",
    emptyDescription: "尝试调整筛选条件或新增第一个商品档案",
  },
};

export const WithFooter: Story = {
  name: "含翻页 footer",
  args: {
    rowKey: (r) => r.code,
    columns: [
      { key: "code", header: "编码", mono: true, width: 140 },
      { key: "name", header: "品名" },
    ],
    data: ROWS,
    footer: (
      <div className="flex items-center justify-between text-xs text-muted-foreground">
        <span>1-4 / 共 1,243 条</span>
        <span>第 1 / 311 页</span>
      </div>
    ),
  },
};
