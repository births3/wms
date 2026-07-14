use std::{
    collections::HashMap,
    env,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{ErrorResponse, ResilienceStatus};

use crate::{
    audit::{append_event, AuditDiff, AuditWriteRequest},
    auth::{decode_auth_context, AuthContext, JWT_SECRET_ENV},
};

const DEFAULT_GLOBAL_QPS: u32 = 1000;
const DEFAULT_GLOBAL_BURST: u32 = 1000;
const DEFAULT_USER_QPS: u32 = 100;
const DEFAULT_USER_BURST: u32 = 100;
const DEFAULT_API_KEY_QPS: u32 = 100;
const DEFAULT_API_KEY_BURST: u32 = 100;
const DEFAULT_RETRY_AFTER_SECONDS: u64 = 1;
const DEFAULT_CIRCUIT_FAILURES: u32 = 5;
const DEFAULT_CIRCUIT_OPEN_SECONDS: u64 = 30;
const API_KEY_HEADER: &str = "x-wms-api-key";
const INTERNAL_SERVICE_HEADER: &str = "x-wms-internal-service";
const REQUEST_ID_HEADER: &str = "x-request-id";

#[derive(Clone, Debug)]
pub struct ResilienceConfig {
    pub global_qps: u32,
    pub global_burst: u32,
    pub user_qps: u32,
    pub user_burst: u32,
    pub api_key_qps: u32,
    pub api_key_burst: u32,
    pub retry_after_seconds: u64,
    pub circuit_failures: u32,
    pub circuit_open_seconds: u64,
}

#[derive(Clone)]
pub struct ResilienceState {
    config: ResilienceConfig,
    inner: Arc<Mutex<ResilienceInner>>,
    audit_pool: Option<PgPool>,
}

#[derive(Debug)]
struct ResilienceInner {
    global: TokenBucket,
    users: HashMap<String, TokenBucket>,
    api_keys: HashMap<String, TokenBucket>,
    rate_limit_rejected_total: u64,
    circuit_open_until: Option<Instant>,
    circuit_half_open: bool,
    circuit_opened_total: u64,
    degraded_responses_total: u64,
    consecutive_failures: u32,
}

#[derive(Debug)]
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
}

#[derive(Clone, Debug)]
enum CallerIdentity {
    User {
        user_id: Uuid,
        owner_id: Uuid,
        actor_name: String,
        jti: String,
    },
    ApiKey {
        key_hash: String,
        key_id: Option<Uuid>,
        owner_id: Option<Uuid>,
        actor_name: Option<String>,
        jti: Option<String>,
    },
    Internal,
    Anonymous,
}

enum ResilienceRejection {
    RateLimited(u64),
    CircuitOpen(u64),
}

enum ResilienceAuditKind {
    RateLimited,
    CircuitOpened,
    CircuitDegraded,
}

struct MetricsSnapshot {
    rate_limit_rejected_total: u64,
    circuit_opened_total: u64,
    degraded_responses_total: u64,
}

impl ResilienceState {
    pub fn from_env() -> Self {
        Self::new(ResilienceConfig {
            global_qps: env_u32("WMS_API_GLOBAL_QPS", DEFAULT_GLOBAL_QPS),
            global_burst: env_u32("WMS_API_GLOBAL_BURST", DEFAULT_GLOBAL_BURST),
            user_qps: env_u32("WMS_API_USER_QPS", DEFAULT_USER_QPS),
            user_burst: env_u32("WMS_API_USER_BURST", DEFAULT_USER_BURST),
            api_key_qps: env_u32("WMS_API_KEY_QPS", DEFAULT_API_KEY_QPS),
            api_key_burst: env_u32("WMS_API_KEY_BURST", DEFAULT_API_KEY_BURST),
            retry_after_seconds: u64::from(env_u32(
                "WMS_API_RETRY_AFTER_SECONDS",
                DEFAULT_RETRY_AFTER_SECONDS as u32,
            )),
            circuit_failures: env_u32("WMS_API_CIRCUIT_FAILURES", DEFAULT_CIRCUIT_FAILURES),
            circuit_open_seconds: u64::from(env_u32(
                "WMS_API_CIRCUIT_OPEN_SECONDS",
                DEFAULT_CIRCUIT_OPEN_SECONDS as u32,
            )),
        })
    }

    pub fn new_for_test(
        global_qps: u32,
        global_burst: u32,
        circuit_failures: u32,
        circuit_open_seconds: u64,
    ) -> Self {
        Self::new(ResilienceConfig {
            global_qps,
            global_burst,
            user_qps: DEFAULT_USER_QPS,
            user_burst: DEFAULT_USER_BURST,
            api_key_qps: DEFAULT_API_KEY_QPS,
            api_key_burst: DEFAULT_API_KEY_BURST,
            retry_after_seconds: DEFAULT_RETRY_AFTER_SECONDS,
            circuit_failures,
            circuit_open_seconds,
        })
    }

    pub fn new(config: ResilienceConfig) -> Self {
        let now = Instant::now();
        Self {
            config: config.clone(),
            inner: Arc::new(Mutex::new(ResilienceInner {
                global: TokenBucket::new(config.global_burst, now),
                users: HashMap::new(),
                api_keys: HashMap::new(),
                rate_limit_rejected_total: 0,
                circuit_open_until: None,
                circuit_half_open: false,
                circuit_opened_total: 0,
                degraded_responses_total: 0,
                consecutive_failures: 0,
            })),
            audit_pool: None,
        }
    }

    pub fn with_audit_pool(mut self, pool: PgPool) -> Self {
        self.audit_pool = Some(pool);
        self
    }

    pub fn status(&self) -> ResilienceStatus {
        let now = Instant::now();
        let mut inner = self.lock_inner();
        inner
            .global
            .refill(self.config.global_qps, self.config.global_burst, now);
        if inner.circuit_open_until.is_some_and(|until| until <= now) {
            inner.circuit_open_until = None;
            inner.circuit_half_open = true;
        }
        let open_remaining = inner
            .circuit_open_until
            .and_then(|until| until.checked_duration_since(now))
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let circuit_state = if open_remaining > 0 {
            "open"
        } else if inner.circuit_half_open {
            "half_open"
        } else {
            "closed"
        };
        ResilienceStatus {
            rate_limit_capacity: self.config.global_burst,
            rate_limit_available: inner.global.tokens.floor() as u32,
            rate_limit_rejected_total: inner.rate_limit_rejected_total,
            circuit_state: circuit_state.to_string(),
            circuit_open_remaining_seconds: open_remaining,
            circuit_opened_total: inner.circuit_opened_total,
            consecutive_failures: inner.consecutive_failures,
        }
    }

    pub fn metrics_text(&self) -> String {
        let metrics = self.metrics_snapshot();
        format!(
            "# TYPE wms_h3_rate_limit_rejected_total counter\n\
             wms_h3_rate_limit_rejected_total {}\n\
             # TYPE wms_h3_circuit_opened_total counter\n\
             wms_h3_circuit_opened_total {}\n\
             # TYPE wms_h3_degraded_responses_total counter\n\
             wms_h3_degraded_responses_total {}\n",
            metrics.rate_limit_rejected_total,
            metrics.circuit_opened_total,
            metrics.degraded_responses_total
        )
    }

    pub fn reset_rate_limit_for_test(&self) {
        let now = Instant::now();
        let mut inner = self.lock_inner();
        inner.global = TokenBucket::new(self.config.global_burst, now);
        inner.users.clear();
        inner.api_keys.clear();
    }

    fn metrics_snapshot(&self) -> MetricsSnapshot {
        let inner = self.lock_inner();
        MetricsSnapshot {
            rate_limit_rejected_total: inner.rate_limit_rejected_total,
            circuit_opened_total: inner.circuit_opened_total,
            degraded_responses_total: inner.degraded_responses_total,
        }
    }

    fn check_request(&self, caller: &CallerIdentity) -> Result<(), ResilienceRejection> {
        let now = Instant::now();
        let mut inner = self.lock_inner();

        if let Some(until) = inner.circuit_open_until {
            if let Some(remaining) = until.checked_duration_since(now) {
                inner.degraded_responses_total += 1;
                return Err(ResilienceRejection::CircuitOpen(remaining.as_secs().max(1)));
            }
            inner.circuit_open_until = None;
            inner.circuit_half_open = true;
        }

        if !inner
            .global
            .consume(self.config.global_qps, self.config.global_burst, now)
        {
            inner.rate_limit_rejected_total += 1;
            return Err(ResilienceRejection::RateLimited(
                self.config.retry_after_seconds.max(1),
            ));
        }

        match caller {
            CallerIdentity::User { user_id, .. } => {
                let bucket = inner
                    .users
                    .entry(user_id.to_string())
                    .or_insert_with(|| TokenBucket::new(self.config.user_burst, now));
                let allowed = bucket.consume(self.config.user_qps, self.config.user_burst, now);
                if !allowed {
                    inner.global.refund(self.config.global_burst);
                    inner.rate_limit_rejected_total += 1;
                    return Err(ResilienceRejection::RateLimited(
                        self.config.retry_after_seconds.max(1),
                    ));
                }
            }
            CallerIdentity::ApiKey { key_hash, .. } => {
                let bucket = inner
                    .api_keys
                    .entry(key_hash.clone())
                    .or_insert_with(|| TokenBucket::new(self.config.api_key_burst, now));
                let allowed =
                    bucket.consume(self.config.api_key_qps, self.config.api_key_burst, now);
                if !allowed {
                    inner.global.refund(self.config.global_burst);
                    inner.rate_limit_rejected_total += 1;
                    return Err(ResilienceRejection::RateLimited(
                        self.config.retry_after_seconds.max(1),
                    ));
                }
            }
            CallerIdentity::Internal | CallerIdentity::Anonymous => {}
        }

        Ok(())
    }

    fn record_response(&self, status: StatusCode) -> Option<ResilienceAuditKind> {
        let mut inner = self.lock_inner();
        if status.is_server_error() {
            inner.consecutive_failures += 1;
            if inner.circuit_half_open
                || inner.consecutive_failures >= self.config.circuit_failures.max(1)
            {
                inner.circuit_open_until =
                    Some(Instant::now() + Duration::from_secs(self.config.circuit_open_seconds));
                inner.circuit_half_open = false;
                inner.circuit_opened_total += 1;
                inner.consecutive_failures = 0;
                return Some(ResilienceAuditKind::CircuitOpened);
            }
            return None;
        }
        if status.is_success() {
            inner.consecutive_failures = 0;
            inner.circuit_half_open = false;
        }
        None
    }

    async fn write_audit(
        &self,
        caller: &CallerIdentity,
        method: &str,
        path: &str,
        kind: ResilienceAuditKind,
        status: StatusCode,
        ip: Option<&str>,
        user_agent: Option<&str>,
        request_id: Option<Uuid>,
    ) {
        let Some(pool) = &self.audit_pool else {
            return;
        };
        let Some(req) = audit_request(
            caller, method, path, kind, status, ip, user_agent, request_id,
        ) else {
            return;
        };
        if let Err(error) = append_event(pool, &req).await {
            tracing::error!(?error, path, "H3 resilience audit write failed");
        }
    }

    fn lock_inner(&self) -> std::sync::MutexGuard<'_, ResilienceInner> {
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

impl TokenBucket {
    fn new(burst: u32, now: Instant) -> Self {
        Self {
            tokens: f64::from(burst.max(1)),
            last_refill: now,
        }
    }

    fn consume(&mut self, qps: u32, burst: u32, now: Instant) -> bool {
        self.refill(qps, burst, now);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            return true;
        }
        false
    }

    fn refund(&mut self, burst: u32) {
        let capacity = f64::from(burst.max(1));
        self.tokens = (self.tokens + 1.0).min(capacity);
    }

    fn refill(&mut self, qps: u32, burst: u32, now: Instant) {
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        if elapsed <= 0.0 {
            return;
        }
        let capacity = f64::from(burst.max(1));
        self.tokens = (self.tokens + elapsed * f64::from(qps.max(1))).min(capacity);
        self.last_refill = now;
    }
}

pub async fn resilience_middleware(
    State(state): State<ResilienceState>,
    request: Request,
    next: Next,
) -> Response {
    if is_exempt_path(request.uri().path()) {
        return next.run(request).await;
    }

    let path = request.uri().path().to_string();
    let method = request.method().to_string();
    let ip = client_ip(request.headers());
    let user_agent = request
        .headers()
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let request_id = request_id(request.headers());
    let caller = caller_identity(&request);
    if let Err(rejection) = state.check_request(&caller) {
        let kind = match rejection {
            ResilienceRejection::RateLimited(_) => ResilienceAuditKind::RateLimited,
            ResilienceRejection::CircuitOpen(_) => ResilienceAuditKind::CircuitDegraded,
        };
        let response = rejection_response(rejection, request_id);
        state
            .write_audit(
                &caller,
                &method,
                &path,
                kind,
                response.status(),
                ip.as_deref(),
                user_agent.as_deref(),
                request_id,
            )
            .await;
        return response;
    }

    let response = next.run(request).await;
    if let Some(kind) = state.record_response(response.status()) {
        state
            .write_audit(
                &caller,
                &method,
                &path,
                kind,
                response.status(),
                ip.as_deref(),
                user_agent.as_deref(),
                request_id,
            )
            .await;
    }
    response
}

pub async fn resilience_status(State(state): State<ResilienceState>) -> Json<ResilienceStatus> {
    Json(state.status())
}

pub async fn resilience_metrics(State(state): State<ResilienceState>) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        state.metrics_text(),
    )
        .into_response()
}

fn caller_identity(request: &Request) -> CallerIdentity {
    let headers = request.headers();
    if headers
        .get(INTERNAL_SERVICE_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| matches!(value, "1" | "true" | "TRUE"))
    {
        return CallerIdentity::Internal;
    }

    if let Some(value) = headers
        .get(API_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let key_hash = short_sha256(value);
        if let Some(context) = request.extensions().get::<AuthContext>() {
            return CallerIdentity::ApiKey {
                key_hash,
                key_id: Some(context.user_id),
                owner_id: Some(context.owner_id),
                actor_name: Some(context.actor_name.clone()),
                jti: Some(context.jti.clone()),
            };
        }
        return CallerIdentity::ApiKey {
            key_hash,
            key_id: None,
            owner_id: None,
            actor_name: None,
            jti: None,
        };
    }

    let Some(token) = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
    else {
        return CallerIdentity::Anonymous;
    };
    let Ok(secret) = env::var(JWT_SECRET_ENV) else {
        return CallerIdentity::Anonymous;
    };
    match decode_auth_context(token, &secret) {
        Ok(ctx) => CallerIdentity::User {
            user_id: ctx.user_id,
            owner_id: ctx.owner_id,
            actor_name: ctx.actor_name,
            jti: ctx.jti,
        },
        Err(_) => CallerIdentity::Anonymous,
    }
}

fn audit_request(
    caller: &CallerIdentity,
    method: &str,
    path: &str,
    kind: ResilienceAuditKind,
    status: StatusCode,
    ip: Option<&str>,
    user_agent: Option<&str>,
    request_id: Option<Uuid>,
) -> Option<AuditWriteRequest> {
    let source = match caller {
        CallerIdentity::User { .. } => "bearer",
        CallerIdentity::ApiKey { .. } => "api_key",
        CallerIdentity::Internal | CallerIdentity::Anonymous => return None,
    };
    let (actor_id, actor_name, owner_id, jti) = match caller {
        CallerIdentity::User {
            user_id,
            owner_id,
            actor_name,
            jti,
        } => (*user_id, actor_name.clone(), *owner_id, jti.clone()),
        CallerIdentity::ApiKey {
            key_hash,
            key_id,
            owner_id,
            actor_name,
            jti,
        } => (
            key_id.unwrap_or_else(Uuid::nil),
            actor_name
                .clone()
                .unwrap_or_else(|| format!("api-key:{key_hash}")),
            (*owner_id)?,
            jti.clone().unwrap_or_else(|| format!("api-key:{key_hash}")),
        ),
        CallerIdentity::Internal | CallerIdentity::Anonymous => return None,
    };
    let (action, event) = match kind {
        ResilienceAuditKind::RateLimited => ("h3.rate_limited", "rate_limited"),
        ResilienceAuditKind::CircuitOpened => ("h3.circuit_opened", "circuit_opened"),
        ResilienceAuditKind::CircuitDegraded => ("h3.circuit_degraded", "circuit_degraded"),
    };
    Some(AuditWriteRequest {
        occurred_at: Utc::now(),
        actor_id,
        actor_name,
        owner_id,
        jti,
        action: action.to_string(),
        module: "H3".to_string(),
        resource_type: "api_resilience".to_string(),
        resource_id: path.to_string(),
        diff: Some(AuditDiff::compute(
            serde_json::json!({}),
            serde_json::json!({
                "event": event,
                "method": method,
                "path": path,
                "status_code": status.as_u16(),
                "source": source,
            }),
        )),
        request_id,
        ip: ip.map(str::to_string),
        user_agent: user_agent.map(str::to_string),
    })
}

fn request_id(headers: &axum::http::HeaderMap) -> Option<Uuid> {
    headers
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value.trim()).ok())
}

fn client_ip(headers: &axum::http::HeaderMap) -> Option<String> {
    ["x-forwarded-for", "x-real-ip"].iter().find_map(|name| {
        headers
            .get(*name)
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.split(',').next().unwrap_or(value).trim().to_string())
    })
}

fn short_sha256(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize()).chars().take(16).collect()
}

fn rejection_response(rejection: ResilienceRejection, request_id: Option<Uuid>) -> Response {
    let (status, code, message, retry_after, circuit_state, degraded) = match rejection {
        ResilienceRejection::RateLimited(seconds) => (
            StatusCode::TOO_MANY_REQUESTS,
            "H3_RATE_LIMITED",
            "API 请求超过限流阈值",
            Some(seconds),
            None,
            false,
        ),
        ResilienceRejection::CircuitOpen(seconds) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "H3_CIRCUIT_OPEN",
            "API 熔断保护已打开，返回降级响应",
            Some(seconds),
            Some("open"),
            true,
        ),
    };
    let mut response = (
        status,
        Json(ErrorResponse {
            code: code.to_string(),
            message: message.to_string(),
            severity: "error".to_string(),
            details: serde_json::json!({
                "degraded": degraded,
                "data_may_be_stale": degraded,
            }),
            trace_id: request_id
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unavailable".to_string()),
            retry_hint: retry_after.map(|seconds| format!("{seconds}s 后重试")),
        }),
    )
        .into_response();
    if let Some(seconds) = retry_after {
        if let Ok(value) = HeaderValue::from_str(&seconds.to_string()) {
            response.headers_mut().insert(header::RETRY_AFTER, value);
        }
    }
    if let Some(state) = circuit_state {
        response
            .headers_mut()
            .insert("x-wms-circuit-state", HeaderValue::from_static(state));
    }
    if degraded {
        response
            .headers_mut()
            .insert("x-wms-degraded-response", HeaderValue::from_static("true"));
    }
    if let Some(request_id) = request_id {
        if let Ok(value) = HeaderValue::from_str(&request_id.to_string()) {
            response.headers_mut().insert(REQUEST_ID_HEADER, value);
        }
    }
    response
}

fn is_exempt_path(path: &str) -> bool {
    matches!(
        path,
        "/healthz"
            | "/readyz"
            | "/api/v1/healthz"
            | "/openapi.json"
            | "/api-docs"
            | "/redoc"
            | "/metrics"
            | "/api/v1/resilience/status"
    )
}

fn env_u32(name: &str, default: u32) -> u32 {
    env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::{audit_request, request_id, CallerIdentity, ResilienceAuditKind};
    use axum::http::{HeaderMap, HeaderValue, StatusCode};
    use uuid::Uuid;

    #[test]
    fn api_key_audit_requires_bound_owner_context() {
        let caller = CallerIdentity::ApiKey {
            key_hash: "key-hash".to_string(),
            key_id: None,
            owner_id: None,
            actor_name: None,
            jti: None,
        };
        assert!(audit_request(
            &caller,
            "GET",
            "/dependency",
            ResilienceAuditKind::RateLimited,
            StatusCode::TOO_MANY_REQUESTS,
            None,
            None,
            None,
        )
        .is_none());
    }

    #[test]
    fn request_id_accepts_only_uuid_header_values() {
        let id = Uuid::new_v4();
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-request-id",
            HeaderValue::from_str(&id.to_string()).expect("uuid header should be valid"),
        );
        assert_eq!(request_id(&headers), Some(id));

        headers.insert("x-request-id", HeaderValue::from_static("not-a-uuid"));
        assert_eq!(request_id(&headers), None);
    }
}
