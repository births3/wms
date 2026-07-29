import { Input } from "@wms/ui";

const selectOptions = {
  business_module: [
    ["M1", "M1 基础档案"],
    ["M2", "M2 入库"],
    ["M3", "M3 库内"],
    ["M4", "M4 出库"],
    ["H5", "H5 快递"],
  ],
  business_direction: [
    ["inbound", "入库"],
    ["outbound", "出库"],
    ["label", "标签"],
  ],
  paper_type: [
    ["a4", "A4"],
    ["a5", "A5"],
    ["label", "标签纸"],
  ],
  default_scope: [
    ["global", "全局默认"],
    ["owner", "货主覆盖"],
  ],
} as const;

export function PrintTemplateTypeFields({
  value,
  onChange,
}: {
  value: string;
  onChange: (value: string) => void;
}) {
  const params = JSON.parse(value) as Record<string, unknown>;
  const update = (key: string, nextValue: string) => {
    const next = { ...params };
    if (nextValue) next[key] = nextValue;
    else delete next[key];
    onChange(JSON.stringify(next, null, 2));
  };

  return (
    <div className="grid gap-3 md:col-span-2 md:grid-cols-2">
      <label className="grid gap-1 text-xs text-muted-foreground">
        字段库编码
        <Input
          value={textParam(params, "field_library_code")}
          onChange={(event) => update("field_library_code", event.target.value)}
        />
      </label>
      <SelectField label="业务模块" paramKey="business_module" params={params} onChange={update} />
      <SelectField
        label="业务方向"
        paramKey="business_direction"
        params={params}
        onChange={update}
      />
      <SelectField label="纸张类型" paramKey="paper_type" params={params} onChange={update} />
      <SelectField label="默认作用域" paramKey="default_scope" params={params} onChange={update} />
    </div>
  );
}

function SelectField({
  label,
  paramKey,
  params,
  onChange,
}: {
  label: string;
  paramKey: keyof typeof selectOptions;
  params: Record<string, unknown>;
  onChange: (key: string, value: string) => void;
}) {
  return (
    <label className="grid gap-1 text-xs text-muted-foreground">
      {label}
      <select
        aria-label={label}
        className="h-9 rounded-md border border-input bg-background px-3 text-sm text-foreground shadow-sm"
        value={textParam(params, paramKey)}
        onChange={(event) => onChange(paramKey, event.target.value)}
      >
        <option value="">请选择</option>
        {selectOptions[paramKey].map(([optionValue, optionLabel]) => (
          <option key={optionValue} value={optionValue}>
            {optionLabel}
          </option>
        ))}
      </select>
    </label>
  );
}

function textParam(params: Record<string, unknown>, key: string) {
  return typeof params[key] === "string" ? params[key] : "";
}
