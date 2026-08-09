import { Input, StatusBadge } from "@wms/ui";

import {
  COLUMN_DOCUMENT_TYPE,
  COLUMN_TEMP_ZONE,
  DOC_TYPE_PURCHASE_RETURN,
  FIELD_PLATE_NO,
  STATUS_PENDING_INPUT,
} from "@/lib/ui-strings";
import type { OutboundOrder, PurchaseReturnOrder } from "./m4-outbound-page-model";

export function OutboundPageErrors({
  messages,
}: {
  messages: Array<string | null | undefined>;
}) {
  return (
    <>
      {messages.filter(Boolean).map((message) => (
        <div
          key={message}
          className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive"
          role="alert"
        >
          {message}
        </div>
      ))}
    </>
  );
}

export function TextField({
  label,
  value,
  onChange,
  type = "text",
  placeholder,
  className,
  required = false,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  type?: string;
  placeholder?: string;
  className?: string;
  required?: boolean;
}) {
  return (
    <label className={className}>
      <span className="mb-1 block text-xs text-muted-foreground">{label}</span>
      <Input required={required} type={type} value={value} placeholder={placeholder} onChange={(event) => onChange(event.target.value)} />
    </label>
  );
}

/** 主显：商品编码 + 件数；批号 / 行数 / 校验另列展示 */
export function ProductSummary({ order }: { order: OutboundOrder }) {
  // lines 可能在空查询/不完整载荷中缺失；`order.lines[0]` 会直接抛错拖垮列表壳层
  const first = order.lines?.[0];
  return (
    <div className="text-sm">
      <div className="font-medium">{first?.product_code ?? "-"}</div>
      <div className="text-xs text-muted-foreground">{lineTotalPlannedQty(order)} 件</div>
    </div>
  );
}

/** 复核列表降噪：计划件数 + 短拣短文案，不堆「复核模式」等次要信息 */
export function ReviewSummary({ order }: { order: OutboundOrder }) {
  return (
    <div className="text-sm">
      <div className="font-medium">{lineTotalPlannedQty(order)} 件</div>
      <div className="text-xs text-muted-foreground">{order.short_pick ? "有短拣" : "无短拣"}</div>
    </div>
  );
}

/** 类型文案完整展示「销售出库」，避免列宽不足截成「销」 */
export function OrderNoSummary({ order }: { order: OutboundOrder }) {
  return (
    <div className="text-sm">
      <div className="whitespace-nowrap font-medium text-primary">{order.wms_order_no}</div>
      <div className="whitespace-nowrap text-xs text-muted-foreground">
        {order.erp_order_no ?? "-"} · {documentTypeLabel(order.document_type)}
      </div>
    </div>
  );
}

export function documentTypeLabel(value: string) {
  return value === "purchase_return_outbound" ? DOC_TYPE_PURCHASE_RETURN : "销售出库";
}

export function BatchNoCell({ order }: { order: OutboundOrder }) {
  const first = order.lines?.[0];
  return <span className="font-mono text-sm">{first?.batch_no ?? "-"}</span>;
}

export function ValidationBadge({ order }: { order: OutboundOrder }) {
  const failed = order.status === "validation_exception";
  return (
    <StatusBadge
      status={failed ? "unqualified" : "completed"}
      label={failed ? "异常" : "通过"}
      size="sm"
    />
  );
}

/** 优先可读客户名；仅有 id 时 shortId。门店名无真实字段时不展示，不得虚构。 */
export function CustomerCell({ customerId, storeName }: { customerId: string; storeName?: string }) {
  const name = customerDisplayName(customerId);
  return (
    <div className="text-sm">
      <div className={`whitespace-nowrap ${name ? "font-medium" : "font-mono"}`}>{name ?? shortId(customerId)}</div>
      {storeName && <div className="text-xs text-muted-foreground whitespace-nowrap">{storeName}</div>}
    </div>
  );
}

export function TwoLine({ top, bottom }: { top: string; bottom: string }) {
  return (
    <div className="text-sm">
      <div className="whitespace-nowrap font-medium text-primary">{top}</div>
      <div className="whitespace-nowrap text-xs text-muted-foreground">{bottom}</div>
    </div>
  );
}

function customerDisplayName(customerId: string) {
  // 列表 seed / 新建默认客户：可读名优先于裸 UUID
  if (customerId === "00000000-0000-0000-0000-000000004001") return "连锁门店 A";
  return null;
}

function shortId(value: string) {
  return value.slice(0, 8);
}

export function purchaseReturnDocumentTypeLabel(value: PurchaseReturnOrder["document_type"]) {
  return value === "purchase_return_outbound" ? DOC_TYPE_PURCHASE_RETURN : value;
}

export function purchaseReturnApprovalSourceLabel(value: PurchaseReturnOrder["approval_source"] | string) {
  if (value === "purchase_return_approval") return "采购退货审批";
  return value;
}

export function ActionExtraFields({ kind }: { kind: string }) {
  return (
    <>
      {extraActionFields(kind).map(([label, value, placeholder]) => (
        <StaticField key={label} label={label} value={value} placeholder={placeholder} />
      ))}
    </>
  );
}

/**
 * 动作弹窗的补充说明字段：只读展示，值从不提交。
 * 有真实来源的才给 value（流程常量、行为说明）；采集尚未接入的字段一律空值 + 「待录入」，
 * 不得预填虚构的工位码 / 车牌 / 供应商等业务数据。
 */
function extraActionFields(kind: string): Array<[string, string, string?]> {
  if (kind === "release-wave" || kind === "create-wave") return [["路径策略", "", STATUS_PENDING_INPUT], [COLUMN_TEMP_ZONE, "", STATUS_PENDING_INPUT], ["容量上限", "", STATUS_PENDING_INPUT]];
  if (kind === "review") return [["工位码", "", STATUS_PENDING_INPUT], ["实际复核数量", "按扫码累计"], ["短拣标识", "", STATUS_PENDING_INPUT], ["复核人", "当前用户"]];
  if (kind === "ship" || kind === "ship-return") {
    return [
      ["配送方类型", "", STATUS_PENDING_INPUT],
      ["包裹数量", "", STATUS_PENDING_INPUT],
      [FIELD_PLATE_NO, "", STATUS_PENDING_INPUT],
      ["装车温度", "", "冷链时必填"],
      ["签字", "交接双方签字"],
    ];
  }
  if (kind === "approve-return" || kind === "reject-return" || kind === "pick-return" || kind === "review-return" || kind === "create-return") {
    return [
      [COLUMN_DOCUMENT_TYPE, DOC_TYPE_PURCHASE_RETURN],
      ["原采购入库单", "", STATUS_PENDING_INPUT],
      ["供应商", "", STATUS_PENDING_INPUT],
      ["退货原因", "", STATUS_PENDING_INPUT],
      ["商品", "", STATUS_PENDING_INPUT],
      ["数量", "", STATUS_PENDING_INPUT],
      ["审批来源", purchaseReturnApprovalSourceLabel("purchase_return_approval")],
    ];
  }
  return [["校验结果", "", "校验后回填"], ["审批来源", "", STATUS_PENDING_INPUT]];
}

/** 只读展示：值从不被读取提交，禁止渲染成可编辑输入框误导用户。 */
function StaticField({ label, value, placeholder }: { label: string; value: string; placeholder?: string }) {
  return (
    <label>
      <span className="mb-1 block text-xs text-muted-foreground">{label}</span>
      <Input readOnly disabled value={value} placeholder={placeholder ?? STATUS_PENDING_INPUT} />
    </label>
  );
}

function lineTotalPlannedQty(order: OutboundOrder) {
  return (order.lines ?? []).reduce((sum, line) => sum + Number(line.planned_qty), 0);
}
