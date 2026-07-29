use std::{fmt, time::Duration};

use serde_json::Value;

const DEFAULT_TIMEOUT_SECONDS: u64 = 35;
const MAX_PDF_BYTES: usize = 20 * 1024 * 1024;

#[derive(Clone)]
pub enum CategoryPdfRenderer {
    Disabled,
    Http {
        client: reqwest::Client,
        endpoint: reqwest::Url,
        token: String,
    },
    DeterministicTest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RenderWorkerError {
    InvalidConfiguration,
    Unavailable,
    InvalidResponse,
}

impl CategoryPdfRenderer {
    pub fn http(endpoint: &str, token: &str) -> Result<Self, RenderWorkerError> {
        if token.trim().is_empty() {
            return Err(RenderWorkerError::InvalidConfiguration);
        }
        let endpoint =
            reqwest::Url::parse(endpoint).map_err(|_| RenderWorkerError::InvalidConfiguration)?;
        if !matches!(endpoint.scheme(), "http" | "https") {
            return Err(RenderWorkerError::InvalidConfiguration);
        }
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECONDS))
            .build()
            .map_err(|_| RenderWorkerError::InvalidConfiguration)?;
        Ok(Self::Http {
            client,
            endpoint,
            token: token.to_string(),
        })
    }

    pub fn from_env() -> Self {
        let endpoint = std::env::var("WMS_H9_RENDER_WORKER_URL").ok();
        let token = std::env::var("WMS_H9_RENDER_TOKEN").ok();
        match (endpoint, token) {
            (Some(endpoint), Some(token)) => {
                Self::http(&endpoint, &token).unwrap_or(Self::Disabled)
            }
            _ => Self::Disabled,
        }
    }

    pub fn deterministic_for_tests() -> Self {
        Self::DeterministicTest
    }

    pub async fn render(
        &self,
        template: &Value,
        data: &Value,
    ) -> Result<Vec<u8>, RenderWorkerError> {
        match self {
            Self::Disabled => Err(RenderWorkerError::Unavailable),
            Self::DeterministicTest => {
                let source = serde_json::to_string(&(template, data))
                    .map_err(|_| RenderWorkerError::InvalidResponse)?;
                Ok(crate::pdf_document::render_text_pdf(&source))
            }
            Self::Http {
                client,
                endpoint,
                token,
            } => {
                let mut response = client
                    .post(endpoint.clone())
                    .bearer_auth(token)
                    .json(&serde_json::json!({"template": template, "data": data}))
                    .send()
                    .await
                    .map_err(|_| RenderWorkerError::Unavailable)?;
                if !response.status().is_success()
                    || response
                        .headers()
                        .get(reqwest::header::CONTENT_TYPE)
                        .and_then(|value| value.to_str().ok())
                        .is_none_or(|value| !value.starts_with("application/pdf"))
                    || response
                        .content_length()
                        .is_some_and(|length| length > MAX_PDF_BYTES as u64)
                {
                    return Err(RenderWorkerError::InvalidResponse);
                }
                let mut pdf = Vec::with_capacity(
                    response
                        .content_length()
                        .and_then(|length| usize::try_from(length).ok())
                        .unwrap_or(0),
                );
                while let Some(chunk) = response
                    .chunk()
                    .await
                    .map_err(|_| RenderWorkerError::InvalidResponse)?
                {
                    if pdf.len().saturating_add(chunk.len()) > MAX_PDF_BYTES {
                        return Err(RenderWorkerError::InvalidResponse);
                    }
                    pdf.extend_from_slice(&chunk);
                }
                if !pdf.starts_with(b"%PDF-") {
                    return Err(RenderWorkerError::InvalidResponse);
                }
                Ok(pdf)
            }
        }
    }
}

impl fmt::Debug for CategoryPdfRenderer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => formatter.write_str("CategoryPdfRenderer::Disabled"),
            Self::DeterministicTest => {
                formatter.write_str("CategoryPdfRenderer::DeterministicTest")
            }
            Self::Http { endpoint, .. } => formatter
                .debug_struct("CategoryPdfRenderer::Http")
                .field("endpoint", endpoint)
                .finish_non_exhaustive(),
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Bytes,
        extract::State,
        http::{header, HeaderMap, StatusCode},
        response::IntoResponse,
        routing::post,
        Json, Router,
    };
    use serde_json::{json, Value};
    use tokio::net::TcpListener;

    use crate::pdf_document::render_text_pdf;

    use super::*;

    #[tokio::test]
    async fn http_renderer_sends_frozen_template_and_business_data() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("render fixture should bind");
        let address = listener
            .local_addr()
            .expect("render fixture address should load");
        let expected = render_text_pdf("browser-worker-result");
        let app = Router::new()
            .route("/render", post(render_fixture))
            .with_state(expected.clone());
        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("render fixture should serve");
        });

        let renderer =
            CategoryPdfRenderer::http(&format!("http://{address}/render"), "rust-render-token")
                .expect("HTTP renderer should configure");
        let actual = renderer
            .render(
                &json!({"panels": [{"printElements": []}]}),
                &json!({"wms_order_no": "OUT-H9-009-HTTP"}),
            )
            .await
            .expect("HTTP renderer should return a PDF");
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn http_renderer_rejects_non_pdf_response() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("invalid render fixture should bind");
        let address = listener
            .local_addr()
            .expect("invalid render fixture address should load");
        let app = Router::new().route(
            "/render",
            post(|| async { ([(header::CONTENT_TYPE, "text/plain")], "not a PDF") }),
        );
        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("invalid render fixture should serve");
        });
        let renderer =
            CategoryPdfRenderer::http(&format!("http://{address}/render"), "rust-render-token")
                .expect("HTTP renderer should configure");

        assert_eq!(
            renderer
                .render(
                    &json!({"panels": [{"printElements": []}]}),
                    &json!({"wms_order_no": "OUT-H9-009-BAD"}),
                )
                .await,
            Err(RenderWorkerError::InvalidResponse)
        );
    }

    async fn render_fixture(
        State(pdf): State<Vec<u8>>,
        headers: HeaderMap,
        Json(payload): Json<Value>,
    ) -> impl IntoResponse {
        assert_eq!(
            headers
                .get(header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok()),
            Some("Bearer rust-render-token")
        );
        assert_eq!(
            payload
                .pointer("/data/wms_order_no")
                .and_then(Value::as_str),
            Some("OUT-H9-009-HTTP")
        );
        assert!(payload.pointer("/template/panels/0").is_some());
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/pdf")],
            Bytes::from(pdf),
        )
    }
}
