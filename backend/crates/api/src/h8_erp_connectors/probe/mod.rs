//! AC7 连接测试探测（不写业务单据）。

mod interface;
mod rest;

#[cfg(test)]
mod tests;

use interface::interface_table_probe;
use rest::rest_url_probe;
use wms_domain::H8ErpConnector;

pub(crate) async fn run_connection_probe(connector: &H8ErpConnector) -> (bool, Option<String>) {
    match connector.channel_mode.as_str() {
        "rest" | "rest_primary_table_fallback" => {
            let Some(url) = connector
                .api_base_url
                .as_deref()
                .map(str::trim)
                .filter(|url| !url.is_empty())
            else {
                return (false, Some("api_base_url missing".into()));
            };
            if connector
                .directions
                .iter()
                .any(|direction| direction == "inbound")
                && connector.api_key_id.is_none()
            {
                return (false, Some("api_key_id required for inbound REST".into()));
            }
            let bearer_token = if connector
                .directions
                .iter()
                .any(|direction| direction == "outbound")
            {
                match crate::secrets::resolve_secret_alias_for_probe(
                    connector.bearer_secret_alias.as_deref(),
                ) {
                    Ok(value) => Some(value),
                    Err(message) => return (false, Some(message)),
                }
            } else {
                None
            };
            if let Err(message) = rest_url_probe(url, bearer_token).await {
                return (false, Some(message));
            }
            if connector.channel_mode == "rest_primary_table_fallback" {
                if let Err(message) = interface_table_probe(connector).await {
                    return (false, Some(format!("fallback interface probe: {message}")));
                }
            }
            (true, None)
        }
        "interface_table" => match interface_table_probe(connector).await {
            Ok(()) => (true, None),
            Err(message) => (false, Some(message)),
        },
        _ => (false, Some("invalid channel_mode".into())),
    }
}
