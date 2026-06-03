//! Wave 2 M1-008 config-center backed Feature Flag service.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;
use wms_domain::{
    ConfigEntry, FeatureFlagArchiveResult, FeatureFlagBatchImportResult, FeatureFlagConfig,
    FeatureFlagExportResponse, FeatureFlagMigrationResult, FeatureFlagReconcileReport,
    FeatureFlagSourceSwitchResponse,
};

use crate::{auth::AuthContext, feature_flags::FeatureFlagRegistry};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigCenterError {
    MissingFlag(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FeatureFlagSource {
    File,
    ConfigCenter,
}

#[derive(Clone, Debug)]
pub struct ConfigCenterStore {
    entries: BTreeMap<String, ConfigEntry>,
    feature_flags: BTreeMap<String, FeatureFlagConfig>,
    active_source: FeatureFlagSource,
}

impl Default for ConfigCenterStore {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
            feature_flags: BTreeMap::new(),
            active_source: FeatureFlagSource::File,
        }
    }
}

impl ConfigCenterStore {
    pub fn put_entry(
        &mut self,
        ctx: &AuthContext,
        key: impl Into<String>,
        value: serde_json::Value,
        now: DateTime<Utc>,
    ) -> ConfigEntry {
        let key = key.into();
        let version = self
            .entries
            .get(&key)
            .map(|entry| entry.version + 1)
            .unwrap_or(1);
        let entry = ConfigEntry {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            config_key: key.clone(),
            config_value: value,
            version,
            updated_at: now,
        };
        self.entries.insert(key, entry.clone());
        entry
    }

    pub fn migrate_feature_flags_from_file(
        &mut self,
        file_registry: &FeatureFlagRegistry,
    ) -> FeatureFlagMigrationResult {
        let mut migrated_count = 0;
        for flag in file_registry.flags() {
            self.feature_flags.insert(
                flag.key.clone(),
                FeatureFlagConfig {
                    key: flag.key.clone(),
                    owner: flag.owner.clone(),
                    created_at: flag.created_at.clone(),
                    cleanup_by: flag.cleanup_by.clone(),
                    enabled: flag.enabled,
                    source: "m1_config_center".to_string(),
                },
            );
            migrated_count += 1;
        }
        FeatureFlagMigrationResult {
            migrated_count,
            source: "deploy/feature_flags.toml".to_string(),
            target: "M1-008 config center".to_string(),
        }
    }

    pub fn import_feature_flags_batch(
        &mut self,
        flags: Vec<FeatureFlagConfig>,
    ) -> FeatureFlagBatchImportResult {
        let imported_count = flags.len() as u32;
        for mut flag in flags {
            flag.source = "m1_config_center".to_string();
            self.feature_flags.insert(flag.key.clone(), flag);
        }
        FeatureFlagBatchImportResult {
            imported_count,
            target: "M1-008 config center".to_string(),
        }
    }

    pub fn reconcile_feature_flags(
        &self,
        file_registry: &FeatureFlagRegistry,
    ) -> FeatureFlagReconcileReport {
        let mut matched = 0;
        let mut missing_in_config_center = Vec::new();
        let mut mismatched = Vec::new();

        for file_flag in file_registry.flags() {
            match self.feature_flags.get(&file_flag.key) {
                Some(config_flag)
                    if config_flag.owner == file_flag.owner
                        && config_flag.created_at == file_flag.created_at
                        && config_flag.cleanup_by == file_flag.cleanup_by
                        && config_flag.enabled == file_flag.enabled =>
                {
                    matched += 1;
                }
                Some(_) => mismatched.push(file_flag.key.clone()),
                None => missing_in_config_center.push(file_flag.key.clone()),
            }
        }

        FeatureFlagReconcileReport {
            matched,
            missing_in_config_center,
            mismatched,
        }
    }

    pub fn switch_feature_flag_source(
        &mut self,
        source: FeatureFlagSource,
    ) -> FeatureFlagSourceSwitchResponse {
        self.active_source = source;
        FeatureFlagSourceSwitchResponse {
            active_source: self.active_source_name().to_string(),
        }
    }

    pub fn is_feature_enabled(
        &self,
        key: &str,
        file_registry: &FeatureFlagRegistry,
    ) -> Result<bool, ConfigCenterError> {
        match self.active_source {
            FeatureFlagSource::File => Ok(file_registry.is_enabled(key)),
            FeatureFlagSource::ConfigCenter => self
                .feature_flags
                .get(key)
                .map(|flag| flag.enabled)
                .ok_or_else(|| ConfigCenterError::MissingFlag(key.to_string())),
        }
    }

    pub fn export_feature_flags(&self) -> FeatureFlagExportResponse {
        FeatureFlagExportResponse {
            source: self.active_source_name().to_string(),
            flags: self.feature_flags.values().cloned().collect(),
        }
    }

    pub fn archive_file_feature_flags(
        &self,
        archive_ref: impl Into<String>,
        archived_at: DateTime<Utc>,
    ) -> FeatureFlagArchiveResult {
        FeatureFlagArchiveResult {
            archived_source: "deploy/feature_flags.toml".to_string(),
            archive_ref: archive_ref.into(),
            archived_at,
        }
    }

    pub fn active_source_name(&self) -> &'static str {
        match self.active_source {
            FeatureFlagSource::File => "file",
            FeatureFlagSource::ConfigCenter => "config_center",
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use serde_json::json;
    use uuid::Uuid;

    use super::{ConfigCenterStore, FeatureFlagSource};
    use crate::{auth::AuthContext, feature_flags::FeatureFlagRegistry};

    fn ctx(owner_id: Uuid) -> AuthContext {
        AuthContext {
            user_id: Uuid::new_v4(),
            owner_id,
            actor_name: "tester".to_string(),
            permissions: vec!["m1.config.write".to_string()],
            jti: Uuid::new_v4().to_string(),
        }
    }

    #[test]
    fn migrates_reconciles_and_switches_feature_flags() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 12, 30, 0)
            .single()
            .expect("valid time");
        let file_registry = FeatureFlagRegistry::from_toml_str(
            r#"
            [[flags]]
            key = "w2_config_center_flags"
            owner = "platform"
            created_at = 2026-06-04
            cleanup_by = 2026-08-31
            enabled = true
            "#,
        )
        .expect("valid file registry");
        let mut store = ConfigCenterStore::default();

        let result = store.migrate_feature_flags_from_file(&file_registry);
        assert_eq!(result.migrated_count, 1);

        let imported = store.import_feature_flags_batch(vec![wms_domain::FeatureFlagConfig {
            key: "w2_bulk_imported_flag".to_string(),
            owner: "platform".to_string(),
            created_at: "2026-06-04".to_string(),
            cleanup_by: "2026-08-31".to_string(),
            enabled: false,
            source: "operator_upload".to_string(),
        }]);
        assert_eq!(imported.imported_count, 1);

        let report = store.reconcile_feature_flags(&file_registry);
        assert_eq!(report.matched, 1);
        assert!(report.missing_in_config_center.is_empty());
        assert!(report.mismatched.is_empty());

        let switched = store.switch_feature_flag_source(FeatureFlagSource::ConfigCenter);
        assert_eq!(switched.active_source, "config_center");
        assert!(store
            .is_feature_enabled("w2_config_center_flags", &file_registry)
            .expect("flag exists"));

        let exported = store.export_feature_flags();
        assert_eq!(exported.source, "config_center");
        assert_eq!(exported.flags.len(), 2);

        let archived =
            store.archive_file_feature_flags("s3://wms-dev/archive/feature_flags.toml", now);
        assert_eq!(archived.archived_source, "deploy/feature_flags.toml");
        assert_eq!(
            archived.archive_ref,
            "s3://wms-dev/archive/feature_flags.toml"
        );
    }

    #[test]
    fn config_entries_are_versioned() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 12, 0, 0)
            .single()
            .expect("valid time");
        let ctx = ctx(Uuid::new_v4());
        let mut store = ConfigCenterStore::default();

        let first = store.put_entry(&ctx, "m1.default_page_size", json!({"value": 50}), now);
        let second = store.put_entry(&ctx, "m1.default_page_size", json!({"value": 100}), now);

        assert_eq!(first.version, 1);
        assert_eq!(second.version, 2);
    }
}
