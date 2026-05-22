import type { Meta, StoryObj } from "@storybook/react";
import { StepFlow } from "./StepFlow";

const meta: Meta<typeof StepFlow> = {
  title: "Components/StepFlow",
  component: StepFlow,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "多步骤流程指示器：当前步高亮、已完成打勾、失败步红叉。覆盖 M2-003 验收(14步)、M2-004 双人签字、M4-003 拣选、M2-005 上架。",
      },
    },
  },
};
export default meta;
type Story = StoryObj<typeof StepFlow>;

export const M2DualSign: Story = {
  name: "M2-004 双人签字（dual_scan）",
  args: {
    current: 1,
    steps: [
      { label: "第一人签字", description: "张三" },
      { label: "等待第二人", description: "签字中" },
      { label: "结论生效" },
    ],
  },
};

export const M2DualSignWithApproval: Story = {
  name: "M2-004 双人签字（dual_scan_with_approval）",
  args: {
    current: 2,
    steps: [
      { label: "第一人签字", description: "张三 ✓" },
      { label: "第二人签字", description: "李四 ✓" },
      { label: "主管审批", description: "企微推送中" },
      { label: "上架中" },
    ],
  },
};

export const M2VerifyVertical: Story = {
  name: "M2-003 PDA 验收（垂直布局）",
  args: {
    orientation: "vertical",
    size: "lg",
    current: 4,
    errorSteps: [3],
    steps: [
      { label: "扫描追溯码" },
      { label: "核对品名规格" },
      { label: "录入数量" },
      { label: "外观检查", description: "标签污损" },
      { label: "判定质量状态" },
      { label: "提交验收结论" },
    ],
  },
};

export const PickingFlow: Story = {
  name: "M4-003 PDA 拣选（PDA 大尺寸）",
  args: {
    size: "lg",
    current: 2,
    steps: [
      { label: "接单" },
      { label: "扫库位" },
      { label: "扫商品" },
      { label: "确认数量" },
      { label: "提交" },
    ],
  },
};
