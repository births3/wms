import * as React from "react";
import type { ReactNode } from "react";

import type { H4WechatNotifyMode } from "@/pages/wechat-notify/H4WechatNotifyPage";
import type { M2InboundMode } from "@/pages/inbound/M2InboundPage";
import type { MasterDataViewId } from "@/pages/master-data/M1MasterDataPage";
import type { M4OutboundMode } from "@/pages/outbound/M4OutboundPage";
import type { CurrentUser } from "@/features/auth/auth-queries";
import type { AdminView } from "./admin-view";

// 页面按视图懒加载：每个页面单独分包，首屏只需要工作台外壳。
const FeatureFlagConfigCenterPage = React.lazy(() => import("@/pages/config-center/FeatureFlagConfigCenterPage").then((m) => ({ default: m.FeatureFlagConfigCenterPage })));
const ErpConnectorConfigPage = React.lazy(() => import("@/pages/config-center/ErpConnectorConfigPage").then((m) => ({ default: m.ErpConnectorConfigPage })));
const ErpMessageLogPage = React.lazy(() => import("@/pages/config-center/ErpMessageLogPage").then((m) => ({ default: m.ErpMessageLogPage })));
const ErpInterfaceTablePage = React.lazy(() => import("@/pages/config-center/ErpInterfaceTablePage").then((m) => ({ default: m.ErpInterfaceTablePage })));
const H1AdminMenuPage = React.lazy(() => import("@/pages/admin-menu/H1AdminMenuPage").then((m) => ({ default: m.H1AdminMenuPage })));
const H1RolePermissionPage = React.lazy(() => import("@/pages/auth/H1RolePermissionPage").then((m) => ({ default: m.H1RolePermissionPage })));
const H1SessionPage = React.lazy(() => import("@/pages/auth/H1SessionPage").then((m) => ({ default: m.H1SessionPage })));
const H1ApiKeyPage = React.lazy(() => import("@/pages/api-key/H1ApiKeyPage").then((m) => ({ default: m.H1ApiKeyPage })));
const H4WechatNotifyPage = React.lazy(() => import("@/pages/wechat-notify/H4WechatNotifyPage").then((m) => ({ default: m.H4WechatNotifyPage })));
const M2InboundPage = React.lazy(() => import("@/pages/inbound/M2InboundPage").then((m) => ({ default: m.M2InboundPage })));
const M2PutawayStrategyPage = React.lazy(() => import("@/pages/inbound/M2PutawayStrategyPage").then((m) => ({ default: m.M2PutawayStrategyPage })));
const M3BatchManagementPage = React.lazy(() => import("@/pages/inventory/M3BatchManagementPage").then((m) => ({ default: m.M3BatchManagementPage })));
const M3InventoryStatusConfigPage = React.lazy(() => import("@/pages/inventory/M3InventoryStatusConfigPage").then((m) => ({ default: m.M3InventoryStatusConfigPage })));
const M3LocationHistoryPage = React.lazy(() => import("@/pages/inventory/M3LocationHistoryPage").then((m) => ({ default: m.M3LocationHistoryPage })));
const M3InventoryCountPage = React.lazy(() => import("@/pages/inventory/M3InventoryCountPage").then((m) => ({ default: m.M3InventoryCountPage })));
const M3MaintenancePage = React.lazy(() => import("@/pages/inventory/M3MaintenancePage").then((m) => ({ default: m.M3MaintenancePage })));
const M3RelocationPage = React.lazy(() => import("@/pages/inventory/M3RelocationPage").then((m) => ({ default: m.M3RelocationPage })));
const MrcReconciliationPage = React.lazy(() => import("@/pages/reconciliation/MrcReconciliationPage").then((m) => ({ default: m.MrcReconciliationPage })));
const M1MasterDataPage = React.lazy(() => import("@/pages/master-data/M1MasterDataPage").then((m) => ({ default: m.M1MasterDataPage })));
const M4OutboundPage = React.lazy(() => import("@/pages/outbound/M4OutboundPage").then((m) => ({ default: m.M4OutboundPage })));
const H2AuditTrailPage = React.lazy(() => import("@/pages/platform/HorizontalCapabilityPages").then((m) => ({ default: m.H2AuditTrailPage })));
const H3ApiContractPage = React.lazy(() => import("@/pages/platform/HorizontalCapabilityPages").then((m) => ({ default: m.H3ApiContractPage })));
const H5ExpressPage = React.lazy(() => import("@/pages/express/H5ExpressPage").then((m) => ({ default: m.H5ExpressPage })));
const AlertDefinitionPage = React.lazy(() => import("@/pages/alert-engine/AlertDefinitionPage").then((m) => ({ default: m.AlertDefinitionPage })));
const AlertDashboardPage = React.lazy(() => import("@/pages/alert-engine/AlertDashboardPage").then((m) => ({ default: m.AlertDashboardPage })));
const AlertEscalationPage = React.lazy(() => import("@/pages/alert-engine/AlertEscalationPage").then((m) => ({ default: m.AlertEscalationPage })));
const H9DeliveryNoteAggregationPage = React.lazy(() => import("@/pages/print-orchestration/H9DeliveryNoteAggregationPage").then((m) => ({ default: m.H9DeliveryNoteAggregationPage })));
const H9PrintDevicePage = React.lazy(() => import("@/pages/print-orchestration/H9PrintDevicePage").then((m) => ({ default: m.H9PrintDevicePage })));
const H9PrintTemplatePage = React.lazy(() => import("@/pages/print-template/H9PrintTemplatePage").then((m) => ({ default: m.H9PrintTemplatePage })));
const MCGDocumentNumberingPage = React.lazy(() => import("@/pages/document-numbering/MCGDocumentNumberingPage").then((m) => ({ default: m.MCGDocumentNumberingPage })));
const DockManagementPage = React.lazy(() => import("@/pages/dock/DockManagementPage").then((m) => ({ default: m.DockManagementPage })));
const DrugInspectionPlatformPage = React.lazy(() => import("@/pages/drug-inspection/DrugInspectionPlatformPage").then((m) => ({ default: m.DrugInspectionPlatformPage })));
const TaskTypeConfigPage = React.lazy(() => import("@/pages/task-engine/TaskTypeConfigPage").then((m) => ({ default: m.TaskTypeConfigPage })));
const TaskGroupConfigPage = React.lazy(() => import("@/pages/task-engine/TaskGroupConfigPage").then((m) => ({ default: m.TaskGroupConfigPage })));
const TaskDispatchPage = React.lazy(() => import("@/pages/task-engine/TaskDispatchPage").then((m) => ({ default: m.TaskDispatchPage })));
const BillingRuleConfigPage = React.lazy(() => import("@/pages/billing/BillingRuleConfigPage").then((m) => ({ default: m.BillingRuleConfigPage })));
const TmsRoutePlanPage = React.lazy(() => import("@/pages/tms/TmsRoutePlanPage").then((m) => ({ default: m.TmsRoutePlanPage })));

export function renderAdminView(
  view: AdminView,
  currentUser: CurrentUser,
  navigateTo: (view: AdminView) => void,
): ReactNode | null {
  const inboundMode = inboundViewToMode(view);
  const outboundMode = outboundViewToMode(view);
  const wechatNotifyMode = wechatNotifyViewToMode(view);
  const masterDataViewId = masterDataViewToId(view);

  if (view === "m1-feature-flags") {
    return <FeatureFlagConfigCenterPage onBack={() => navigateTo("dashboard")} />;
  }
  if (view === "h8-erp-connectors") {
    return (
      <ErpConnectorConfigPage
        currentUser={currentUser}
        onBack={() => navigateTo("dashboard")}
      />
    );
  }
  if (view === "h8-erp-messages") {
    return <ErpMessageLogPage />;
  }
  if (view === "h8-erp-interface-tables") {
    return <ErpInterfaceTablePage />;
  }
  if (masterDataViewId) {
    return <M1MasterDataPage currentUser={currentUser} viewId={masterDataViewId} onBack={() => navigateTo("dashboard")} />;
  }
  if (view === "dock-management") return <DockManagementPage />;
  if (view === "m-di-platforms") return <DrugInspectionPlatformPage currentUser={currentUser} />;
  if (inboundMode) {
    return (
      <M2InboundPage
        mode={inboundMode}
        currentOwner={{ ownerId: currentUser.owner_id, ownerCode: currentUser.owner_code }}
        onBack={() => navigateTo("dashboard")}
      />
    );
  }
  if (view === "m2-putaway-strategy") {
    return <M2PutawayStrategyPage currentUser={currentUser} />;
  }
  if (view === "m3-batches") {
    return (
      <M3BatchManagementPage
        onBack={() => navigateTo("dashboard")}
        onOpenLocationHistory={() => navigateTo("m3-location-history")}
      />
    );
  }
  if (view === "m3-location-history") {
    return <M3LocationHistoryPage onBack={() => navigateTo("m3-batches")} />;
  }
  if (view === "m3-status-config") return <M3InventoryStatusConfigPage currentUser={currentUser} />;
  if (view === "m3-counts") return <M3InventoryCountPage />;
  if (view === "m3-maintenance") return <M3MaintenancePage />;
  if (view === "m3-relocations") return <M3RelocationPage />;
  if (view === "mrc-reconciliation") return <MrcReconciliationPage currentUser={currentUser} />;
  if (view === "mte-task-types") return <TaskTypeConfigPage />;
  if (view === "mte-task-groups") return <TaskGroupConfigPage />;
  if (view === "mte-task-dispatch") return <TaskDispatchPage />;
  if (view === "m9-billing-rules") return <BillingRuleConfigPage />;
  if (view === "m10-route-plans") return <TmsRoutePlanPage />;
  if (outboundMode) {
    return <M4OutboundPage mode={outboundMode} onBack={() => navigateTo("dashboard")} />;
  }
  if (view === "h1-menu-management") return <H1AdminMenuPage />;
  if (view === "h1-role-permission") return <H1RolePermissionPage currentUser={currentUser} />;
  if (view === "h1-session-management") return <H1SessionPage currentUser={currentUser} />;
  if (view === "h1-api-keys") return <H1ApiKeyPage currentUser={currentUser} />;
  if (view === "h2-audit-trail") return <H2AuditTrailPage />;
  if (view === "h3-api-contract") return <H3ApiContractPage />;
  if (wechatNotifyMode) return <H4WechatNotifyPage mode={wechatNotifyMode} />;
  if (view === "hal-alert-dashboard") return <AlertDashboardPage />;
  if (view === "hal-alert-definitions") return <AlertDefinitionPage />;
  if (view === "hal-alert-escalations") return <AlertEscalationPage />;
  if (view === "h5-express") return <H5ExpressPage />;
  if (view === "h9-delivery-note-aggregation") return <H9DeliveryNoteAggregationPage currentUser={currentUser} />;
  if (view === "h9-print-devices") return <H9PrintDevicePage currentUser={currentUser} />;
  if (view === "h9-print-templates") return <H9PrintTemplatePage currentUser={currentUser} />;
  if (view === "mcg-numbering") return <MCGDocumentNumberingPage />;
  return null;
}

function inboundViewToMode(view: AdminView): M2InboundMode | null {
  if (view === "m2-receiving") return "receiving";
  if (view === "m2-inspecting") return "inspecting";
  if (view === "m2-putaway") return "putaway";
  return null;
}

function masterDataViewToId(view: AdminView): MasterDataViewId | null {
  if (
    view === "m1-products" ||
    view === "m1-business-partners" ||
    view === "m1-warehouses" ||
    view === "m1-zones" ||
    view === "m1-locations" ||
    view === "m1-system-dictionary"
  ) {
    return view;
  }
  return null;
}

function outboundViewToMode(view: AdminView): M4OutboundMode | null {
  if (view === "m4-orders") return "orders";
  if (view === "m4-waves") return "waves";
  if (view === "m4-review") return "review";
  if (view === "m4-returns") return "returns";
  return null;
}

function wechatNotifyViewToMode(view: AdminView): H4WechatNotifyMode | null {
  if (view === "h4-wechat-settings") return "settings";
  if (view === "h4-notify-configs") return "configs";
  if (view === "h4-notify-records") return "records";
  return null;
}
