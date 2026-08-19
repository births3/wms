import type { ShipOutboundOrderRequest } from "@/features/outbound/outbound-queries";
import type { DualPersonPolicy } from "@/features/validation-rules/dual-person-policy-queries";
import type {
  OutboundOrder,
  OutboundWave,
} from "./m4-outbound-page-model";

/** 过滤出波次内的订单（wave.order_ids 为空时返回空数组）。 */
export function waveOrders(wave: OutboundWave, orders: OutboundOrder[]) {
  const orderIds = wave.order_ids ?? [];
  return orders.filter((order) => orderIds.includes(order.id));
}

export function waveQty(wave: OutboundWave, orders: OutboundOrder[]) {
  return waveOrders(wave, orders).reduce(
    (sum, order) =>
      sum +
      (order.lines ?? []).reduce(
        (lineSum, line) => lineSum + Number(line.planned_qty),
        0,
      ),
    0,
  );
}

export function waveLineCount(wave: OutboundWave, orders: OutboundOrder[]) {
  return waveOrders(wave, orders).reduce(
    (sum, order) => sum + (order.lines ?? []).length,
    0,
  );
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
