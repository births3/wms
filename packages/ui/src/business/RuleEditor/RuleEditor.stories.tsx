import type { Meta, StoryObj } from "@storybook/react";
import { RuleEditor } from "./RuleEditor";

const meta: Meta<typeof RuleEditor> = {
  title: "Components/RuleEditor",
  component: RuleEditor,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "校验规则配置 · 条件组（AND/OR）+ 动作叠加。覆盖 VR-001 校验规则 / M-PM-002 参数映射 / M9-001 计费规则。",
      },
    },
  },
  argTypes: {
    readOnly: { control: "boolean" },
  },
};
export default meta;
type Story = StoryObj<typeof RuleEditor>;

export const M9BillingRule: Story = {
  name: "M9-001 计费规则（冷藏托盘日费）",
  args: {
    readOnly: true,
    fields: ["warehouse_type", "temp_zone", "owner_id", "category"],
    groups: [
      {
        connector: "AND",
        conditions: [
          { field: "warehouse_type", op: "eq", value: "CR" },
          { field: "temp_zone", op: "in", value: "[2-8°C, -25°C]" },
        ],
      },
    ],
    actions: [
      {
        type: "charge_unit",
        label: "按托盘日计费",
        params: { unit: "托盘", price: "8.5", currency: "CNY/日" },
      },
      {
        type: "alert",
        label: "超温告警",
        params: { channel: "企微", severity: "P2" },
      },
    ],
  },
};

export const VR001ValidationRule: Story = {
  name: "VR-001 校验规则（冷链入库批号必填）",
  args: {
    readOnly: false,
    fields: ["category", "temp_zone", "batch_no", "expiry_date"],
    groups: [
      {
        connector: "AND",
        conditions: [
          { field: "category", op: "eq", value: "冷链药品" },
          { field: "batch_no", op: "neq", value: "" },
        ],
      },
      {
        connector: "OR",
        conditions: [
          { field: "expiry_date", op: "gt", value: "today + 90d" },
          { field: "expiry_date", op: "contains", value: "近效期豁免" },
        ],
      },
    ],
    actions: [
      { type: "block", label: "阻断入库", params: { error_code: "M2-VR-014" } },
    ],
  },
};

export const MPM002Mapping: Story = {
  name: "M-PM-002 参数映射规则",
  args: {
    readOnly: true,
    fields: ["source_field", "target_field", "owner_code"],
    groups: [
      {
        connector: "AND",
        conditions: [
          { field: "source_field", op: "eq", value: "ERP.ItemCode" },
          { field: "owner_code", op: "in", value: "[OWNER-001, OWNER-002]" },
        ],
      },
    ],
    actions: [
      {
        type: "map",
        label: "映射至",
        params: { target: "wms.sku_code", transform: "trim+upper" },
      },
    ],
  },
};
