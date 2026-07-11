//! Wave 1 file-backed Feature Flag runtime reader.
//!
//! This module intentionally keeps Wave 1 small: it reads the governance-owned
//! `deploy/feature_flags.toml` file and supports `WMS_FEATURE_*` environment
//! overrides. It does not introduce business flags.

use std::{collections::BTreeMap, env, fs, path::Path};

/// One registry entry from `deploy/feature_flags.toml`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeatureFlag {
    pub key: String,
    pub owner: String,
    pub created_at: String,
    pub cleanup_by: String,
    pub enabled: bool,
}

/// File-backed registry for Wave 1.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FeatureFlagRegistry {
    flags: BTreeMap<String, FeatureFlag>,
}

/// Runtime load/parse error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FeatureFlagError {
    ReadFailed(String),
    InvalidFormat(String),
}

impl FeatureFlagRegistry {
    pub fn empty() -> Self {
        Self {
            flags: BTreeMap::new(),
        }
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, FeatureFlagError> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .map_err(|e| FeatureFlagError::ReadFailed(format!("{}: {e}", path.display())))?;
        Self::from_toml_str(&text)
    }

    pub fn from_toml_str(text: &str) -> Result<Self, FeatureFlagError> {
        let mut registry = Self::empty();
        let mut current: Option<PartialFlag> = None;
        let mut in_flag_table = false;

        for raw_line in text.lines() {
            let line = strip_comment(raw_line).trim().to_string();
            if line.is_empty() {
                continue;
            }

            if line == "[[flags]]" {
                push_partial(&mut registry, current.take())?;
                current = Some(PartialFlag::default());
                in_flag_table = true;
                continue;
            }

            if line.starts_with('[') {
                push_partial(&mut registry, current.take())?;
                in_flag_table = false;
                continue;
            }

            if !in_flag_table {
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                return Err(FeatureFlagError::InvalidFormat(format!(
                    "invalid flag line: {line}"
                )));
            };
            let field = key.trim();
            let value = value.trim();
            let partial = current.get_or_insert_with(PartialFlag::default);

            match field {
                "key" => partial.key = Some(parse_string(value)?),
                "owner" => partial.owner = Some(parse_string(value)?),
                "created_at" => partial.created_at = Some(parse_string(value)?),
                "cleanup_by" => partial.cleanup_by = Some(parse_string(value)?),
                "enabled" => partial.enabled = Some(parse_bool(value)?),
                _ => {}
            }
        }

        push_partial(&mut registry, current)?;
        Ok(registry)
    }

    pub fn len(&self) -> usize {
        self.flags.len()
    }

    pub fn is_empty(&self) -> bool {
        self.flags.is_empty()
    }

    pub fn flags(&self) -> impl Iterator<Item = &FeatureFlag> {
        self.flags.values()
    }

    pub fn is_enabled(&self, key: &str) -> bool {
        self.is_enabled_with_env(key, |name| env::var(name).ok())
    }

    pub fn is_enabled_with_env<F>(&self, key: &str, env_lookup: F) -> bool
    where
        F: Fn(&str) -> Option<String>,
    {
        let env_name = env_key_name(key);
        if let Some(value) = env_lookup(&env_name) {
            match parse_bool(&value) {
                Ok(enabled) => return enabled,
                Err(_) => return false,
            }
        }

        match self.flags.get(key) {
            Some(flag) => flag.enabled,
            None => false,
        }
    }
}

#[derive(Default)]
struct PartialFlag {
    key: Option<String>,
    owner: Option<String>,
    created_at: Option<String>,
    cleanup_by: Option<String>,
    enabled: Option<bool>,
}

fn push_partial(
    registry: &mut FeatureFlagRegistry,
    partial: Option<PartialFlag>,
) -> Result<(), FeatureFlagError> {
    let Some(partial) = partial else {
        return Ok(());
    };

    let key = required(partial.key, "key")?;
    let flag = FeatureFlag {
        key: key.clone(),
        owner: required(partial.owner, "owner")?,
        created_at: required(partial.created_at, "created_at")?,
        cleanup_by: required(partial.cleanup_by, "cleanup_by")?,
        enabled: partial.enabled == Some(true),
    };
    registry.flags.insert(key, flag);
    Ok(())
}

fn required(value: Option<String>, field: &str) -> Result<String, FeatureFlagError> {
    match value {
        Some(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(FeatureFlagError::InvalidFormat(format!(
            "flag missing required field: {field}"
        ))),
    }
}

fn parse_string(value: &str) -> Result<String, FeatureFlagError> {
    let trimmed = value.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        return Ok(trimmed[1..trimmed.len() - 1].to_string());
    }
    if trimmed.is_empty() || trimmed.contains(char::is_whitespace) {
        return Err(FeatureFlagError::InvalidFormat(format!(
            "invalid string value: {trimmed}"
        )));
    }
    Ok(trimmed.to_string())
}

fn parse_bool(value: &str) -> Result<bool, FeatureFlagError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "on" | "yes" => Ok(true),
        "false" | "0" | "off" | "no" => Ok(false),
        other => Err(FeatureFlagError::InvalidFormat(format!(
            "invalid boolean value: {other}"
        ))),
    }
}

fn strip_comment(line: &str) -> &str {
    match line.split_once('#') {
        Some((before_comment, _)) => before_comment,
        None => line,
    }
}

fn env_key_name(key: &str) -> String {
    let suffix = key
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("WMS_FEATURE_{suffix}")
}

#[cfg(test)]
mod tests {
    use super::{FeatureFlagError, FeatureFlagRegistry};

    #[test]
    fn empty_registry_keeps_flags_disabled() {
        let registry = FeatureFlagRegistry::from_toml_str("flags = []").expect("valid registry");

        assert!(registry.is_empty());
        assert!(!registry.is_enabled("h1_auth_login_v1"));
    }

    #[test]
    fn parses_file_backed_flag_entries() {
        let registry = FeatureFlagRegistry::from_toml_str(
            r#"
            [[flags]]
            key = "h1_auth_login_v1"
            owner = "platform"
            created_at = 2026-06-02
            cleanup_by = 2026-08-31
            enabled = true
            "#,
        )
        .expect("valid registry");

        assert_eq!(registry.len(), 1);
        assert!(registry.is_enabled("h1_auth_login_v1"));
    }

    #[test]
    fn env_override_wins_over_file_value() {
        let registry = FeatureFlagRegistry::from_toml_str(
            r#"
            [[flags]]
            key = "m4_outbound_v2_picker"
            owner = "platform"
            created_at = 2026-06-02
            cleanup_by = 2026-08-31
            enabled = false
            "#,
        )
        .expect("valid registry");

        let enabled = registry.is_enabled_with_env("m4_outbound_v2_picker", |name| {
            if name == "WMS_FEATURE_M4_OUTBOUND_V2_PICKER" {
                Some("true".to_string())
            } else {
                None
            }
        });

        assert!(enabled);
    }

    #[test]
    fn malformed_env_override_fails_closed() {
        let registry = FeatureFlagRegistry::from_toml_str(
            r#"
            [[flags]]
            key = "m4_outbound_v2_picker"
            owner = "platform"
            created_at = 2026-06-02
            cleanup_by = 2026-08-31
            enabled = true
            "#,
        )
        .expect("valid registry");

        let enabled = registry.is_enabled_with_env("m4_outbound_v2_picker", |name| {
            if name == "WMS_FEATURE_M4_OUTBOUND_V2_PICKER" {
                Some("treu".to_string())
            } else {
                None
            }
        });

        assert!(!enabled);
    }

    #[test]
    fn invalid_entries_fail_closed() {
        let error = FeatureFlagRegistry::from_toml_str(
            r#"
            [[flags]]
            key = "h1_auth_login_v1"
            enabled = true
            "#,
        );

        assert!(matches!(
            error,
            Err(FeatureFlagError::InvalidFormat(message)) if message.contains("owner")
        ));
    }
}
