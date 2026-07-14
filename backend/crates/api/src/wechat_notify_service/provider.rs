use std::{future::Future, pin::Pin};

#[derive(Clone, Debug)]
pub struct WechatProviderRequest {
    pub corp_id: Option<String>,
    pub agent_id: Option<String>,
    pub secret_alias: Option<String>,
    pub recipient: String,
    pub content: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WechatProviderError {
    NotConfigured(String),
    Retryable(String),
    Permanent(String),
}

pub type WechatProviderFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(), WechatProviderError>> + Send + 'a>>;

pub trait WechatProvider: Send + Sync {
    fn send<'a>(&'a self, request: WechatProviderRequest) -> WechatProviderFuture<'a>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnconfiguredWechatProvider;

impl WechatProvider for UnconfiguredWechatProvider {
    fn send<'a>(&'a self, _request: WechatProviderRequest) -> WechatProviderFuture<'a> {
        Box::pin(async {
            Err(WechatProviderError::NotConfigured(
                "企业微信外部 provider 未接入".to_string(),
            ))
        })
    }
}
