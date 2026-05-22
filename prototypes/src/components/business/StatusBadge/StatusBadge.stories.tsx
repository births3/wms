import type { Meta, StoryObj } from "@storybook/react";
import { StatusBadge, type StatusKey } from "./StatusBadge";

const meta: Meta<typeof StatusBadge> = {
  title: "Components/StatusBadge",
  component: StatusBadge,
  tags: ["autodocs"],
  argTypes: {
    status: {
      control: "select",
      options: [
        "qualified",
        "unqualified",
        "pending",
        "isolated",
        "expired",
        "near_expiry",
        "in_progress",
        "completed",
        "offline_cached",
      ] satisfies StatusKey[],
    },
    size: { control: "radio", options: ["sm", "md", "lg"] },
  },
};
export default meta;
type Story = StoryObj<typeof StatusBadge>;

export const Default: Story = {
  args: { status: "qualified", size: "md" },
};

export const AllStatuses: Story = {
  render: () => {
    const statuses: StatusKey[] = [
      "qualified",
      "unqualified",
      "pending",
      "isolated",
      "expired",
      "near_expiry",
      "in_progress",
      "completed",
      "offline_cached",
    ];
    return (
      <div style={{ display: "flex", flexWrap: "wrap", gap: 8, padding: 16 }}>
        {statuses.map((s) => (
          <StatusBadge key={s} status={s} />
        ))}
      </div>
    );
  },
};

export const PdaSize: Story = {
  name: "PDA 端尺寸（lg）",
  args: { status: "qualified", size: "lg" },
  parameters: { docs: { description: { story: "PDA 端使用 lg 尺寸，字号 18pt 满足 usability-baseline" } } },
};

export const CustomLabel: Story = {
  args: { status: "unqualified", label: "外观破损" },
};
