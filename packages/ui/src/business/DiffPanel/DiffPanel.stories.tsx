import type { Meta, StoryObj } from "@storybook/react";
import { DiffPanel } from "./DiffPanel";

const meta: Meta<typeof DiffPanel> = {
  title: "Components/DiffPanel",
  component: DiffPanel,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "字段 diff 对比面板：before/after 双列高亮变化字段。覆盖 H2-002 审计详情、M-VR 校验异常、M-PM 参数映射变更。",
      },
    },
  },
  argTypes: {
    layout: { control: "radio", options: ["side-by-side", "stacked"] },
    highlightChanged: { control: "boolean" },
  },
};
export default meta;
type Story = StoryObj<typeof DiffPanel>;

export const H2AuditUpdate: Story = {
  name: "H2-002 审计事件 diff（更新供应商）",
  args: {
    layout: "side-by-side",
    highlightChanged: true,
    before: {
      "供应商名称": "国药控股北京有限公司",
      "GSP 证号": "GSP-BJ-2025-0301",
      "经营范围": "中药材, 化学药制剂",
      "质量评分": "92",
    },
    after: {
      "供应商名称": "国药控股北京有限公司",
      "GSP 证号": "GSP-BJ-2026-0001",
      "经营范围": "中药材, 化学药制剂, 生物制品",
      "质量评分": "95",
    },
  },
};

export const Stacked: Story = {
  name: "堆叠布局（窄屏）",
  args: {
    layout: "stacked",
    before: { 状态: "待验收", 备注: "" },
    after: { 状态: "已上架", 备注: "外观完好" },
  },
};

export const NoHighlight: Story = {
  name: "关闭变化高亮",
  args: {
    highlightChanged: false,
    before: { 字段A: "1", 字段B: "2" },
    after: { 字段A: "1", 字段B: "3" },
  },
};
