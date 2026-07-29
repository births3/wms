//! Admin menu shared model, validation, and tree helpers.

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;
use wms_domain::{
    AdminMenuButtonPermission, AdminMenuNode, AdminMenuVersion,
    UpsertAdminMenuButtonPermissionRequest,
};

use crate::auth::AuthContext;

const VALID_VIEW_IDS: &[&str] = &[
    "dashboard",
    "m1-products",
    "m1-business-partners",
    "m1-warehouses",
    "m1-zones",
    "m1-locations",
    "m1-system-dictionary",
    "dock-management",
    "m1-feature-flags",
    "m2-receiving",
    "m2-inbound-documents",
    "m2-inspecting",
    "m2-putaway",
    "m2-putaway-strategy",
    "m-di-platforms",
    "m-di-review",
    "m-di-stamp",
    "m3-status-config",
    "mte-task-types",
    "mte-task-groups",
    "mte-task-dispatch",
    "m9-billing-rules",
    "m10-route-plans",
    "m3-batches",
    "m3-counts",
    "m3-location-history",
    "m3-maintenance",
    "m3-relocations",
    "m4-orders",
    "m4-waves",
    "m4-review",
    "m4-returns",
    "h1-menu-management",
    "h1-role-permission",
    "h1-session-management",
    "h1-api-keys",
    "h2-audit-trail",
    "h3-api-contract",
    "h4-wechat-settings",
    "h4-notify-configs",
    "h4-notify-records",
    "h5-express",
    "h8-erp-connectors",
    "h8-erp-interface-tables",
    "h8-erp-messages",
    "h9-print-templates",
    "hal-alert-dashboard",
    "hal-alert-definitions",
    "hal-alert-escalations",
    "mcg-numbering",
];

const VALID_ICON_KEYS: &[&str] = &[
    "Activity",
    "ArrowUpCircle",
    "Bell",
    "BellRing",
    "BookOpen",
    "CheckCircle2",
    "ClipboardList",
    "Database",
    "History",
    "Inbox",
    "KeyRound",
    "Layers",
    "MapPinned",
    "PackageCheck",
    "PanelLeftOpen",
    "Printer",
    "ShieldCheck",
    "Settings",
    "Stamp",
    "Truck",
    "Users",
    "Warehouse",
];

const STANDARD_ACTION_KEYS: &[&str] = &[
    "acknowledge",
    "close",
    "create",
    "delete",
    "detail",
    "disable",
    "edit",
    "enable",
    "export",
    "field",
    "handle",
    "history",
    "ignore",
    "print",
    "query",
    "refresh",
    "replay",
    "reuse",
    "review",
    "submit",
    "summary",
    "upload",
    "view",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdminMenuError {
    NodeNotFound,
    VersionNotFound,
    InvalidTree,
    UnknownView,
    InvalidIcon,
    InvalidPermission,
    IdempotencyConflict,
    Audit(String),
    Database(String),
    Serialize(String),
}

#[derive(Clone, Debug, FromRow)]
pub(crate) struct MenuNodeRow {
    pub(crate) id: Uuid,
    pub(crate) parent_id: Option<Uuid>,
    pub(crate) level: i32,
    pub(crate) code: String,
    pub(crate) path: String,
    pub(crate) title: String,
    pub(crate) view_id: Option<String>,
    pub(crate) icon_key: String,
    pub(crate) permission_key: String,
    pub(crate) sort_order: i32,
    pub(crate) enabled: bool,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, FromRow)]
pub(crate) struct ButtonRow {
    pub(crate) menu_node_id: Uuid,
    action_key: String,
    action_label: String,
    action_kind: String,
    enabled: bool,
    sort_order: i32,
}

#[derive(Clone, Debug, FromRow)]
pub(crate) struct VersionRow {
    pub(crate) id: Uuid,
    pub(crate) version_no: i64,
    note: Option<String>,
    published_by: Uuid,
    published_at: DateTime<Utc>,
}

pub(crate) fn build_tree(nodes: Vec<MenuNodeRow>, buttons: Vec<ButtonRow>) -> Vec<AdminMenuNode> {
    let mut by_parent: BTreeMap<Option<Uuid>, Vec<MenuNodeRow>> = BTreeMap::new();
    let mut buttons_by_node: HashMap<Uuid, Vec<AdminMenuButtonPermission>> = HashMap::new();
    for button in buttons {
        buttons_by_node
            .entry(button.menu_node_id)
            .or_default()
            .push(button.into());
    }
    for node in nodes {
        by_parent.entry(node.parent_id).or_default().push(node);
    }
    build_children(None, &mut by_parent, &buttons_by_node)
}

pub(crate) fn filter_visible_tree(
    nodes: Vec<AdminMenuNode>,
    ctx: &AuthContext,
) -> Vec<AdminMenuNode> {
    nodes
        .into_iter()
        .filter_map(|mut node| {
            node.children = filter_visible_tree(node.children, ctx);
            if node.level == 3 && !ctx.has_permission(&node.permission_key) {
                return None;
            }
            if node.level < 3 && node.children.is_empty() {
                return None;
            }
            Some(node)
        })
        .collect()
}

pub(crate) fn code_segment(code: &str) -> Result<String, AdminMenuError> {
    let segment = code
        .rsplit('.')
        .next()
        .unwrap_or(code)
        .trim()
        .replace('_', "-");
    if segment.is_empty() {
        return Err(AdminMenuError::InvalidTree);
    }
    Ok(segment)
}

pub(crate) fn validate_node(
    level: i32,
    view_id: Option<&str>,
    icon_key: &str,
    permission_key: &str,
) -> Result<(), AdminMenuError> {
    if level == 3 {
        let Some(view_id) = view_id else {
            return Err(AdminMenuError::UnknownView);
        };
        if !VALID_VIEW_IDS.contains(&view_id) {
            return Err(AdminMenuError::UnknownView);
        }
    } else if view_id.is_some() {
        return Err(AdminMenuError::UnknownView);
    }
    if !VALID_ICON_KEYS.contains(&icon_key) {
        return Err(AdminMenuError::InvalidIcon);
    }
    if permission_key.trim().is_empty() {
        return Err(AdminMenuError::InvalidPermission);
    }
    Ok(())
}

pub(crate) fn validate_buttons(
    buttons: &[UpsertAdminMenuButtonPermissionRequest],
) -> Result<(), AdminMenuError> {
    let mut keys = HashSet::new();
    for button in buttons {
        if button.action_key.trim().is_empty()
            || button.action_label.trim().is_empty()
            || !keys.insert(button.action_key.clone())
        {
            return Err(AdminMenuError::InvalidPermission);
        }
        match button.action_kind.as_str() {
            "standard" if STANDARD_ACTION_KEYS.contains(&button.action_key.as_str()) => {}
            "private" => {}
            _ => return Err(AdminMenuError::InvalidPermission),
        }
    }
    Ok(())
}

pub(crate) fn map_db_error(error: sqlx::Error) -> AdminMenuError {
    AdminMenuError::Database(error.to_string())
}

fn build_children(
    parent_id: Option<Uuid>,
    by_parent: &mut BTreeMap<Option<Uuid>, Vec<MenuNodeRow>>,
    buttons_by_node: &HashMap<Uuid, Vec<AdminMenuButtonPermission>>,
) -> Vec<AdminMenuNode> {
    let mut nodes = by_parent.remove(&parent_id).unwrap_or_default();
    nodes.sort_by(|left, right| {
        left.sort_order
            .cmp(&right.sort_order)
            .then_with(|| left.title.cmp(&right.title))
    });
    nodes
        .into_iter()
        .map(|row| {
            let id = row.id;
            let children = build_children(Some(id), by_parent, buttons_by_node);
            row.into_node(
                buttons_by_node.get(&id).cloned().unwrap_or_default(),
                children,
            )
        })
        .collect()
}

impl MenuNodeRow {
    pub(crate) fn into_node(
        self,
        button_permissions: Vec<AdminMenuButtonPermission>,
        children: Vec<AdminMenuNode>,
    ) -> AdminMenuNode {
        AdminMenuNode {
            id: self.id,
            parent_id: self.parent_id,
            level: self.level,
            code: self.code,
            path: self.path,
            title: self.title,
            view_id: self.view_id,
            icon_key: self.icon_key,
            permission_key: self.permission_key,
            sort_order: self.sort_order,
            enabled: self.enabled,
            button_permissions,
            children,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

impl From<ButtonRow> for AdminMenuButtonPermission {
    fn from(row: ButtonRow) -> Self {
        Self {
            action_key: row.action_key,
            action_label: row.action_label,
            action_kind: row.action_kind,
            enabled: row.enabled,
            sort_order: row.sort_order,
        }
    }
}

impl From<VersionRow> for AdminMenuVersion {
    fn from(row: VersionRow) -> Self {
        Self {
            id: row.id,
            version_no: row.version_no,
            note: row.note,
            published_by: row.published_by,
            published_at: row.published_at,
        }
    }
}
