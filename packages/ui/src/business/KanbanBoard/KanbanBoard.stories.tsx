import type { Meta, StoryObj } from "@storybook/react";
import { KanbanBoard } from "./KanbanBoard";

const meta: Meta<typeof KanbanBoard> = {
  title: "Components/KanbanBoard",
  component: KanbanBoard,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "多列看板 · 卡片含优先级（low/normal/high/urgent）· 实时刷新 ≤ 3s。覆盖 M2-008 收货看板 / M4-007 出库看板 / DOCK-006 月台占用。",
      },
    },
  },
};
export default meta;
type Story = StoryObj<typeof KanbanBoard>;

export const M2InboundKanban: Story = {
  name: "M2-008 入库看板（待验收 / 验收中 / 已上架）",
  args: {
    columns: [
      {
        title: "待验收",
        variant: "warning",
        items: [
          {
            id: "k1",
            title: "PO-2026-0001",
            subtitle: "国药控股北京",
            priority: "high",
            status: "pending",
            meta: [
              { label: "件数", value: "240" },
              { label: "ETA", value: "10:30" },
            ],
          },
          {
            id: "k2",
            title: "PO-2026-0002",
            subtitle: "上海医药华东",
            priority: "normal",
            meta: [{ label: "件数", value: "180" }],
          },
        ],
      },
      {
        title: "验收中",
        variant: "default",
        items: [
          {
            id: "k3",
            title: "PO-2026-0003",
            subtitle: "九州通医药",
            priority: "urgent",
            status: "in_progress",
            meta: [
              { label: "进度", value: "5/12" },
              { label: "操作员", value: "u001" },
            ],
          },
        ],
      },
      {
        title: "已上架",
        variant: "success",
        items: [
          {
            id: "k4",
            title: "PO-2026-0000",
            subtitle: "甘李药业",
            status: "completed",
            meta: [{ label: "件数", value: "120" }],
          },
        ],
      },
    ],
  },
};

export const SingleColumn: Story = {
  name: "单列异常告警",
  args: {
    columns: [
      {
        title: "异常待处理",
        variant: "danger",
        items: [
          {
            id: "x1",
            title: "M5-超温报警",
            subtitle: "冷藏 A 区 · 9.2°C",
            priority: "urgent",
            status: "isolated",
            meta: [{ label: "持续", value: "12 分钟" }],
          },
        ],
      },
    ],
  },
};

export const Empty: Story = {
  name: "空看板",
  args: {
    columns: [
      { title: "待处理", items: [] },
      { title: "进行中", items: [] },
      { title: "完成", items: [] },
    ],
  },
};
