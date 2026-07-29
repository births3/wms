import type { ShipOutboundOrderRequest } from "@/features/outbound/outbound-queries";
import type { DualPersonPolicy } from "@/features/validation-rules/dual-person-policy-queries";
import type {
  OutboundOrder,
  OutboundWave,
  PurchaseReturnOrder,
} from "./M4OutboundDetailDialog";

export const outboundOwnerId = "00000000-0000-0000-0000-000000000001";
export const outboundWarehouseId = "00000000-0000-0000-0000-000000003001";
export const outboundCustomerId = "00000000-0000-0000-0000-000000001201";

export function makeOrder(
  id: string,
  wmsNo: string,
  erpNo: string,
  status: string,
  qty: number,
  shortPick: boolean,
  now = "2026-06-27T09:00:00.000Z",
): OutboundOrder {
  return {
    id,
    owner_id: outboundOwnerId,
    document_type: "sales_outbound",
    customer_id: outboundCustomerId,
    delivery_address_id: "00000000-0000-0000-0000-000000001211",
    delivery_address_snapshot: {
      province: "上海市",
      city: "上海市",
      district: "浦东新区",
      detail_address: "示例路 1 号",
      contact_name: "门店收货人",
      contact_phone: "13800000000",
    },
    warehouse_id: outboundWarehouseId,
    wms_order_no: wmsNo,
    erp_order_no: erpNo,
    required_ship_at: "2026-06-28T09:00:00.000Z",
    status,
    short_pick: shortPick,
    created_at: now,
    updated_at: now,
    lines: [
      {
        line_no: 1,
        product_code: "P-M4-001",
        batch_no: "BATCH-OUT-202606",
        planned_qty: qty,
        picked_qty: shortPick ? qty - 2 : qty,
        reviewed_qty:
          status === "reviewed" || status === "shipped" ? qty : 0,
        shipped_qty: status === "shipped" ? qty : 0,
        short_pick_qty: shortPick ? 2 : 0,
      },
    ],
  };
}

export function makeReturn(returnNo: string): PurchaseReturnOrder {
  const now = new Date().toISOString();
  return {
    id: crypto.randomUUID(),
    owner_id: outboundOwnerId,
    warehouse_id: outboundWarehouseId,
    return_no: returnNo,
    document_type: "purchase_return_outbound",
    source_purchase_order_no: "ASN-M2-PC-0001",
    supplier_name: "华东医药供应商",
    reason: "供应商召回",
    approval_source: "purchase_return_approval",
    status: "pending_approval",
    product_code: "P-M4-001",
    qty: 3,
    created_at: now,
    updated_at: now,
  };
}

export function waveQty(wave: OutboundWave, orders: OutboundOrder[]) {
  const orderIds = wave.order_ids ?? [];
  return orders
    .filter((order) => orderIds.includes(order.id))
    .reduce(
      (sum, order) =>
        sum +
        (order.lines ?? []).reduce(
          (lineSum, line) => lineSum + line.planned_qty,
          0,
        ),
      0,
    );
}

export function waveLineCount(
  wave: OutboundWave,
  orders: OutboundOrder[],
) {
  const orderIds = wave.order_ids ?? [];
  return orders
    .filter((order) => orderIds.includes(order.id))
    .reduce((sum, order) => sum + (order.lines ?? []).length, 0);
}

export function formatDate(value: string | null | undefined) {
  return value ? value.slice(0, 10) : "-";
}

export function toInteger(value: string) {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : 0;
}

export function isUuid(value: string) {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(
    value,
  );
}

export interface OutboundShipForm {
  deliveryProviderType: string;
  vehicleNo: string;
  plateNo: string;
  driverUserId: string;
  courierName: string;
  courierPhone: string;
  signatureAttachmentId: string;
  loadingTemperatureCelsius: string;
  insulatedContainerNo: string;
  icePackCount: string;
  packageCount: string;
}

/** 承运方式选项与后端 wave4 交接契约保持一致（own_fleet / third_party_express）。 */
export const outboundCarrierTypeOptions: Array<{
  value: string;
  label: string;
}> = [
  { value: "third_party_express", label: "第三方快递" },
  { value: "own_fleet", label: "自有车队" },
];

export function defaultOutboundShipForm(): OutboundShipForm {
  return {
    deliveryProviderType: "",
    vehicleNo: "",
    plateNo: "",
    driverUserId: "",
    courierName: "",
    courierPhone: "",
    signatureAttachmentId: "",
    loadingTemperatureCelsius: "",
    insulatedContainerNo: "",
    icePackCount: "",
    packageCount: "1",
  };
}

export function buildShipOutboundRequest(
  form: OutboundShipForm,
): ShipOutboundOrderRequest {
  const providerValid = outboundCarrierTypeOptions.some(
    (option) => option.value === form.deliveryProviderType,
  );
  if (!providerValid) throw new Error("请选择配送方类型");
  const plateNo = form.plateNo.trim();
  if (!plateNo) throw new Error("请填写车牌号");
  const packageCount = Number(form.packageCount);
  if (!Number.isInteger(packageCount) || packageCount <= 0) {
    throw new Error("件数必须为正整数");
  }

  const signatureAttachmentId = form.signatureAttachmentId.trim() || null;
  if (signatureAttachmentId && !isUuid(signatureAttachmentId)) {
    throw new Error("签字附件 ID 必须是 UUID");
  }

  const ownFleet = form.deliveryProviderType === "own_fleet";
  const vehicleNo = form.vehicleNo.trim() || null;
  const driverUserId = form.driverUserId.trim() || null;
  const courierName = form.courierName.trim() || null;
  const courierPhone = form.courierPhone.trim() || null;
  if (ownFleet) {
    if (!vehicleNo) throw new Error("请填写车辆编号");
    if (!driverUserId || !isUuid(driverUserId)) {
      throw new Error("司机用户 ID 必须是 UUID");
    }
  } else if (!courierName || !courierPhone || !signatureAttachmentId) {
    throw new Error("第三方快递须填写快递员姓名、电话和签字附件 ID");
  }

  const temperatureText = form.loadingTemperatureCelsius.trim();
  const containerNo = form.insulatedContainerNo.trim();
  const icePackText = form.icePackCount.trim();
  const hasColdChainFields = Boolean(
    temperatureText || containerNo || icePackText,
  );
  const loadingTemperature = temperatureText
    ? Number(temperatureText)
    : null;
  const icePackCount = icePackText ? Number(icePackText) : 0;
  if (
    hasColdChainFields &&
    (!temperatureText ||
      !Number.isFinite(loadingTemperature) ||
      !containerNo ||
      !Number.isInteger(icePackCount) ||
      icePackCount < 0)
  ) {
    throw new Error("冷链交接须填写有效装车温度、保温箱编号和冰袋数量");
  }

  return {
    delivery_provider_type: form.deliveryProviderType,
    vehicle_no: ownFleet ? vehicleNo : null,
    plate_no: plateNo,
    driver_user_id: ownFleet ? driverUserId : null,
    courier_name: ownFleet ? null : courierName,
    courier_phone: ownFleet ? null : courierPhone,
    signature_attachment_id: signatureAttachmentId,
    loading_temperature_celsius: loadingTemperature,
    cold_chain_packages: hasColdChainFields
      ? [
          {
            insulated_container_no: containerNo,
            ice_pack_count: icePackCount,
          },
        ]
      : [],
    package_count: packageCount,
  };
}

export function strictestDualPersonPolicy(
  policies: DualPersonPolicy[],
): DualPersonPolicy {
  if (policies.includes("dual_scan_with_approval")) {
    return "dual_scan_with_approval";
  }
  if (policies.includes("dual_scan")) return "dual_scan";
  return "single";
}
