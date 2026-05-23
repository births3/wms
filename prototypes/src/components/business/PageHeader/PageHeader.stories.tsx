import type { Meta, StoryObj } from "@storybook/react";
import { Button } from "@/components/ui/button";
import { PageHeader } from "./PageHeader";

const meta: Meta<typeof PageHeader> = {
  title: "Components/PageHeader",
  component: PageHeader,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "管理页通用骨架 · 标题 + 副标题 + 操作 + 面包屑。覆盖 M1/M3/M6 全部 PC 列表页。",
      },
    },
  },
};
export default meta;
type Story = StoryObj<typeof PageHeader>;

export const M1Items: Story = {
  name: "M1-001 商品档案（标准布局）",
  args: {
    title: "商品档案",
    subtitle: "M1-001 · ADR-0021 · 共 1,243 条记录 · GSP 合规",
    breadcrumb: (
      <span className="text-xs text-muted-foreground">M1 基础数据 / 商品档案</span>
    ),
    actions: (
      <div className="flex gap-2">
        <Button variant="outline" size="sm">导出</Button>
        <Button size="sm">+ 新增</Button>
      </div>
    ),
  },
};

export const TitleOnly: Story = {
  name: "仅标题（最简）",
  args: { title: "仪表盘" },
};

export const WithLongSubtitle: Story = {
  name: "长副标题（多元数据）",
  args: {
    title: "供应商资质档案",
    subtitle:
      "M1-002 · GSP 证（如 GSP-BJ-2026-0001）+ 营业执照 + 经营范围 + 质量评分 · 维护 u001",
    actions: (
      <Button variant="outline" size="sm">资质审核</Button>
    ),
  },
};
