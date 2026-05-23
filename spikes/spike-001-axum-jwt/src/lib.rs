//! SPIKE-001 — Axum + JWT + 多租户 middleware
//!
//! 验证：
//! - H1: < 50 行 middleware 实现 JWT 验签 + claim 提取
//! - H2: AuthContext extractor 让 handler 直接拿到 user_id / owner_id / permissions
//! - H3: in-memory blacklist 实现 token 撤销
//! - H4: 多租户 owner_id 自动隔离（middleware 注入 + handler 用 ctx.owner_id 过滤业务数据）
//!
//! 不在范围（spike 阶段）：
//! - 真 PostgreSQL（用 HashMap 模拟业务表）
//! - 真 Redis blacklist（用 RwLock<HashSet> 单机版本，验证假设足够）
//! - 真 refresh token rotation（H5 用 markdown 状态机文档化）
//! - 速率限制 / IP 黑名单 / 暴力破解防护（生产化时 Wave 1 实现）

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use axum::{
    extract::{FromRef, FromRequestParts, Path, State},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================
// JWT Claims
// ============================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// JWT 标准字段：subject = user_id（UUID）
    pub sub: String,
    /// JWT 标准字段：过期时间（Unix 秒）
    pub exp: usize,
    /// JWT 标准字段：jti = token unique ID（用于 blacklist 撤销）
    pub jti: String,
    /// 自定义：所属货主（多租户隔离的核心字段）
    pub owner_id: String,
    /// 自定义：用户名（审计 actor 用）
    pub user_name: String,
    /// 自定义：权限码列表
    pub permissions: Vec<String>,
}

impl Claims {
    pub fn new(
        user_id: Uuid,
        user_name: String,
        owner_id: Uuid,
        permissions: Vec<String>,
        ttl: Duration,
    ) -> Self {
        let exp = (Utc::now() + ttl).timestamp() as usize;
        Self {
            sub: user_id.to_string(),
            exp,
            jti: Uuid::new_v4().to_string(),
            owner_id: owner_id.to_string(),
            user_name,
            permissions,
        }
    }
}

// ============================================================
// AuthContext extractor — handler 用此参数即得鉴权信息
// ============================================================

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub user_name: String,
    pub owner_id: Uuid,
    pub permissions: Vec<String>,
    pub jti: String,
}

impl AuthContext {
    pub fn has_permission(&self, code: &str) -> bool {
        self.permissions.iter().any(|p| p == code || p == "*")
    }
}

// ============================================================
// 应用状态
// ============================================================

#[derive(Clone)]
pub struct AppState {
    pub jwt_encoding: EncodingKey,
    pub jwt_decoding: DecodingKey,
    /// jti → token 过期时间（秒级，用于自动清理）
    pub blacklist: Arc<RwLock<HashSet<String>>>,
    /// 模拟用户表：user_name → (user_id, owner_id, password, permissions)
    pub users: Arc<HashMap<String, MockUser>>,
    /// 模拟业务表：item_id → (owner_id, code, name)；owner_id 用于隔离测试
    pub items: Arc<RwLock<HashMap<Uuid, MockItem>>>,
}

#[derive(Debug, Clone)]
pub struct MockUser {
    pub user_id: Uuid,
    pub owner_id: Uuid,
    pub password: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MockItem {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub code: String,
    pub name: String,
}

// ============================================================
// 错误类型
// ============================================================

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("缺少 Authorization 头")]
    MissingAuth,
    #[error("Authorization 头格式错（必须 Bearer xxx）")]
    InvalidAuthFormat,
    #[error("token 无效或已过期")]
    InvalidToken,
    #[error("token 已撤销（blacklist）")]
    TokenRevoked,
    #[error("权限不足：需要 {0}")]
    PermissionDenied(String),
    #[error("跨货主越权：尝试访问 owner={requested}，实际属于 owner={actual}")]
    OwnerMismatch {
        requested: String,
        actual: String,
    },
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status = match &self {
            AuthError::PermissionDenied(_) | AuthError::OwnerMismatch { .. } => StatusCode::FORBIDDEN,
            _ => StatusCode::UNAUTHORIZED,
        };
        let body = Json(serde_json::json!({
            "code": match &self {
                AuthError::MissingAuth => "AUTH-001",
                AuthError::InvalidAuthFormat => "AUTH-002",
                AuthError::InvalidToken => "AUTH-003",
                AuthError::TokenRevoked => "AUTH-004",
                AuthError::PermissionDenied(_) => "AUTH-005",
                AuthError::OwnerMismatch { .. } => "AUTH-006",
            },
            "message": self.to_string(),
        }));
        (status, body).into_response()
    }
}

// ============================================================
// FromRequestParts — extractor 实现
//
// H1+H2 验证点：本块代码量统计如下：
//   - struct AuthContext: 7 行
//   - impl FromRequestParts: 约 35 行（含 4 个失败路径）
//   合计 < 50 行，达成 H1 假设。
// ============================================================

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthContext
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        // 1. 取 Authorization 头
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .ok_or(AuthError::MissingAuth)?
            .to_str()
            .map_err(|_| AuthError::InvalidAuthFormat)?;

        // 2. 必须 Bearer 前缀
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AuthError::InvalidAuthFormat)?;

        // 3. 验签 + 校验过期
        let token_data = decode::<Claims>(token, &app_state.jwt_decoding, &Validation::default())
            .map_err(|_| AuthError::InvalidToken)?;
        let claims = token_data.claims;

        // 4. 检查 blacklist（jti 是否已撤销）
        if app_state
            .blacklist
            .read()
            .expect("blacklist lock poisoned")
            .contains(&claims.jti)
        {
            return Err(AuthError::TokenRevoked);
        }

        Ok(Self {
            user_id: Uuid::parse_str(&claims.sub).map_err(|_| AuthError::InvalidToken)?,
            user_name: claims.user_name,
            owner_id: Uuid::parse_str(&claims.owner_id).map_err(|_| AuthError::InvalidToken)?,
            permissions: claims.permissions,
            jti: claims.jti,
        })
    }
}

// ============================================================
// Handlers
// ============================================================

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub user_name: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub expires_in: i64,
    pub user_id: Uuid,
    pub owner_id: Uuid,
    pub permissions: Vec<String>,
}

pub async fn login_handler(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AuthError> {
    let user = state
        .users
        .get(&req.user_name)
        .ok_or(AuthError::InvalidToken)?
        .clone();

    if user.password != req.password {
        return Err(AuthError::InvalidToken);
    }

    let ttl = Duration::hours(1); // access token 1 小时（spike-005 决定生产期）
    let claims = Claims::new(
        user.user_id,
        req.user_name,
        user.owner_id,
        user.permissions.clone(),
        ttl,
    );
    let token = encode(&Header::default(), &claims, &state.jwt_encoding)
        .expect("JWT 编码失败（密钥/header 配置错）");

    Ok(Json(LoginResponse {
        access_token: token,
        expires_in: ttl.num_seconds(),
        user_id: user.user_id,
        owner_id: user.owner_id,
        permissions: user.permissions,
    }))
}

/// GET /me — 受保护，返回当前 AuthContext
pub async fn me_handler(ctx: AuthContext) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "user_id": ctx.user_id,
        "user_name": ctx.user_name,
        "owner_id": ctx.owner_id,
        "permissions": ctx.permissions,
    }))
}

/// GET /admin — 需要 admin 权限
pub async fn admin_handler(ctx: AuthContext) -> Result<Json<serde_json::Value>, AuthError> {
    if !ctx.has_permission("admin") {
        return Err(AuthError::PermissionDenied("admin".into()));
    }
    Ok(Json(serde_json::json!({
        "user": ctx.user_name,
        "message": "admin only",
    })))
}

/// POST /logout — 撤销当前 token（jti 加入 blacklist）
pub async fn logout_handler(
    State(state): State<AppState>,
    ctx: AuthContext,
) -> Json<serde_json::Value> {
    state
        .blacklist
        .write()
        .expect("blacklist lock poisoned")
        .insert(ctx.jti.clone());
    Json(serde_json::json!({ "revoked_jti": ctx.jti }))
}

/// GET /items — 列出当前 owner 的所有商品（多租户隔离）
pub async fn list_items_handler(
    State(state): State<AppState>,
    ctx: AuthContext,
) -> Json<Vec<MockItem>> {
    let items = state.items.read().expect("items lock poisoned");
    let visible: Vec<MockItem> = items
        .values()
        .filter(|i| i.owner_id == ctx.owner_id) // 关键隔离逻辑
        .cloned()
        .collect();
    Json(visible)
}

/// GET /items/:id — 单条查询（必须验证 owner_id 一致）
pub async fn get_item_handler(
    State(state): State<AppState>,
    ctx: AuthContext,
    Path(id): Path<Uuid>,
) -> Result<Json<MockItem>, AuthError> {
    let items = state.items.read().expect("items lock poisoned");
    let item = items.get(&id).cloned();
    match item {
        Some(i) if i.owner_id == ctx.owner_id => Ok(Json(i)),
        Some(i) => Err(AuthError::OwnerMismatch {
            requested: ctx.owner_id.to_string(),
            actual: i.owner_id.to_string(),
        }),
        None => Err(AuthError::InvalidToken), // spike 简化；生产应是 NotFound
    }
}

// ============================================================
// Router 构建器
// ============================================================

pub fn build_router(state: AppState) -> axum::Router {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/login", post(login_handler))
        .route("/me", get(me_handler))
        .route("/admin", get(admin_handler))
        .route("/logout", post(logout_handler))
        .route("/items", get(list_items_handler))
        .route("/items/:id", get(get_item_handler))
        .with_state(state)
}

// ============================================================
// 测试夹具
// ============================================================

pub fn build_test_state() -> AppState {
    let secret = b"spike-001-test-secret-do-not-use-in-prod";
    let owner_a = Uuid::from_u128(0xa);
    let owner_b = Uuid::from_u128(0xb);
    let alice_id = Uuid::from_u128(0x1a);
    let bob_id = Uuid::from_u128(0x2b);
    let carol_id = Uuid::from_u128(0x3c);

    let mut users = HashMap::new();
    users.insert(
        "alice".into(),
        MockUser {
            user_id: alice_id,
            owner_id: owner_a,
            password: "alice_pwd".into(),
            permissions: vec!["read".into(), "write".into(), "admin".into()],
        },
    );
    users.insert(
        "bob".into(),
        MockUser {
            user_id: bob_id,
            owner_id: owner_a,
            password: "bob_pwd".into(),
            permissions: vec!["read".into()],
        },
    );
    users.insert(
        "carol".into(),
        MockUser {
            user_id: carol_id,
            owner_id: owner_b,
            password: "carol_pwd".into(),
            permissions: vec!["read".into(), "write".into()],
        },
    );

    let mut items = HashMap::new();
    let item_a1 = Uuid::from_u128(0xa01);
    let item_a2 = Uuid::from_u128(0xa02);
    let item_b1 = Uuid::from_u128(0xb01);
    items.insert(
        item_a1,
        MockItem {
            id: item_a1,
            owner_id: owner_a,
            code: "P-A-001".into(),
            name: "葡萄糖注射液（货主 A）".into(),
        },
    );
    items.insert(
        item_a2,
        MockItem {
            id: item_a2,
            owner_id: owner_a,
            code: "P-A-002".into(),
            name: "氯化钠注射液（货主 A）".into(),
        },
    );
    items.insert(
        item_b1,
        MockItem {
            id: item_b1,
            owner_id: owner_b,
            code: "P-B-001".into(),
            name: "盐酸吗啡注射液（货主 B）".into(),
        },
    );

    AppState {
        jwt_encoding: EncodingKey::from_secret(secret),
        jwt_decoding: DecodingKey::from_secret(secret),
        blacklist: Arc::new(RwLock::new(HashSet::new())),
        users: Arc::new(users),
        items: Arc::new(RwLock::new(items)),
    }
}
