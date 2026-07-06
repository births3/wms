import type { Meta, StoryObj } from "@storybook/react";
import { TreeCatalog } from "./TreeCatalog";

const meta: Meta<typeof TreeCatalog> = {
  title: "Components/TreeCatalog",
  component: TreeCatalog,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "管理端树状导航组件，承载搜索、展开、选择和偏好保存。",
      },
    },
  },
};
export default meta;
type Story = StoryObj<typeof TreeCatalog>;

export const Basic: Story = {
  name: "模板树",
  args: {
    title: "模板树",
    searchPlaceholder: "搜索模板类型、字段库",
    storageKey: "storybook-tree-catalog",
    nodes: [
      {
        id: "type:asn",
        label: "ASN 单",
        description: "asn",
        badge: "M2",
        children: [
          {
            id: "library:m2_asn",
            label: "M2 ASN 字段库",
            description: "m2_asn",
            badge: "12 字段",
            children: [{ id: "version:v1", label: "v1", description: "2026-07-06", badge: "已发布" }],
          },
        ],
      },
    ],
  },
};
