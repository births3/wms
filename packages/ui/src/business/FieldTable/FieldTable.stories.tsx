import type { Meta, StoryObj } from "@storybook/react";
import { FieldTable } from "./FieldTable";
import { StatusBadge } from "../StatusBadge";

const meta: Meta<typeof FieldTable> = {
  title: "Components/FieldTable",
  component: FieldTable,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "字段核对表：左标签右值，扫码自动填充时高亮（autoFilled=true）。覆盖 M2-003 验收 / M4-004 复核 / M3-005 盘点。",
      },
    },
  },
};
export default meta;
type Story = StoryObj<typeof FieldTable>;

export const M2Receive: Story = {
  name: "M2-003 PDA 验收（14 项核对）",
  args: {
    size: "lg",
    rows: [
      { label: "ASN 号", value: "PO-2026-0001" },
      { label: "品名", value: "葡萄糖注射液", autoFilled: true },
      { label: "规格", value: "500ml × 24 瓶", autoFilled: true },
      { label: "批号", value: "20260301A", autoFilled: true, required: true },
      { label: "有效期", value: "2028-03-01", autoFilled: true, required: true },
      { label: "生产日期", value: "2026-03-01", autoFilled: true },
      { label: "实际到货数量", value: "240 瓶", required: true },
      { label: "批准文号", value: "国药准字 H20020023" },
      { label: "外观", value: <StatusBadge status="qualified" size="sm" /> },
      { label: "包装", value: <StatusBadge status="qualified" size="sm" /> },
      { label: "说明书", value: <StatusBadge status="qualified" size="sm" /> },
      { label: "标签", value: <StatusBadge status="unqualified" size="sm" />, error: "标签污损" },
      { label: "验收结论", value: <StatusBadge status="pending" size="sm" /> },
      { label: "验收员", value: "张三（user_id=u001）" },
    ],
  },
};

export const Compact: Story = {
  args: {
    size: "sm",
    rows: [
      { label: "商品编码", value: "SKU-001" },
      { label: "库位", value: "A-01-02-03" },
      { label: "批号", value: "20260301A" },
    ],
  },
};

export const WithErrors: Story = {
  args: {
    size: "md",
    rows: [
      { label: "数量", value: "200", error: "实际到货 240，与 ASN 不符", required: true },
      { label: "外观", value: "破损 3 瓶", error: "需要拍照取证" },
    ],
  },
};
