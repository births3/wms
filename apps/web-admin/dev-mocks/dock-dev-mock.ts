import type { IncomingMessage, ServerResponse } from "node:http";

import { asNullableString, asString, readJsonBody, sendError, sendJson } from "./web-admin-dev-mock-core-common";
import { devOwnerId, devWarehouseId } from "./web-admin-dev-mock-model";

type DevDock = {
  id: string;
  warehouse_id: string;
  dock_code: string;
  dock_type: string;
  temperature_zone: string;
  status: string;
  maintenance_recovery_at: string | null;
  location_description: string | null;
  created_at: string;
  updated_at: string;
};

type DevDockAppointment = {
  id: string;
  owner_id: string;
  warehouse_id: string;
  dock_id: string;
  appointment_no: string;
  document_type: string;
  document_no: string;
  window_start_at: string;
  window_end_at: string;
  vehicle_plate_no: string | null;
  vehicle_type: string;
  driver_name: string;
  driver_phone: string;
  status: string;
  version: number;
  supersedes_id: string | null;
  created_at: string;
  updated_at: string;
};

const docks: DevDock[] = [
  {
    id: "00000000-0000-0000-0000-000000004001",
    warehouse_id: devWarehouseId,
    dock_code: "D-01",
    dock_type: "receiving",
    temperature_zone: "normal",
    status: "active",
    maintenance_recovery_at: null,
    location_description: "东门收货区",
    created_at: "2026-07-01T08:00:00.000Z",
    updated_at: "2026-07-01T08:00:00.000Z",
  },
  {
    id: "00000000-0000-0000-0000-000000004002",
    warehouse_id: devWarehouseId,
    dock_code: "D-02",
    dock_type: "both",
    temperature_zone: "cold_chain",
    status: "maintenance",
    maintenance_recovery_at: "2026-07-15T00:00:00.000Z",
    location_description: "冷链月台",
    created_at: "2026-07-02T08:00:00.000Z",
    updated_at: "2026-07-12T08:00:00.000Z",
  },
];

const appointments: DevDockAppointment[] = [];

export async function handleDockDevMock(req: IncomingMessage, res: ServerResponse, pathname: string) {
  const warehouseId = new URL(req.url ?? pathname, "http://wms.local").searchParams.get("warehouse_id");
  if (req.method === "POST" && pathname === "/api/v1/dock-appointments") {
    const body = await readJsonBody(req);
    const appointmentNo = asString(body.appointment_no, "");
    const dockId = asString(body.dock_id, "");
    const appointmentWarehouseId = asString(body.warehouse_id, "");
    const requiredFields = [
      appointmentNo,
      dockId,
      appointmentWarehouseId,
      asString(body.document_type, ""),
      asString(body.document_no, ""),
      asString(body.window_start_at, ""),
      asString(body.window_end_at, ""),
      asString(body.vehicle_type, ""),
      asString(body.driver_name, ""),
      asString(body.driver_phone, ""),
    ];
    if (requiredFields.some((value) => !value)) {
      sendError(res, 400, "M1-400", "预约字段不完整");
      return;
    }
    if (appointments.some((appointment) => appointment.appointment_no === appointmentNo)) {
      sendError(res, 409, "H_DOCK_APPOINTMENT_CONFLICT", "预约编号已存在");
      return;
    }
    const dock = docks.find((item) => item.id === dockId && item.warehouse_id === appointmentWarehouseId);
    if (!dock) {
      sendError(res, 404, "M1-404", "月台档案不存在");
      return;
    }
    const now = new Date().toISOString();
    const appointment: DevDockAppointment = {
      id: `00000000-0000-0000-0000-${String(5100 + appointments.length + 1).padStart(12, "0")}`,
      owner_id: devOwnerId,
      warehouse_id: appointmentWarehouseId,
      dock_id: dockId,
      appointment_no: appointmentNo,
      document_type: asString(body.document_type, ""),
      document_no: asString(body.document_no, ""),
      window_start_at: asString(body.window_start_at, now),
      window_end_at: asString(body.window_end_at, now),
      vehicle_plate_no: asNullableString(body.vehicle_plate_no),
      vehicle_type: asString(body.vehicle_type, ""),
      driver_name: asString(body.driver_name, ""),
      driver_phone: asString(body.driver_phone, ""),
      status: "pending",
      version: 1,
      supersedes_id: null,
      created_at: now,
      updated_at: now,
    };
    appointments.unshift(appointment);
    sendJson(res, 200, appointment);
    return;
  }
  const appointmentMatch = pathname.match(/^\/api\/v1\/dock-appointments\/([^/]+)$/);
  if (req.method === "PATCH" && appointmentMatch) {
    const current = appointments.find((item) => item.id === decodeURIComponent(appointmentMatch[1]));
    if (!current) { sendError(res, 404, "H_DOCK_APPOINTMENT_NOT_FOUND", "预约不存在"); return; }
    if (current.status === "arrived") { sendError(res, 409, "DOCK_ALREADY_COMPLETED", "已到达预约不可变更"); return; }
    if (current.status === "cancelled") { sendError(res, 409, "DOCK_APPOINTMENT_CANCELLED", "已取消预约不可变更"); return; }
    const body = await readJsonBody(req);
    const now = new Date().toISOString();
    current.status = "cancelled";
    current.updated_at = now;
    const next: DevDockAppointment = { ...current, id: `${current.id.slice(0, -4)}${String(5200 + appointments.length).padStart(4, "0")}`, status: "pending", version: current.version + 1, supersedes_id: current.id, window_start_at: asString(body.window_start_at, current.window_start_at), window_end_at: asString(body.window_end_at, current.window_end_at), vehicle_plate_no: asNullableString(body.vehicle_plate_no), vehicle_type: asString(body.vehicle_type, current.vehicle_type), driver_name: asString(body.driver_name, current.driver_name), driver_phone: asString(body.driver_phone, current.driver_phone), created_at: now, updated_at: now };
    appointments.unshift(next);
    sendJson(res, 200, next);
    return;
  }
  const cancelMatch = pathname.match(/^\/api\/v1\/dock-appointments\/([^/]+)\/cancel$/);
  if (req.method === "POST" && cancelMatch) {
    const appointment = appointments.find((item) => item.id === decodeURIComponent(cancelMatch[1]));
    if (!appointment) { sendError(res, 404, "H_DOCK_APPOINTMENT_NOT_FOUND", "预约不存在"); return; }
    if (appointment.status === "arrived") { sendError(res, 409, "DOCK_ALREADY_COMPLETED", "已到达预约不可取消"); return; }
    if (appointment.status !== "cancelled") { appointment.status = "cancelled"; appointment.updated_at = new Date().toISOString(); }
    sendJson(res, 200, appointment);
    return;
  }
  if (req.method === "GET" && pathname === "/api/v1/docks") {
    sendJson(res, 200, docks.filter((dock) => dock.warehouse_id === warehouseId));
    return;
  }
  if (req.method === "POST" && pathname === "/api/v1/docks") {
    const body = await readJsonBody(req);
    const dockCode = asString(body.dock_code, "").trim();
    if (!dockCode || docks.some((dock) => dock.warehouse_id === asString(body.warehouse_id, "") && dock.dock_code === dockCode)) {
      sendError(res, 409, "M1-409", "同一仓库的月台编号已存在或为空");
      return;
    }
    const now = new Date().toISOString();
    const dock: DevDock = {
      id: `00000000-0000-0000-0000-${String(4100 + docks.length + 1).padStart(12, "0")}`,
      warehouse_id: asString(body.warehouse_id, devWarehouseId),
      dock_code: dockCode,
      dock_type: asString(body.dock_type, "receiving"),
      temperature_zone: asString(body.temperature_zone, "normal"),
      status: "active",
      maintenance_recovery_at: null,
      location_description: asNullableString(body.location_description),
      created_at: now,
      updated_at: now,
    };
    docks.unshift(dock);
    sendJson(res, 200, dock);
    return;
  }
  const match = pathname.match(/^\/api\/v1\/docks\/([^/]+)$/);
  if (req.method === "PATCH" && match) {
    const dock = docks.find((item) => item.id === decodeURIComponent(match[1]) && item.warehouse_id === devWarehouseId);
    if (!dock) {
      sendError(res, 404, "M1-404", "月台档案不存在");
      return;
    }
    const body = await readJsonBody(req);
    dock.status = asString(body.status, dock.status);
    dock.maintenance_recovery_at = dock.status === "maintenance" ? asNullableString(body.maintenance_recovery_at) : null;
    dock.updated_at = new Date().toISOString();
    sendJson(res, 200, dock);
    return;
  }
  sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Dock dev mock route not found");
}
