import type { Meta, StoryObj } from "@storybook/react";
import { OfflineIndicator } from "./OfflineIndicator";

const meta: Meta<typeof OfflineIndicator> = {
  title: "Components/OfflineIndicator",
  component: OfflineIndicator,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "PDA 端顶部 banner，跟随 H1 §7 离线策略。online 状态默认隐藏（除非有待同步），避免占用屏幕。",
      },
    },
  },
};
export default meta;
type Story = StoryObj<typeof OfflineIndicator>;

export const Online: Story = {
  args: { state: "online" },
  parameters: { docs: { description: { story: "在线时返回 null（不渲染）" } } },
};

export const OnlineWithPending: Story = {
  args: { state: "online", pendingCount: 3 },
};

export const Offline: Story = {
  args: { state: "offline", pendingCount: 12 },
};

export const Syncing: Story = {
  args: { state: "syncing", pendingCount: 12, syncProgress: 45 },
};

export const SyncingComplete: Story = {
  args: { state: "syncing", pendingCount: 0, syncProgress: 100 },
};
