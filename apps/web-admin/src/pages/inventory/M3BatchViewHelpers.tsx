import {
  StatusBadge,
  type StatusKey,
} from "@wms/ui";

import type { InventoryBatch } from "@/features/inventory/inventory-queries";

export type QualityStatusOption = { value: string; label: string };

export function availableQty(batch: InventoryBatch) {
  if (batch.quality_status !== "qualified" || batch.recall_flag) return 0;
  return batch.qty_on_hand - batch.qty_locked;
}

export function qualityStatusLabel(status: string, options: readonly QualityStatusOption[] = []) {
  const label = options.find((option) => option.value === status)?.label;
  return (typeof label === "string" ? label.trim() : "") || status || "-";
}

export function qualityStatusKey(status: string, recalled: boolean): StatusKey {
  if (recalled) return "isolated";
  if (status === "qualified") return "qualified";
  if (status === "quarantined" || status === "quarantine") return "isolated";
  if (status === "unqualified" || status === "pending_destruction" || status === "loss_deducted") {
    return "unqualified";
  }
  return "pending";
}

export function formatDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value || "-";
  return date.toLocaleString("zh-CN", { hour12: false });
}

export function ExpiryDateCell({
  expiryDate,
  warningDays,
}: {
  expiryDate: string;
  warningDays: number;
}) {
  const tone = expiryTone(expiryDate, warningDays);
  if (tone === "normal") return <span>{expiryDate || "-"}</span>;
  if (tone === "expired") {
    return (
      <div className="text-sm">
        <div className="font-medium text-destructive">{expiryDate || "-"}</div>
        <div className="text-xs text-destructive">已过期</div>
      </div>
    );
  }
  return (
    <div className="text-sm">
      <div className="font-medium text-wms-warning">{expiryDate || "-"}</div>
      <div className="text-xs text-wms-warning">近效期</div>
    </div>
  );
}

export function expiryTone(expiryDate: string, warningDays: number): "expired" | "near" | "normal" {
  const days = daysUntilExpiry(expiryDate);
  if (days === null) return "normal";
  if (days < 0) return "expired";
  if (days <= warningDays) return "near";
  return "normal";
}

export function expiryCopyValue(expiryDate: string, warningDays: number) {
  const tone = expiryTone(expiryDate, warningDays);
  if (tone === "expired") return `${expiryDate} 已过期`;
  if (tone === "near") return `${expiryDate} 近效期`;
  return expiryDate;
}

function daysUntilExpiry(expiryDate: string): number | null {
  const datePart = expiryDate.slice(0, 10);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(datePart)) return null;
  const expiry = new Date(`${datePart}T00:00:00`);
  if (Number.isNaN(expiry.getTime())) return null;
  const today = new Date();
  const startOfToday = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  return Math.floor((expiry.getTime() - startOfToday.getTime()) / 86_400_000);
}
