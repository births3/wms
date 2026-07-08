use std::{
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
use wms_domain::{ErrorResponse, ResilienceStatus};

const DEFAULT_GLOBAL_QPS: u32 = 1000;
const DEFAULT_GLOBAL_BURST: u32 = 1000;
const DEFAULT_RETRY_AFTER_SECONDS: u64 = 1;
const DEFAULT_CIRCUIT_FAILURES: u32 = 5;
const DEFAULT_CIRCUIT_OPEN_SECONDS: u64 = 30;

#[derive(Clone, Debug)]
pub struct ResilienceConfig {
    pub global_qps: u32,
    pub global_burst: u32,
    pub retry_after_seconds: u64,
    pub circuit_failures: u32,
    pub circuit_open_seconds: u64,
}

#[derive(Clone)]
pub struct ResilienceState {
    config: ResilienceConfig,
    inner: Arc<Mutex<ResilienceInner>>,
}

#[derive(Debug)]
struct ResilienceInner {
    tokens: f64,
    last_refill: Instant,
    rejected_total: u64,
    circuit_open_until: Option<Instant>,
    circuit_opened_total: u64,
    consecutive_failures: u32,
}

enum ResilienceRejection {
    RateLimited(u64),
    CircuitOpen(u64),
}

impl ResilienceState {
    pub fn from_env() -> Self {
        Self::new(ResilienceConfig {
            global_qps: env_u32("WMS_API_GLOBAL_QPS", DEFAULT_GLOBAL_QPS),
            global_burst: env_u32("WMS_API_GLOBAL_BURST", DEFAULT_GLOBAL_BURST),
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
            retry_after_seconds: DEFAULT_RETRY_AFTER_SECONDS,
            circuit_failures,
            circuit_open_seconds,
        })
    }

    pub fn new(config: ResilienceConfig) -> Self {
        let burst = config.global_burst.max(1);
        Self {
            config,
            inner: Arc::new(Mutex::new(ResilienceInner {
                tokens: f64::from(burst),
                last_refill: Instant::now(),
                rejected_total: 0,
                circuit_open_until: None,
                circuit_opened_total: 0,
                consecutive_failures: 0,
            })),
        }
    }

    pub fn status(&self) -> ResilienceStatus {
        let now = Instant::now();
        let mut inner = self.lock_inner();
        refill(&self.config, &mut inner, now);
        let open_remaining = inner
            .circuit_open_until
            .and_then(|until| until.checked_duration_since(now))
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        ResilienceStatus {
            rate_limit_capacity: self.config.global_burst,
            rate_limit_available: inner.tokens.floor() as u32,
            rate_limit_rejected_total: inner.rejected_total,
            circuit_state: if open_remaining > 0 { "open" } else { "closed" }.to_string(),
            circuit_open_remaining_seconds: open_remaining,
            circuit_opened_total: inner.circuit_opened_total,
            consecutive_failures: inner.consecutive_failures,
        }
    }

    fn check_request(&self) -> Result<(), ResilienceRejection> {
        let now = Instant::now();
        let mut inner = self.lock_inner();
        refill(&self.config, &mut inner, now);

        if let Some(until) = inner.circuit_open_until {
            if let Some(remaining) = until.checked_duration_since(now) {
                inner.rejected_total += 1;
                return Err(ResilienceRejection::CircuitOpen(remaining.as_secs().max(1)));
            }
            inner.circuit_open_until = None;
            inner.consecutive_failures = 0;
        }

        if inner.tokens >= 1.0 {
            inner.tokens -= 1.0;
            return Ok(());
        }

        inner.rejected_total += 1;
        Err(ResilienceRejection::RateLimited(
            self.config.retry_after_seconds.max(1),
        ))
    }

    fn record_response(&self, status: StatusCode) {
        let mut inner = self.lock_inner();
        if status.is_server_error() {
            inner.consecutive_failures += 1;
            if inner.consecutive_failures >= self.config.circuit_failures.max(1) {
                inner.circuit_open_until =
                    Some(Instant::now() + Duration::from_secs(self.config.circuit_open_seconds));
                inner.circuit_opened_total += 1;
                inner.consecutive_failures = 0;
            }
            return;
        }
        if status.is_success() {
            inner.consecutive_failures = 0;
        }
    }

    fn lock_inner(&self) -> std::sync::MutexGuard<'_, ResilienceInner> {
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
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

    if let Err(rejection) = state.check_request() {
        return rejection_response(rejection);
    }

    let response = next.run(request).await;
    state.record_response(response.status());
    response
}

pub async fn resilience_status(State(state): State<ResilienceState>) -> Json<ResilienceStatus> {
    Json(state.status())
}

fn refill(config: &ResilienceConfig, inner: &mut ResilienceInner, now: Instant) {
    let elapsed = now.duration_since(inner.last_refill).as_secs_f64();
    if elapsed <= 0.0 {
        return;
    }
    let capacity = f64::from(config.global_burst.max(1));
    inner.tokens = (inner.tokens + elapsed * f64::from(config.global_qps.max(1))).min(capacity);
    inner.last_refill = now;
}

fn rejection_response(rejection: ResilienceRejection) -> Response {
    let (status, code, message, retry_after, circuit_state) = match rejection {
        ResilienceRejection::RateLimited(seconds) => (
            StatusCode::TOO_MANY_REQUESTS,
            "H3_RATE_LIMITED",
            "API 请求超过限流阈值",
            Some(seconds),
            None,
        ),
        ResilienceRejection::CircuitOpen(seconds) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "H3_CIRCUIT_OPEN",
            "API 熔断保护已打开",
            Some(seconds),
            Some("open"),
        ),
    };
    let mut response = (
        status,
        Json(ErrorResponse {
            code: code.to_string(),
            message: message.to_string(),
            severity: "error".to_string(),
            details: serde_json::json!({}),
            trace_id: "unavailable".to_string(),
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
