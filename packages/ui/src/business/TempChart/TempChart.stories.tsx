import type { Meta, StoryObj } from "@storybook/react";
import { TempChart } from "./TempChart";

const meta: Meta<typeof TempChart> = {
  title: "Components/TempChart",
  component: TempChart,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "温度曲线 · 上下阈值带 · 超阈高亮。覆盖 M5-002 冷链监控 / M6-002d 冷链月报 / M10-002 在途温控。",
      },
    },
  },
  argTypes: {
    minThreshold: { control: { type: "number" } },
    maxThreshold: { control: { type: "number" } },
    height: { control: { type: "range", min: 80, max: 400, step: 10 } },
    unit: { control: "text" },
  },
};
export default meta;
type Story = StoryObj<typeof TempChart>;

const COLD_24H = [
  { t: "00:00", v: 4.2 },
  { t: "02:00", v: 4.5 },
  { t: "04:00", v: 5.0 },
  { t: "06:00", v: 5.2 },
  { t: "08:00", v: 5.6 },
  { t: "10:00", v: 6.1 },
  { t: "12:00", v: 7.4 },
  { t: "14:00", v: 8.7 }, // 超阈
  { t: "16:00", v: 7.2 },
  { t: "18:00", v: 6.0 },
  { t: "20:00", v: 5.4 },
  { t: "22:00", v: 4.8 },
];

export const M5ColdMonitor: Story = {
  name: "M5-002 冷藏 24h（含一次超温）",
  args: {
    minThreshold: 2,
    maxThreshold: 8,
    unit: "°C",
    height: 180,
    points: COLD_24H,
  },
};

export const FrozenZone: Story = {
  name: "冷冻 -25°C 区（M6-002d 月报）",
  args: {
    minThreshold: -28,
    maxThreshold: -18,
    unit: "°C",
    height: 200,
    points: [
      { t: "周一", v: -24 },
      { t: "周二", v: -23 },
      { t: "周三", v: -25 },
      { t: "周四", v: -22 },
      { t: "周五", v: -16 }, // 超阈
      { t: "周六", v: -24 },
      { t: "周日", v: -25 },
    ],
  },
};

export const M10InTransit: Story = {
  name: "M10-002 在途温控（GPS 同步采集）",
  args: {
    minThreshold: 2,
    maxThreshold: 8,
    unit: "°C",
    height: 160,
    points: [
      { t: "08:00 出库", v: 4.5 },
      { t: "10:00 高速", v: 5.0 },
      { t: "12:00 服务区", v: 6.2 },
      { t: "14:00 路途", v: 5.8 },
      { t: "16:00 抵达", v: 5.2 },
    ],
  },
};

export const Empty: Story = {
  name: "空数据",
  args: { minThreshold: 2, maxThreshold: 8, points: [], height: 160 },
};
