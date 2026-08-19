import type { Meta, StoryObj } from "@storybook/react";
import { TwoPaneCatalog } from "./TwoPaneCatalog";

const meta: Meta<typeof TwoPaneCatalog> = {
  title: "Components/TwoPaneCatalog",
  component: TwoPaneCatalog,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "分类-明细两栏通用骨架，承载筛选、选择、字段显隐、复制和偏好保存。",
      },
    },
  },
};
export default meta;
type Story = StoryObj<typeof TwoPaneCatalog>;

export const Basic: Story = {
  name: "基础分类明细",
  args: {
    title: "字典项",
    groupTitle: "字典分类",
    itemTitle: "字典项",
    selectable: true,
    storageKey: "storybook-two-pane-catalog",
    fields: [
      { key: "code", label: "编码", copyText: (item) => item.code },
      { key: "source", label: "来源" },
    ],
    groups: [
      {
        code: "dict",
        name: "系统字典",
        items: [
          { code: "document_type", name: "单据类型", source: "global", enabled: true },
          { code: "obsolete", name: "停用类型", source: "owner", enabled: false },
        ],
      },
    ],
  },
};
