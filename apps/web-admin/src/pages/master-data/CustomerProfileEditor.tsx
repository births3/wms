import * as React from "react";
import { Button, Input } from "@wms/ui";

import {
  useCustomerProfileQuery,
  useUpsertCustomerProfileMutation,
  type CustomerQualification,
  type UpsertCustomerProfileRequest,
} from "@/features/master-data/master-data-queries";

import { LOADING_SAVING } from "@/lib/ui-strings";

const emptyProfile: UpsertCustomerProfileRequest = {
  customer_type: "customer",
  contact_name: "",
  contact_phone: "",
  business_scope: [],
  qualification_certificates: [],
  chain_name: null,
};

export function CustomerProfileEditor({ customerId }: { customerId: string }) {
  const profileQuery = useCustomerProfileQuery(customerId);
  const saveMutation = useUpsertCustomerProfileMutation(customerId);
  const [form, setForm] = React.useState<UpsertCustomerProfileRequest>(emptyProfile);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    if (!profileQuery.data) return;
    setForm({
      customer_type: profileQuery.data.customer_type,
      contact_name: profileQuery.data.contact_name ?? "",
      contact_phone: profileQuery.data.contact_phone ?? "",
      business_scope: profileQuery.data.business_scope,
      qualification_certificates: profileQuery.data.qualification_certificates,
      chain_name: profileQuery.data.chain_name,
    });
  }, [profileQuery.data]);

  const update = (value: Partial<UpsertCustomerProfileRequest>) =>
    setForm((current) => ({ ...current, ...value }));
  const businessScope = form.business_scope ?? [];
  const qualifications = form.qualification_certificates ?? [];
  const updateQualification = (index: number, value: Partial<CustomerQualification>) =>
    update({
      qualification_certificates: qualifications.map((item, itemIndex) =>
        itemIndex === index ? { ...item, ...value } : item,
      ),
    });
  const save = async () => {
    setError(null);
    try {
      await saveMutation.mutateAsync({
        ...form,
        contact_name: form.contact_name.trim(),
        contact_phone: form.contact_phone.trim(),
        business_scope: businessScope.map((value) => value.trim()).filter(Boolean),
        chain_name: form.chain_name?.trim() || null,
        qualification_certificates: qualifications.map((item) => ({
          certificate_type: item.certificate_type.trim(),
          certificate_no: item.certificate_no.trim(),
          expires_at: item.expires_at || null,
        })),
      });
    } catch (value) {
      setError(value instanceof Error ? value.message : "保存客户档案扩展信息失败");
    }
  };

  return (
    <section className="grid gap-3 rounded-md border bg-muted/20 p-3 md:col-span-2" aria-label="客户门店信息">
      <div className="flex items-center justify-between gap-2">
        <div>
          <h3 className="text-sm font-semibold">客户门店信息</h3>
          <p className="text-xs text-muted-foreground">联系方式、经营范围、资质和连锁归属。</p>
        </div>
        <Button type="button" size="sm" onClick={() => void save()} disabled={saveMutation.isPending || profileQuery.isPending}>
          {saveMutation.isPending ? LOADING_SAVING : "保存档案"}
        </Button>
      </div>
      {profileQuery.isPending && <p className="text-xs text-muted-foreground">档案加载中...</p>}
      {profileQuery.error && <p className="text-xs text-destructive">{profileQuery.error.message}</p>}
      {!profileQuery.isPending && !profileQuery.error && (
        <div className="grid gap-2 md:grid-cols-2">
          <label className="grid gap-1 text-xs text-muted-foreground">
            <span>档案类型</span>
            <select
              aria-label="档案类型"
              className="h-9 rounded-md border border-input bg-background px-3 text-sm"
              value={form.customer_type}
              onChange={(event) => update({ customer_type: event.target.value })}
            >
              <option value="customer">客户</option>
              <option value="store">门店</option>
            </select>
          </label>
          <ProfileField label="联系人" value={form.contact_name} onChange={(contact_name) => update({ contact_name })} />
          <ProfileField label="联系电话" value={form.contact_phone} onChange={(contact_phone) => update({ contact_phone })} />
          <ProfileField label="所属连锁" value={form.chain_name ?? ""} onChange={(chain_name) => update({ chain_name })} />
          <label className="grid gap-1 text-xs text-muted-foreground md:col-span-2">
            <span>经营范围（可多选）</span>
            <Input
              aria-label="经营范围"
              value={businessScope.join(", ")}
              placeholder="处方药, 医疗器械"
                onChange={(event) => update({ business_scope: event.target.value.split(/[,，]/) })}
            />
          </label>
          <div className="grid gap-2 border-t pt-2 md:col-span-2">
            <div className="flex items-center justify-between gap-2">
              <span className="text-xs font-medium">资质证照</span>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => update({ qualification_certificates: [...qualifications, { certificate_type: "", certificate_no: "", expires_at: null }] })}
              >
                新增资质
              </Button>
            </div>
            {/* 资质证照为可增删列表且条目无稳定业务主键（新增即空对象），
                输入均为受控值（value + onChange 全量重渲染），index key 为有意保留 */}
            {qualifications.map((item, index) => (
              <div className="grid gap-2 md:grid-cols-[1fr_1fr_10rem_auto]" key={index}>
                <Input aria-label={`资质类型-${index + 1}`} placeholder="证照类型" value={item.certificate_type} onChange={(event) => updateQualification(index, { certificate_type: event.target.value })} />
                <Input aria-label={`资质编号-${index + 1}`} placeholder="证照编号" value={item.certificate_no} onChange={(event) => updateQualification(index, { certificate_no: event.target.value })} />
                <Input aria-label={`资质有效期-${index + 1}`} type="date" value={item.expires_at ?? ""} onChange={(event) => updateQualification(index, { expires_at: event.target.value || null })} />
                <Button type="button" variant="ghost" size="sm" onClick={() => update({ qualification_certificates: qualifications.filter((_, itemIndex) => itemIndex !== index) })}>移除</Button>
              </div>
            ))}
            {qualifications.length === 0 && <p className="text-xs text-muted-foreground">暂无资质证照。</p>}
          </div>
        </div>
      )}
      {error && <p className="text-xs text-destructive" role="alert">{error}</p>}
    </section>
  );
}

function ProfileField({ label, value, onChange }: { label: string; value: string; onChange: (value: string) => void }) {
  // 不加 required：本编辑器由自己的"保存档案"按钮（type=button）提交，
  // 原生 required 只会拦截外层客户编辑表单的提交，导致只改名称也无法保存
  return (
    <label className="grid gap-1 text-xs text-muted-foreground">
      <span>{label}</span>
      <Input value={value} onChange={(event) => onChange(event.target.value)} />
    </label>
  );
}
