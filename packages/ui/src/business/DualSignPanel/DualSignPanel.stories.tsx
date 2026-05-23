import type { Meta, StoryObj } from "@storybook/react";
import { DualSignPanel, type DualSignPolicy } from "./DualSignPanel";

const meta: Meta<typeof DualSignPanel> = {
  title: "Components/DualSignPanel",
  component: DualSignPanel,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "双人签字面板 · 三档策略（single / dual_scan / dual_scan_with_approval）。覆盖 M2-004 入库双签 / VR-006 策略矩阵 / BA-002 批号调整双签。",
      },
    },
  },
  argTypes: {
    policy: {
      control: "select",
      options: ["single", "dual_scan", "dual_scan_with_approval"] satisfies DualSignPolicy[],
    },
  },
};
export default meta;
type Story = StoryObj<typeof DualSignPanel>;

export const Single: Story = {
  name: "single 策略（单人签字，常规收货）",
  args: {
    policy: "single",
    first: { user: "u001 张三", time: "09:14", comment: "外观完好，数量一致" },
  },
};

export const DualScanFirstOnly: Story = {
  name: "dual_scan 第一人已签 · 等第二人",
  args: {
    policy: "dual_scan",
    first: { user: "u001 张三", time: "09:14", comment: "外观完好" },
  },
};

export const DualScanComplete: Story = {
  name: "dual_scan 双人签字完成（M2-004）",
  args: {
    policy: "dual_scan",
    first: { user: "u001 张三", time: "09:14" },
    second: { user: "u002 李四", time: "09:16" },
  },
};

export const DualScanWithApproval: Story = {
  name: "dual_scan_with_approval 三段全签（特殊药品）",
  args: {
    policy: "dual_scan_with_approval",
    first: { user: "u001 张三", time: "09:14" },
    second: { user: "u002 李四", time: "09:16" },
    approval: { user: "u005 王主管", time: "09:32", comment: "已审批，可上架" },
  },
};

export const DualScanWithApprovalPending: Story = {
  name: "dual_scan_with_approval 等审批",
  args: {
    policy: "dual_scan_with_approval",
    first: { user: "u001 张三", time: "09:14" },
    second: { user: "u002 李四", time: "09:16" },
  },
};
