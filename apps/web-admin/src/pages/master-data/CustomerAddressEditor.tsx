import * as React from "react";
import { Button, Input } from "@wms/ui";

import {
  useCreateCustomerAddressMutation,
  useCustomerAddressesQuery,
  useUpdateCustomerAddressMutation,
  type CustomerAddress,
  type CreateCustomerAddressRequest,
} from "@/features/master-data/master-data-queries";

type AddressForm = CreateCustomerAddressRequest;

const emptyAddress: AddressForm = {
  province: "",
  city: "",
  district: "",
  detail_address: "",
  contact_name: "",
  contact_phone: "",
  is_default: false,
};

export function CustomerAddressEditor({ customerId }: { customerId: string }) {
  const addressesQuery = useCustomerAddressesQuery(customerId);
  const createMutation = useCreateCustomerAddressMutation(customerId);
  const updateMutation = useUpdateCustomerAddressMutation(customerId);
  const [editing, setEditing] = React.useState<CustomerAddress | null>(null);
  const [form, setForm] = React.useState<AddressForm>(emptyAddress);
  const [error, setError] = React.useState<string | null>(null);
  const pending = createMutation.isPending || updateMutation.isPending;

  const startCreate = () => {
    setEditing(null);
    setForm(emptyAddress);
    setError(null);
  };
  const startEdit = (address: CustomerAddress) => {
    setEditing(address);
    setForm({
      province: address.province,
      city: address.city,
      district: address.district,
      detail_address: address.detail_address,
      contact_name: address.contact_name,
      contact_phone: address.contact_phone,
      is_default: address.is_default,
    });
    setError(null);
  };
  const update = (value: Partial<AddressForm>) => setForm((current) => ({ ...current, ...value }));
  const submit = async () => {
    setError(null);
    if (!form.province.trim() || !form.city.trim() || !form.district.trim() || !form.detail_address.trim()
      || !form.contact_name.trim() || !form.contact_phone.trim()) {
      setError("省 / 市 / 区 / 详细地址 / 联系人 / 联系电话均为必填");
      return;
    }
    try {
      if (editing) {
        await updateMutation.mutateAsync({ addressId: editing.id, request: form });
      } else {
        await createMutation.mutateAsync(form);
      }
      startCreate();
    } catch (value) {
      setError(value instanceof Error ? value.message : "保存客户地址失败");
    }
  };

  return (
    <section className="grid gap-3 rounded-md border bg-muted/20 p-3 md:col-span-2" aria-label="客户收货地址">
      <div className="flex items-center justify-between gap-2">
        <div>
          <h3 className="text-sm font-semibold">收货地址</h3>
          <p className="text-xs text-muted-foreground">支持多个地址，默认地址只能有一个。</p>
        </div>
        <Button type="button" variant="outline" size="sm" onClick={startCreate} disabled={pending}>新增地址</Button>
      </div>
      {addressesQuery.isPending && <p className="text-xs text-muted-foreground">地址加载中...</p>}
      {addressesQuery.error && <p className="text-xs text-destructive">{addressesQuery.error.message}</p>}
      {!addressesQuery.isPending && !addressesQuery.error && (
        <div className="grid gap-2">
          {(addressesQuery.data ?? []).map((address) => (
            <div key={address.id} className="flex items-center justify-between gap-3 rounded border bg-background px-3 py-2 text-xs">
              <div className="min-w-0">
                <div className="truncate font-medium">{address.province}{address.city}{address.district}{address.detail_address}</div>
                <div className="text-muted-foreground">{address.contact_name} · {address.contact_phone}{address.is_default ? " · 默认" : ""}</div>
              </div>
              <Button type="button" variant="ghost" size="sm" onClick={() => startEdit(address)} disabled={pending}>编辑</Button>
            </div>
          ))}
          {(addressesQuery.data ?? []).length === 0 && <p className="text-xs text-muted-foreground">暂无收货地址。</p>}
        </div>
      )}
      <div className="grid gap-2 border-t pt-3 md:grid-cols-3" role="group" aria-label="客户地址编辑">
        <AddressField label="省" value={form.province} onChange={(province) => update({ province })} />
        <AddressField label="市" value={form.city} onChange={(city) => update({ city })} />
        <AddressField label="区" value={form.district} onChange={(district) => update({ district })} />
        <AddressField label="详细地址" value={form.detail_address} onChange={(detail_address) => update({ detail_address })} className="md:col-span-2" />
        <AddressField label="联系人" value={form.contact_name} onChange={(contact_name) => update({ contact_name })} />
        <AddressField label="联系电话" value={form.contact_phone} onChange={(contact_phone) => update({ contact_phone })} />
        <label className="flex items-center gap-2 text-xs text-muted-foreground">
          <input type="checkbox" checked={form.is_default} onChange={(event) => update({ is_default: event.target.checked })} />
          设为默认地址
        </label>
        <div className="flex items-end justify-end gap-2 md:col-span-2">
          {editing && <Button type="button" variant="ghost" size="sm" onClick={startCreate} disabled={pending}>取消编辑</Button>}
          <Button type="button" size="sm" disabled={pending} onClick={() => void submit()}>{pending ? "保存中..." : editing ? "保存地址" : "新增地址"}</Button>
        </div>
      </div>
      {error && <p className="text-xs text-destructive" role="alert">{error}</p>}
    </section>
  );
}

function AddressField({ label, value, onChange, className = "" }: { label: string; value: string; onChange: (value: string) => void; className?: string }) {
  // 不加 required：地址由本区块的按钮（type=button）提交并在 submit 里手动校验，
  // 原生 required 只会拦截外层客户编辑表单，导致空白新地址卡死整个弹窗的保存
  return (
    <label className={`grid gap-1 text-xs text-muted-foreground ${className}`}>
      <span>{label}</span>
      <Input value={value} onChange={(event) => onChange(event.target.value)} />
    </label>
  );
}
