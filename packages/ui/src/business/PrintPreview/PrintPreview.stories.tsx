import type { Meta, StoryObj } from "@storybook/react";
import { useState } from "react";
import { PrintPreview, type PrintTemplate } from "./PrintPreview";

const meta: Meta<typeof PrintPreview> = {
  title: "Components/PrintPreview",
  component: PrintPreview,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "打印预览容器 · A4 / 标签 / 面单 三种模板 · 支持缩放 + 翻页。覆盖 M2-007 收货单 / M4-005 随货同行单 / H5-003 快递面单。",
      },
    },
  },
  argTypes: {
    template: { control: "radio", options: ["a4", "label", "shipping"] satisfies PrintTemplate[] },
    zoom: { control: { type: "range", min: 0.3, max: 1.5, step: 0.1 } },
  },
};
export default meta;
type Story = StoryObj<typeof PrintPreview>;

export const A4Manifest: Story = {
  name: "A4 · M4-005 随货同行单",
  render: (args) => {
    const [zoom, setZoom] = useState(args.zoom ?? 0.6);
    return (
      <PrintPreview
        {...args}
        template="a4"
        pageCount={1}
        currentPage={1}
        zoom={zoom}
        onZoomChange={setZoom}
      >
        <div className="font-sans text-[10px] leading-snug">
          <h3 className="text-base font-semibold text-center mb-1">随货同行单</h3>
          <p className="text-center text-muted-foreground mb-3 text-[9px]">
            SO-2026-0042 · 国药控股北京 → 北京同仁堂
          </p>
          <table className="w-full border-collapse">
            <thead>
              <tr className="border-b border-foreground/30">
                <th className="text-left p-1 font-medium">商品编码</th>
                <th className="text-left p-1 font-medium">品名</th>
                <th className="text-left p-1 font-medium">批号</th>
                <th className="text-right p-1 font-medium">数量</th>
              </tr>
            </thead>
            <tbody>
              <tr className="border-b border-foreground/10">
                <td className="p-1">P-001234</td>
                <td className="p-1">葡萄糖注射液</td>
                <td className="p-1">20260301A</td>
                <td className="p-1 text-right">120 瓶</td>
              </tr>
              <tr className="border-b border-foreground/10">
                <td className="p-1">P-001235</td>
                <td className="p-1">氯化钠注射液</td>
                <td className="p-1">20260315B</td>
                <td className="p-1 text-right">80 瓶</td>
              </tr>
            </tbody>
          </table>
          <div className="mt-4 grid grid-cols-2 gap-2 text-[9px]">
            <div>发货人签字：________</div>
            <div>收货人签字：________</div>
          </div>
        </div>
      </PrintPreview>
    );
  },
};

export const LabelTemplate: Story = {
  name: "标签 · M2-007 库位标签（缩略）",
  args: {
    template: "label",
    zoom: 1,
    children: (
      <div className="font-sans text-[10px] text-center">
        <div className="text-2xl font-bold">A-01-02-03</div>
        <div className="text-xs text-muted-foreground mt-1">冷藏 2-8°C</div>
        <div className="mt-2 text-[8px]">SKU-001 · 葡萄糖注射液</div>
      </div>
    ),
  },
};

export const ShippingMultiPage: Story = {
  name: "面单 · 多页（H5-003 快递面单）",
  args: {
    template: "shipping",
    pageCount: 3,
    currentPage: 2,
    zoom: 0.7,
    children: (
      <div className="font-sans text-[9px]">
        <div className="text-[14px] font-bold mb-1">顺丰速运</div>
        <div>SF-2026-0042-002 / 3</div>
        <div className="mt-2 text-[8px]">收件人：北京同仁堂 · 王经理 · 13800000000</div>
      </div>
    ),
  },
};
