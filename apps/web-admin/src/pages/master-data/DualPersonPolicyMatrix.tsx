import * as React from "react";
import { Button, Card, CardContent, CardHeader, CardTitle } from "@wms/ui";

import { useRoleUsersQuery } from "@/features/auth/role-permission-queries";
import { useMasterDataRowsQuery } from "@/features/master-data/master-data-queries";
import type { SystemDictionaryPaneItem } from "@/features/master-data/master-data-queries";
import {
  useDualPersonPolicyRulesQuery,
  useUpsertDualPersonPolicyRuleMutation,
  type DualPersonPolicy,
  type DualPersonPolicyRule,
} from "@/features/validation-rules/dual-person-policy-queries";

const processNodes = {
  入库: ["收货", "验收", "上架"],
  出库: ["拣货", "复核", "装箱", "发货交接"],
  报损: ["报损执行"],
  报溢: ["报溢执行"],
  销毁: ["销毁执行"],
  退货: ["退货验收", "退货上架"],
} as const;

type ProcessName = keyof typeof processNodes;
type Scope = "owner" | "warehouse";

const policyOptions: Array<{ value: DualPersonPolicy; label: string }> = [
  { value: "single", label: "单人" },
  { value: "dual_scan", label: "双人扫码" },
  { value: "dual_scan_with_approval", label: "双人扫码 + 主管审批" },
];

export function DualPersonPolicyMatrix({ categories }: { categories: SystemDictionaryPaneItem[] }) {
  const [process, setProcess] = React.useState<ProcessName>("入库");
  const [scope, setScope] = React.useState<Scope>("owner");
  const [warehouseId, setWarehouseId] = React.useState("");
  const [confirmerId, setConfirmerId] = React.useState("");
  const [notice, setNotice] = React.useState<string | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  const warehouses = useMasterDataRowsQuery("m1-warehouses");
  const users = useRoleUsersQuery();
  const rules = useDualPersonPolicyRulesQuery(scope === "warehouse" ? warehouseId : undefined);
  const save = useUpsertDualPersonPolicyRuleMutation();
  const matrixCategories = categories.filter((item) => item.enabled);

  async function updateCell(category: string, node: string, policy: DualPersonPolicy) {
    setNotice(null);
    setError(null);
    if (!confirmerId) {
      setError("请选择与当前操作人不同的矩阵确认人");
      return;
    }
    if (scope === "warehouse" && !warehouseId) {
      setError("仓库级策略必须选择仓库");
      return;
    }
    try {
      await save.mutateAsync({
        special_drug_category: category,
        process,
        node,
        policy,
        scope,
        warehouse_id: scope === "warehouse" ? warehouseId : null,
        priority: 100,
        enabled: true,
        confirmed_by_user_id: confirmerId,
      });
      setNotice(`${category} / ${process} / ${node} 已保存`);
    } catch (value) {
      setError(value instanceof Error ? value.message : "保存双人策略失败");
    }
  }

  return (
    <Card>
      <CardHeader className="gap-3">
        <CardTitle>双人作业矩阵</CardTitle>
        <p className="text-sm text-muted-foreground">
          按特殊药品分类 × 流程 × 节点维护策略；每次生效需另一名有权限的用户确认。
        </p>
        <div className="flex flex-wrap gap-2" aria-label="流程选择">
          {(Object.keys(processNodes) as ProcessName[]).map((value) => (
            <Button
              key={value}
              type="button"
              size="sm"
              variant={process === value ? "default" : "outline"}
              onClick={() => setProcess(value)}
            >
              {value}
            </Button>
          ))}
        </div>
        <div className="grid gap-3 md:grid-cols-3">
          <MatrixSelect label="规则范围" value={scope} onChange={(value) => setScope(value as Scope)}>
            <option value="owner">货主级</option>
            <option value="warehouse">仓库级</option>
          </MatrixSelect>
          <MatrixSelect
            label="仓库"
            value={warehouseId}
            disabled={scope !== "warehouse"}
            onChange={setWarehouseId}
          >
            <option value="">请选择仓库</option>
            {(warehouses.data ?? []).map((warehouse) => (
              <option key={warehouse.id} value={warehouse.id}>
                {warehouse.code} · {warehouse.name}
              </option>
            ))}
          </MatrixSelect>
          <MatrixSelect label="矩阵确认人" value={confirmerId} onChange={setConfirmerId}>
            <option value="">请选择另一名确认人</option>
            {(users.data ?? []).map((user) => (
              <option key={user.user_id} value={user.user_id}>
                {user.display_name} · {user.username}
              </option>
            ))}
          </MatrixSelect>
        </div>
        {notice && <p className="text-sm text-wms-success" role="status">{notice}</p>}
        {(error || rules.error) && (
          <p className="text-sm text-destructive" role="alert">
            {error ?? rules.error?.message}
          </p>
        )}
      </CardHeader>
      <CardContent className="overflow-x-auto">
        <table className="w-full min-w-[720px] border-collapse text-sm">
          <caption className="sr-only">{process}流程双人策略矩阵</caption>
          <thead>
            <tr className="border-b text-left">
              <th className="px-3 py-2">特殊药品分类</th>
              {processNodes[process].map((node) => <th key={node} className="px-3 py-2">{node}</th>)}
            </tr>
          </thead>
          <tbody>
            {matrixCategories.map((category) => (
              <tr key={category.code} className="border-b last:border-0">
                <th className="px-3 py-2 text-left font-medium">{category.name}</th>
                {processNodes[process].map((node) => (
                  <td key={node} className="px-3 py-2">
                    <select
                      className="min-h-10 w-full rounded-md border border-input bg-background px-2"
                      aria-label={`${category.name} ${process} ${node} 双人策略`}
                      value={configuredPolicy(rules.data?.data ?? [], category.code, process, node, scope, warehouseId)}
                      disabled={save.isPending || rules.isPending || (scope === "warehouse" && !warehouseId)}
                      onChange={(event) => void updateCell(category.code, node, event.target.value as DualPersonPolicy)}
                    >
                      {policyOptions.map((option) => (
                        <option key={option.value} value={option.value}>{option.label}</option>
                      ))}
                    </select>
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </CardContent>
    </Card>
  );
}

function MatrixSelect({ label, value, disabled, onChange, children }: {
  label: string;
  value: string;
  disabled?: boolean;
  onChange: (value: string) => void;
  children: React.ReactNode;
}) {
  return (
    <label className="grid gap-1 text-sm">
      <span>{label}</span>
      <select
        className="min-h-10 rounded-md border border-input bg-background px-3"
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value)}
      >
        {children}
      </select>
    </label>
  );
}

function configuredPolicy(
  rules: DualPersonPolicyRule[],
  category: string,
  process: string,
  node: string,
  scope: Scope,
  warehouseId: string,
): DualPersonPolicy {
  const matches = rules.filter((rule) =>
    rule.enabled &&
    rule.special_drug_category === category &&
    rule.process === process &&
    rule.node === node
  );
  const configured = matches.find((rule) => scope === "owner"
    ? Boolean(rule.owner_id) && !rule.warehouse_id
    : rule.warehouse_id === warehouseId);
  return configured?.policy ?? matches[0]?.policy ?? "single";
}
