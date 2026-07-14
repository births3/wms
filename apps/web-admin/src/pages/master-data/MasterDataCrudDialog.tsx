import * as React from "react";
import {
  Button,
  Dialog,
  DialogClose,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  type DataGridColumn,
} from "@wms/ui";
import { Ban, Pencil } from "lucide-react";
import { CustomerAddressEditor } from "./CustomerAddressEditor";
import { CustomerProfileEditor } from "./CustomerProfileEditor";

import {
  createLocation,
  createWarehouse,
  createWarehouseZone,
  updateCustomer,
  updateLocation,
  updateSupplier,
  updateWarehouse,
  updateWarehouseZone,
} from "@/features/master-data/master-data-queries";
import { isValidUnifiedSocialCreditCode, validateSupplierQualificationFields } from "./supplier-qualification-validation";
import type {
  CreateLocationRequest,
  CreateWarehouseRequest,
  CreateWarehouseZoneRequest,
  MasterDataRow,
  MasterDataViewId,
  UpdateCustomerRequest,
  UpdateLocationRequest,
  UpdateSupplierRequest,
  UpdateWarehouseRequest,
  UpdateWarehouseZoneRequest,
  SystemDictionaryOption,
} from "@/features/master-data/master-data-queries";

export type MasterDataCrudViewId = "m1-business-partners" | "m1-warehouses" | "m1-zones" | "m1-locations";

export interface LocationScopeOption { key: string; label: string; warehouseId: string; zoneId: string; ownerId: string | null; }

export type MasterDataCrudTarget =
  | { kind: "supplier"; mode: "edit"; row: MasterDataRow }
  | { kind: "customer"; mode: "edit"; row: MasterDataRow }
  | { kind: "warehouse"; mode: "create" }
  | { kind: "warehouse"; mode: "edit"; row: MasterDataRow }
  | { kind: "zone"; mode: "create" }
  | { kind: "zone"; mode: "edit"; row: MasterDataRow }
  | { kind: "location"; mode: "create" }
  | { kind: "location"; mode: "edit"; row: MasterDataRow };

export type SourceEditFormState =
  | { kind: "supplier"; mode: "edit"; id: string; code: string; name: string; licenseNo: string; contactName: string; status: string }
  | { kind: "customer"; mode: "edit"; id: string; code: string; name: string; licenseNo: string; status: string };

export interface WarehouseFormState { kind: "warehouse"; mode: "create" | "edit"; id?: string; code: string; name: string; warehouseType: string; status: string; }
export interface ZoneFormState { kind: "zone"; mode: "create" | "edit"; id?: string; warehouseId: string; code: string; name: string; temperatureZone: string; qualityColor: string; status: string; }

export interface LocationFormState { kind: "location"; mode: "create" | "edit"; id?: string; scopeKey: string; code: string; rowNo: number; columnNo: number; layerNo: number; maxVolumeCm3: number; usedVolumeCm3: number; maxSkuCount: number; locationType: string; status: string; }

export type MasterDataCrudForm = SourceEditFormState | WarehouseFormState | ZoneFormState | LocationFormState;

const activeOptions = [
  ["active", "启用"],
  ["disabled", "停用"],
] as const;
const warehouseTypeOptions = [
  ["physical", "物理仓"],
  ["logical", "逻辑仓"],
  ["virtual", "虚拟仓"],
] as const;
const locationStatusOptions = [
  ["available", "可用"],
  ["occupied", "占用"],
  ["locked", "锁定"],
  ["disabled", "停用"],
] as const;
export function isMasterDataCrudView(viewId: MasterDataViewId): viewId is MasterDataCrudViewId {
  return ["m1-business-partners", "m1-warehouses", "m1-zones", "m1-locations"].includes(viewId);
}

export function crudTargetForRow(viewId: MasterDataCrudViewId, row: MasterDataRow): MasterDataCrudTarget {
  if (viewId === "m1-business-partners") return businessPartnerCrudTarget(row);
  if (viewId === "m1-warehouses") return { kind: "warehouse", mode: "edit", row };
  if (viewId === "m1-zones") return { kind: "zone", mode: "edit", row };
  return { kind: "location", mode: "edit", row };
}

export function masterDataCrudColumns(
  base: DataGridColumn<MasterDataRow>[],
  viewId: MasterDataViewId,
  onEdit: (row: MasterDataRow) => void,
  onDisable: (row: MasterDataRow) => void,
  disablingId: string | null,
): DataGridColumn<MasterDataRow>[] {
  if (!isMasterDataCrudView(viewId)) return base;
  return [
    ...base,
    {
      key: "actions",
      header: "操作",
      width: 210,
      minWidth: 190,
      align: "right",
      sortable: false,
      filter: false,
      copyable: false,
      hideable: false,
      render: (row) => {
        const disabled = row.status === "disabled" || row.status === "inactive";
        return (
          <div className="flex justify-end gap-2">
            <Button type="button" variant="outline" size="sm" onClick={() => onEdit(row)}>
              <Pencil className="size-4" aria-hidden /> 编辑
            </Button>
            <Button type="button" variant="outline" size="sm" disabled={disabled || disablingId === row.id} onClick={() => onDisable(row)}>
              <Ban className="size-4" aria-hidden /> {disabled ? "已停用" : "停用"}
            </Button>
          </div>
        );
      },
    },
  ];
}

export function MasterDataCrudDialog({
  target,
  locationScopes,
  locationTypeOptions,
  warehouseOptions,
  temperatureZoneOptions,
  qualityColorOptions,
  onOpenChange,
  onSubmit,
}: {
  target: MasterDataCrudTarget | null;
  locationScopes: LocationScopeOption[];
  locationTypeOptions: SystemDictionaryOption[];
  warehouseOptions: MasterDataRow[];
  temperatureZoneOptions: SystemDictionaryOption[];
  qualityColorOptions: SystemDictionaryOption[];
  onOpenChange: (open: boolean) => void;
  onSubmit: (form: MasterDataCrudForm) => Promise<void>;
}) {
  const [form, setForm] = React.useState<MasterDataCrudForm | null>(null);
  const [pending, setPending] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    setForm(target ? formFromTarget(target, locationScopes[0] ?? null, warehouseOptions[0] ?? null) : null);
    setError(null);
  }, [target, locationScopes, warehouseOptions]);

  if (!target || !form) return null;
  const patch = (value: Partial<MasterDataCrudForm>) => {
    setForm((current) => (current ? ({ ...current, ...value } as MasterDataCrudForm) : current));
    setError(null);
  };
  const submit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setPending(true);
    setError(null);
    try {
      await onSubmit(form);
      onOpenChange(false);
    } catch (errorValue) {
      setError(errorValue instanceof Error ? errorValue.message : "保存基础档案失败");
    } finally {
      setPending(false);
    }
  };

  return (
    <Dialog open onOpenChange={(open) => !open && !pending && onOpenChange(false)}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-3xl">
        <form className="grid gap-4" onSubmit={submit}>
          <DialogHeader>
            <DialogTitle>{title(form)}</DialogTitle>
          </DialogHeader>
          <div className="grid gap-4 md:grid-cols-2">
            {form.kind === "supplier" && (
              <>
                <TextField label="供应商编码" value={form.code} disabled onChange={() => undefined} />
                <TextField label="供应商名称" value={form.name} required onChange={(name) => patch({ name })} />
                <TextField label="统一社会信用代码" required value={form.licenseNo} onChange={(licenseNo) => patch({ licenseNo })} />
                <TextField label="联系人" required value={form.contactName} onChange={(contactName) => patch({ contactName })} />
                <SelectField label="状态" value={form.status} options={activeOptions} onChange={(status) => patch({ status })} />
              </>
            )}
            {form.kind === "customer" && (
              <>
                <TextField label="客户编码" value={form.code} disabled onChange={() => undefined} />
                <TextField label="客户名称" value={form.name} required onChange={(name) => patch({ name })} />
                <TextField label="资质证号" value={form.licenseNo} onChange={(licenseNo) => patch({ licenseNo })} />
                <SelectField label="状态" value={form.status} options={activeOptions} onChange={(status) => patch({ status })} />
                <CustomerProfileEditor customerId={form.id} />
                <CustomerAddressEditor customerId={form.id} />
              </>
            )}
            {form.kind === "warehouse" && (
              <>
                <TextField label="仓库编码" value={form.code} required disabled={form.mode === "edit"} onChange={(code) => patch({ code })} />
                <TextField label="仓库名称" value={form.name} required onChange={(name) => patch({ name })} />
                <SelectField label="仓库类型" value={form.warehouseType} options={warehouseTypeOptions} onChange={(warehouseType) => patch({ warehouseType })} />
                {form.mode === "edit" && <SelectField label="状态" value={form.status} options={activeOptions} onChange={(status) => patch({ status })} />}
              </>
            )}
            {form.kind === "zone" && (
              <>
                <SelectField label="仓库" value={form.warehouseId} options={warehouseOptions.map((row) => [row.id, `${row.code} · ${row.name}`] as const)} onChange={(warehouseId) => patch({ warehouseId })} />
                <TextField label="库区编码" value={form.code} required disabled={form.mode === "edit"} onChange={(code) => patch({ code })} />
                <TextField label="库区名称" value={form.name} required onChange={(name) => patch({ name })} />
                <SelectField label="温区" value={form.temperatureZone} options={temperatureZoneOptions} onChange={(temperatureZone) => patch({ temperatureZone })} />
                <SelectField label="色标" value={form.qualityColor} options={qualityColorOptions} onChange={(qualityColor) => patch({ qualityColor })} />
                {form.mode === "edit" && <SelectField label="状态" value={form.status} options={activeOptions} onChange={(status) => patch({ status })} />}
              </>
            )}
            {form.kind === "location" && (
              <>
                {form.mode === "create" ? (
                  <SelectField label="仓库 / 库区" value={form.scopeKey} options={scopeOptions(locationScopes)} onChange={(scopeKey) => patch({ scopeKey })} />
                ) : (
                  <TextField label="仓库 / 库区" value={locationScopes.find((scope) => scope.key === form.scopeKey)?.label ?? form.scopeKey} disabled onChange={() => undefined} />
                )}
                <TextField label="库位编码" value={form.code} required onChange={(code) => patch({ code })} />
                <NumberField label="排" value={form.rowNo} min={1} onChange={(rowNo) => patch({ rowNo })} />
                <NumberField label="列" value={form.columnNo} min={1} onChange={(columnNo) => patch({ columnNo })} />
                <NumberField label="层" value={form.layerNo} min={1} onChange={(layerNo) => patch({ layerNo })} />
                <NumberField label="最大容积 cm³" value={form.maxVolumeCm3} min={1} onChange={(maxVolumeCm3) => patch({ maxVolumeCm3 })} />
                {form.mode === "edit" && <NumberField label="当前已用容积 cm³" value={form.usedVolumeCm3} min={0} onChange={(usedVolumeCm3) => patch({ usedVolumeCm3 })} />}
                <NumberField label="最大 SKU 数" value={form.maxSkuCount} min={1} onChange={(maxSkuCount) => patch({ maxSkuCount })} />
                <SelectField label="库位类型" value={form.locationType} options={locationTypeOptions} onChange={(locationType) => patch({ locationType })} />
                {form.mode === "edit" && <SelectField label="状态" value={form.status} options={locationStatusOptions} onChange={(status) => patch({ status })} />}
              </>
            )}
          </div>
          {error && <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">{error}</div>}
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline" disabled={pending}>取消</Button>
            </DialogClose>
            <Button type="submit" disabled={pending || !canSubmit(form, locationScopes, locationTypeOptions)}>{pending ? "保存中..." : "保存"}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

export const supplierEditRequestFromForm = (form: Extract<SourceEditFormState, { kind: "supplier" }>): UpdateSupplierRequest => ({
  supplier_name: requiredText(form.name),
  license_no: requiredSupplierQualification(form).unifiedSocialCreditCode,
  contact_name: requiredSupplierQualification(form).contactName,
  status: form.status,
});

export const customerEditRequestFromForm = (form: Extract<SourceEditFormState, { kind: "customer" }>): UpdateCustomerRequest => ({
  customer_name: requiredText(form.name),
  license_no: nullableText(form.licenseNo),
  status: form.status,
});

export const warehouseCreateRequestFromForm = (form: WarehouseFormState): CreateWarehouseRequest => ({
  warehouse_code: requiredText(form.code),
  warehouse_name: requiredText(form.name),
  warehouse_type: form.warehouseType,
});

export const warehouseEditRequestFromForm = (form: WarehouseFormState): UpdateWarehouseRequest => ({
  warehouse_name: requiredText(form.name),
  warehouse_type: form.warehouseType,
  status: form.status,
});

export const zoneCreateRequestFromForm = (form: ZoneFormState): CreateWarehouseZoneRequest => ({
  warehouse_id: form.warehouseId,
  zone_code: requiredText(form.code),
  zone_name: requiredText(form.name),
  temperature_zone: form.temperatureZone,
  quality_color: form.qualityColor,
});

export const zoneEditRequestFromForm = (form: ZoneFormState): UpdateWarehouseZoneRequest => ({
  zone_name: requiredText(form.name), temperature_zone: form.temperatureZone,
  quality_color: form.qualityColor, status: form.status,
});

export function locationCreateRequestFromForm(form: LocationFormState, scopes: LocationScopeOption[]): CreateLocationRequest {
  const scope = scopes.find((option) => option.key === form.scopeKey);
  if (!scope) throw new Error("缺少仓库 / 库区上下文");
  return {
    warehouse_id: scope.warehouseId,
    zone_id: scope.zoneId,
    bound_owner_id: scope.ownerId,
    location_code: requiredText(form.code),
    row_no: positive(form.rowNo, "排"),
    column_no: positive(form.columnNo, "列"),
    layer_no: positive(form.layerNo, "层"),
    max_volume_cm3: positive(form.maxVolumeCm3, "最大容积"),
    max_sku_count: positive(form.maxSkuCount, "最大 SKU 数"),
    location_type: form.locationType,
  };
}

export const locationEditRequestFromForm = (form: LocationFormState): UpdateLocationRequest => ({
  location_code: requiredText(form.code),
  row_no: positive(form.rowNo, "排"),
  column_no: positive(form.columnNo, "列"),
  layer_no: positive(form.layerNo, "层"),
  max_volume_cm3: positive(form.maxVolumeCm3, "最大容积"),
  used_volume_cm3: nonNegative(form.usedVolumeCm3, "当前已用容积"),
  max_sku_count: positive(form.maxSkuCount, "最大 SKU 数"),
  location_type: form.locationType,
  status: form.status,
});

export async function disableMasterDataCrudRow(
  viewId: MasterDataCrudViewId,
  row: MasterDataRow,
): Promise<MasterDataRow> {
  if (viewId === "m1-business-partners" && row.partnerKind === "supplier") {
    return updateSupplier({ id: row.id, request: { status: "disabled" } });
  }
  if (viewId === "m1-business-partners" && row.partnerKind === "customer") {
    return updateCustomer({ id: row.id, request: { status: "disabled" } });
  }
  if (viewId === "m1-business-partners") throw new Error("缺少客商类型，无法停用");
  if (viewId === "m1-warehouses") {
    return updateWarehouse({ id: row.id, request: { status: "disabled" } });
  }
  if (viewId === "m1-zones") return updateWarehouseZone({ id: row.id, request: { status: "disabled" } });
  return updateLocation({ id: row.id, request: { status: "disabled" } });
}

export async function saveMasterDataCrudForm(
  form: MasterDataCrudForm,
  scopes: LocationScopeOption[],
): Promise<MasterDataRow> {
  if (form.kind === "supplier") {
    return updateSupplier({ id: form.id, request: supplierEditRequestFromForm(form) });
  }
  if (form.kind === "customer") {
    return updateCustomer({ id: form.id, request: customerEditRequestFromForm(form) });
  }
  if (form.kind === "warehouse" && form.mode === "create") {
    return createWarehouse(warehouseCreateRequestFromForm(form));
  }
  if (form.kind === "warehouse") {
    return updateWarehouse({ id: requiredRecordId(form.id), request: warehouseEditRequestFromForm(form) });
  }
  if (form.kind === "zone" && form.mode === "create") return createWarehouseZone(zoneCreateRequestFromForm(form));
  if (form.kind === "zone") return updateWarehouseZone({ id: requiredRecordId(form.id), request: zoneEditRequestFromForm(form) });
  if (form.mode === "create") {
    return createLocation(locationCreateRequestFromForm(form, scopes));
  }
  return updateLocation({ id: requiredRecordId(form.id), request: locationEditRequestFromForm(form) });
}

function formFromTarget(target: MasterDataCrudTarget, firstScope: LocationScopeOption | null, firstWarehouse: MasterDataRow | null): MasterDataCrudForm {
  if (target.kind === "supplier") return { kind: "supplier", mode: "edit", id: target.row.id, code: target.row.code, name: target.row.name, licenseNo: clean(target.row.primaryValue), contactName: clean(target.row.secondaryValue), status: target.row.status || "active" };
  if (target.kind === "customer") return { kind: "customer", mode: "edit", id: target.row.id, code: target.row.code, name: target.row.name, licenseNo: clean(target.row.primaryValue), status: target.row.status || "active" };
  if (target.kind === "warehouse" && target.mode === "edit") return { kind: "warehouse", mode: "edit", id: target.row.id, code: target.row.code, name: target.row.name, warehouseType: target.row.warehouseFields?.warehouseType || "physical", status: target.row.status || "active" };
  if (target.kind === "warehouse") return { kind: "warehouse", mode: "create", code: "", name: "", warehouseType: "physical", status: "active" };
  if (target.kind === "zone" && target.mode === "edit") {
    const fields = target.row.zoneFields;
    return { kind: "zone", mode: "edit", id: target.row.id, warehouseId: fields?.warehouseId ?? "", code: target.row.code, name: target.row.name, temperatureZone: target.row.secondaryValue, qualityColor: target.row.extraValue, status: target.row.status || "active" };
  }
  if (target.kind === "zone") return { kind: "zone", mode: "create", warehouseId: firstWarehouse?.id ?? "", code: "", name: "", temperatureZone: "normal", qualityColor: "qualified_green", status: "active" };
  if (target.kind === "location" && target.mode === "edit") {
    const fields = target.row.locationFields;
    return {
      kind: "location",
      mode: "edit",
      id: target.row.id,
      scopeKey: fields
        ? `${fields.warehouseId}:${fields.zoneId}:${fields.owner === "-" ? "none" : fields.owner}`
        : "",
      code: target.row.code,
      rowNo: int(fields?.rowNo, 1),
      columnNo: int(fields?.columnNo, 1),
      layerNo: int(fields?.layerNo, 1),
      maxVolumeCm3: int(fields?.maxVolumeCm3, 5_000_000),
      usedVolumeCm3: int(fields?.usedVolumeCm3, 0),
      maxSkuCount: int(fields?.maxSku, 1),
      locationType: clean(fields?.locationTypeCode) || "storage",
      status: target.row.status || "available",
    };
  }
  return { kind: "location", mode: "create", scopeKey: firstScope?.key ?? "", code: "", rowNo: 1, columnNo: 1, layerNo: 1, maxVolumeCm3: 5_000_000, usedVolumeCm3: 0, maxSkuCount: 1, locationType: "storage", status: "available" };
}

function businessPartnerCrudTarget(row: MasterDataRow): MasterDataCrudTarget {
  if (row.partnerKind === "supplier") return { kind: "supplier", mode: "edit", row };
  if (row.partnerKind === "customer") return { kind: "customer", mode: "edit", row };
  throw new Error("缺少客商类型，无法编辑");
}

function title(form: MasterDataCrudForm) {
  if (form.kind === "supplier") return "编辑供应商";
  if (form.kind === "customer") return "编辑客户";
  if (form.kind === "warehouse") return form.mode === "create" ? "新建仓库" : "编辑仓库";
  if (form.kind === "zone") return form.mode === "create" ? "新建库区" : "编辑库区";
  return form.mode === "create" ? "新建库位" : "编辑库位";
}

function canSubmit(
  form: MasterDataCrudForm,
  scopes: LocationScopeOption[],
  locationTypeOptions: SystemDictionaryOption[],
) {
  if (form.kind === "location") {
    const hasScope = form.mode === "edit" || scopes.some((scope) => scope.key === form.scopeKey);
    const hasLocationType = locationTypeOptions.some(([value]) => value === form.locationType);
    return hasScope && hasLocationType && !!form.code.trim() && form.rowNo > 0 && form.columnNo > 0 && form.layerNo > 0 && form.maxVolumeCm3 > 0 && form.usedVolumeCm3 >= 0 && form.maxSkuCount > 0;
  }
  if (form.kind === "zone") return !!form.warehouseId && !!form.code.trim() && !!form.name.trim() && !!form.temperatureZone && !!form.qualityColor;
  if (form.kind === "supplier") return !!form.name.trim() && isValidUnifiedSocialCreditCode(form.licenseNo) && !!form.contactName.trim();
  return !!form.name.trim() && (form.kind !== "warehouse" || !!form.code.trim());
}

function TextField({ label, value, onChange, disabled, required }: { label: string; value: string; onChange: (value: string) => void; disabled?: boolean; required?: boolean }) {
  return <label className="grid gap-1.5 text-sm"><span className="font-medium">{label}</span><Input value={value} disabled={disabled} required={required} onChange={(event) => onChange(event.target.value)} /></label>;
}

function NumberField({ label, value, min, onChange }: { label: string; value: number; min: number; onChange: (value: number) => void }) {
  return <label className="grid gap-1.5 text-sm"><span className="font-medium">{label}</span><Input type="number" min={min} value={value} onChange={(event) => onChange(int(event.target.value, 0))} /></label>;
}

function SelectField({ label, value, options, onChange }: { label: string; value: string; options: readonly (readonly [string, string])[]; onChange: (value: string) => void }) {
  return <label className="grid gap-1.5 text-sm"><span className="font-medium">{label}</span><select value={value} onChange={(event) => onChange(event.target.value)} className="h-9 rounded-md border border-input bg-background px-3 text-sm">{options.map(([optionValue, optionLabel]) => <option key={optionValue} value={optionValue}>{optionLabel}</option>)}</select></label>;
}

const scopeOptions = (scopes: LocationScopeOption[]) =>
  scopes.length > 0 ? scopes.map((scope) => [scope.key, scope.label] as const) : ([["", "暂无可用仓库 / 库区"]] as const);
const requiredText = (value: string) => {
  const text = value.trim();
  if (!text) throw new Error("必填字段不能为空");
  return text;
};
const nullableText = (value: string) => {
  const text = value.trim();
  return text ? text : null;
};
const requiredSupplierQualification = (form: Extract<SourceEditFormState, { kind: "supplier" }>) => {
  const unifiedSocialCreditCode = requiredText(form.licenseNo);
  const contactName = requiredText(form.contactName);
  validateSupplierQualificationFields({ unifiedSocialCreditCode, contactName });
  return { unifiedSocialCreditCode, contactName };
};
const positive = (value: number, label: string) => {
  if (!Number.isFinite(value) || value <= 0) throw new Error(`${label}必须大于 0`);
  return value;
};
const nonNegative = (value: number, label: string) => {
  if (!Number.isFinite(value) || value < 0) throw new Error(`${label}不能小于 0`);
  return value;
};
const requiredRecordId = (id: string | undefined) => {
  if (!id) throw new Error("缺少档案 ID");
  return id;
};
const clean = (value: unknown) => (typeof value === "string" && value.trim() !== "-" ? value.trim() : "");
const int = (value: unknown, fallback: number) => {
  const parsed = Number.parseInt(String(value ?? ""), 10);
  return Number.isFinite(parsed) ? parsed : fallback;
};
