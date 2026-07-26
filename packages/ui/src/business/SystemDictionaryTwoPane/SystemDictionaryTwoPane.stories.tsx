import type { Meta, StoryObj } from "@storybook/react";
import { SystemDictionaryTwoPane } from "./SystemDictionaryTwoPane";

const meta: Meta<typeof SystemDictionaryTwoPane> = {
  title: "Components/SystemDictionaryTwoPane",
  component: SystemDictionaryTwoPane,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "系统字典两层展示 · 左侧字典分类，右侧选中分类的字典项、来源、启停状态与参数摘要。",
      },
    },
  },
};
export default meta;
type Story = StoryObj<typeof SystemDictionaryTwoPane>;

export const DocumentType: Story = {
  name: "单据类型",
  args: {
    groups: [
      {
        code: "document_type",
        name: "单据类型",
        items: [
          {
            code: "purchase_inbound",
            name: "采购入库",
            source: "global",
            enabled: true,
            sortOrder: 10,
            params: {
              direction: "inbound",
              workflow_template: "inbound_standard",
              batch_policy: "none",
            },
          },
          {
            code: "sales_return",
            name: "销售退货",
            source: "owner_override",
            enabled: true,
            sortOrder: 20,
            params: {
              direction: "inbound",
              workflow_template: "return_inbound",
              batch_policy: "required",
            },
          },
        ],
      },
    ],
  },
};

export const EmptyCategory: Story = {
  name: "空分类",
  args: {
    groups: [{ code: "document_type", name: "单据类型", items: [] }],
  },
};
