import type { Meta, StoryObj } from "@storybook/react";
import { ApprovalFlow } from "./ApprovalFlow";

const meta: Meta<typeof ApprovalFlow> = {
  title: "Components/ApprovalFlow",
  component: ApprovalFlow,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "审批流时间线 · 5 状态（pending / approved / rejected / current / skipped）。覆盖 QL-003 质量联系单 / BA-002 批号调整 / DOCK-004 月台预约。",
      },
    },
  },
};
export default meta;
type Story = StoryObj<typeof ApprovalFlow>;

export const QL003InProgress: Story = {
  name: "QL-003 质量联系单（库管已批，主管待审）",
  args: {
    nodes: [
      {
        role: "库管",
        approver: "u001 张三",
        time: "2026-05-23 09:14",
        status: "approved",
        comment: "外观完好，数量一致",
      },
      {
        role: "主管",
        approver: "u005 王主管",
        time: "2026-05-23 09:32",
        status: "current",
      },
      { role: "质量负责人", status: "pending" },
    ],
  },
};

export const Rejected: Story = {
  name: "驳回（rejected）— 主管打回",
  args: {
    nodes: [
      {
        role: "库管",
        approver: "u001 张三",
        time: "2026-05-23 09:14",
        status: "approved",
        comment: "请求批号调整",
      },
      {
        role: "主管",
        approver: "u005 王主管",
        time: "2026-05-23 09:32",
        status: "rejected",
        comment: "证据不足，请补充供应商质保单",
      },
    ],
  },
};

export const FullyApproved: Story = {
  name: "完全通过（BA-002 批号调整闭环）",
  args: {
    nodes: [
      { role: "申请人", approver: "u001 张三", time: "09:14", status: "approved" },
      { role: "主管", approver: "u005 王主管", time: "09:30", status: "approved" },
      { role: "质量负责人", approver: "u008 陈质量", time: "10:05", status: "approved", comment: "通过" },
    ],
  },
};

export const Skipped: Story = {
  name: "跳过（系统触发免二次审批，K14）",
  args: {
    nodes: [
      { role: "申请人", approver: "u001 张三", time: "09:14", status: "approved" },
      { role: "主管", status: "skipped", comment: "系统触发隔离 · 免审批" },
      { role: "质量负责人", status: "current" },
    ],
  },
};
