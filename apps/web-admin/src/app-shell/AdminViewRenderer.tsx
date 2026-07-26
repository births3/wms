import type { ReactNode } from "react";

import { FeatureFlagConfigCenterPage } from "@/pages/config-center/FeatureFlagConfigCenterPage";
import { ErpConnectorConfigPage } from "@/pages/config-center/ErpConnectorConfigPage";
import { ErpMessageLogPage } from "@/pages/config-center/ErpMessageLogPage";
import { ErpInterfaceTablePage } from "@/pages/config-center/ErpInterfaceTablePage";
import { H1AdminMenuPage } from "@/pages/admin-menu/H1AdminMenuPage";
import { H1RolePermissionPage } from "@/pages/auth/H1RolePermissionPage";
import { H1SessionPage } from "@/pages/auth/H1SessionPage";
import { H1ApiKeyPage } from "@/pages/api-key/H1ApiKeyPage";
import { H4WechatNotifyPage, type H4WechatNotifyMode } from "@/pages/wechat-notify/H4WechatNotifyPage";
import { M2InboundPage, type M2InboundMode } from "@/pages/inbound/M2InboundPage";
import { M2PutawayStrategyPage } from "@/pages/inbound/M2PutawayStrategyPage";
import { M3BatchManagementPage } from "@/pages/inventory/M3BatchManagementPage";
import { M3InventoryStatusConfigPage } from "@/pages/inventory/M3InventoryStatusConfigPage";
import { M3LocationHistoryPage } from "@/pages/inventory/M3LocationHistoryPage";
import { M3InventoryCountPage } from "@/pages/inventory/M3InventoryCountPage";
import { M3MaintenancePage } from "@/pages/inventory/M3MaintenancePage";
import { M3RelocationPage } from "@/pages/inventory/M3RelocationPage";
import { MrcReconciliationPage } from "@/pages/reconciliation/MrcReconciliationPage";
import { M1MasterDataPage, type MasterDataViewId } from "@/pages/master-data/M1MasterDataPage";
import { M4OutboundPage, type M4OutboundMode } from "@/pages/outbound/M4OutboundPage";
import { H2AuditTrailPage, H3ApiContractPage } from "@/pages/platform/HorizontalCapabilityPages";
import { H5ExpressPage } from "@/pages/express/H5ExpressPage";
import { AlertDefinitionPage } from "@/pages/alert-engine/AlertDefinitionPage";
import { AlertDashboardPage } from "@/pages/alert-engine/AlertDashboardPage";
import { AlertEscalationPage } from "@/pages/alert-engine/AlertEscalationPage";
import { H9PrintTemplatePage } from "@/pages/print-template/H9PrintTemplatePage";
import { MCGDocumentNumberingPage } from "@/pages/document-numbering/MCGDocumentNumberingPage";
import { DockManagementPage } from "@/pages/dock/DockManagementPage";
import { DrugInspectionPlatformPage } from "@/pages/drug-inspection/DrugInspectionPlatformPage";
import { TaskTypeConfigPage } from "@/pages/task-engine/TaskTypeConfigPage";
import { TaskGroupConfigPage } from "@/pages/task-engine/TaskGroupConfigPage";
import { TaskDispatchPage } from "@/pages/task-engine/TaskDispatchPage";
import { BillingRuleConfigPage } from "@/pages/billing/BillingRuleConfigPage";
import { TmsRoutePlanPage } from "@/pages/tms/TmsRoutePlanPage";
import type { CurrentUser } from "@/features/auth/auth-queries";
import type { AdminView } from "./admin-view";

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
