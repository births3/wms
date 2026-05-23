import type { Meta, StoryObj } from "@storybook/react";
import { Lock, WifiOff } from "lucide-react";
import { Button } from "@/components/ui/button";
import { EmptyState } from "./EmptyState";

const meta: Meta<typeof EmptyState> = {
  title: "Components/EmptyState",
  component: EmptyState,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "空状态 · 图标 + 标题 + 描述 + CTA · 通用兜底。覆盖所有列表 / 看板 / 详情场景（无数据 / 无权限 / 网络错误）。",
      },
    },
  },
};
export default meta;
type Story = StoryObj<typeof EmptyState>;

export const NoData: Story = {
  name: "无数据（默认）",
  args: {
    title: "暂无数据",
    description: "尝试调整筛选条件或新增第一条记录",
  },
};

export const NoPermission: Story = {
  name: "无权限（GSP 报表权限）",
  args: {
    icon: <Lock className="size-10" aria-hidden />,
    title: "无访问权限",
    description: "该模块需 GSP 报表查阅权限，请联系管理员",
  },
};

export const NetworkError: Story = {
  name: "网络异常 + 重试 CTA",
  args: {
    icon: <WifiOff className="size-10" aria-hidden />,
    title: "网络异常",
    description: "无法连接到服务器，请检查后重试",
    action: <Button size="sm">重试</Button>,
  },
};

export const TitleOnly: Story = {
  name: "仅标题（最简）",
  args: { title: "暂无数据" },
};
