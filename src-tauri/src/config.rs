use crate::errors::{AppError, AppResult};
use crate::models::{AppConfig, SCHEMA_VERSION};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

const CONFIG_FILE: &str = "config.json";

fn config_dir() -> AppResult<PathBuf> {
    let proj = ProjectDirs::from("com", "sparkdns", "SparkDns")
        .ok_or_else(|| AppError::Config("Could not resolve config directory".to_string()))?;
    let dir = proj.config_dir().to_path_buf();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

fn config_path() -> AppResult<PathBuf> {
    Ok(config_dir()?.join(CONFIG_FILE))
}

/// Configuration is stored via the `directories` crate at a platform-specific
/// location that lives OUTSIDE the app install directory, so user data survives
/// reinstalls:
///   Windows : %APPDATA%/SparkDns/config/config.json
///   macOS   : ~/Library/Application Support/com.sparkdns.SparkDns/config.json
///   Linux   : ~/.config/SparkDns/config.json
///
/// The schema_version field in the JSON tracks the config format. When a newer
/// version is detected the migration logic below adds missing fields with safe
/// defaults while preserving all user data (custom profiles, history, rules,
/// auto-switch rules, and settings).
pub fn load_config() -> AppResult<AppConfig> {
    let path = config_path()?;
    if !path.exists() {
        let cfg = AppConfig::default();
        save_config(&cfg)?;
        return Ok(cfg);
    }
    let data = fs::read_to_string(&path)?;
    if data.trim().is_empty() {
        return Ok(AppConfig::default());
    }
    let mut cfg: AppConfig = serde_json::from_str(&data)?;
    migrate_config(&mut cfg)?;
    Ok(cfg)
}

/// Save config to disk atomically.
pub fn save_config(cfg: &AppConfig) -> AppResult<()> {
    let path = config_path()?;
    let data = serde_json::to_string_pretty(cfg)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, data)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

/// Reset config to defaults.
pub fn reset_config() -> AppResult<AppConfig> {
    let cfg = AppConfig::default();
    save_config(&cfg)?;
    Ok(cfg)
}

/// Apply forward migrations for older config schemas.
fn migrate_config(cfg: &mut AppConfig) -> AppResult<()> {
    if cfg.schema_version > SCHEMA_VERSION {
        return Err(AppError::Config(format!(
            "Config schema version {} is newer than supported {}",
            cfg.schema_version, SCHEMA_VERSION
        )));
    }

    // Ensure presets exist (merge by id).
    let preset_ids: Vec<String> = cfg.profiles.iter().map(|p| p.id.clone()).collect();
    let defaults = AppConfig::default();
    for preset in defaults.profiles {
        if !preset_ids.contains(&preset.id) {
            cfg.profiles.push(preset);
        }
    }

    if cfg.settings.diagnostic_targets.is_empty()
        || cfg.settings.diagnostic_targets.iter().any(|t| t.id == "google" || t.id == "quad9")
    {
        cfg.settings.diagnostic_targets = defaults.settings.diagnostic_targets;
    }

    cfg.schema_version = SCHEMA_VERSION;
    Ok(())
}

/// Export config as JSON string.
pub fn export_config_json(cfg: &AppConfig) -> AppResult<String> {
    Ok(serde_json::to_string_pretty(cfg)?)
}

/// Import config from JSON string, validating the schema version.
pub fn import_config_json(json: &str) -> AppResult<AppConfig> {
    if json.trim().is_empty() {
        return Err(AppError::Config("Import data is empty".to_string()));
    }
    let mut cfg: AppConfig = serde_json::from_str(json)?;
    migrate_config(&mut cfg)?;
    save_config(&cfg)?;
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AppSettings, DnsProfile, SCHEMA_VERSION};

    #[test]
    fn migrate_merges_missing_presets() {
        let mut cfg = AppConfig {
            profiles: vec![],
            settings: AppSettings {
                theme: "dark".into(),
                doh_enabled: false,
                dns_encryption_type: "doh".into(),
                ipv6_enabled: false,
                minimize_to_tray: false,
                start_on_boot: false,
                active_adapter_id: None,
                diagnostic_targets: vec![],
                auto_ping_enabled: true,
                auto_update: false,
            },
            schema_version: 0,
            ..AppConfig::default()
        };
        migrate_config(&mut cfg).unwrap();
        assert!(cfg.profiles.iter().any(|p| p.id == "preset_cloudflare"));
        assert!(!cfg.settings.diagnostic_targets.is_empty());
        assert_eq!(cfg.schema_version, SCHEMA_VERSION);
    }

    #[test]
    fn migrate_rejects_future_schema() {
        let mut cfg = AppConfig::default();
        cfg.schema_version = SCHEMA_VERSION + 1;
        assert!(migrate_config(&mut cfg).is_err());
    }

    #[test]
    fn migrate_preserves_existing_custom_profiles() {
        let custom = DnsProfile {
            id: "custom_test".into(),
            name: "Custom Test".into(),
            primary_ipv4: "1.2.3.4".into(),
            secondary_ipv4: None,
            primary_ipv6: None,
            secondary_ipv6: None,
            doh_url: None,
            dot_host: None,
            favorite: false,
            preset: false,
        };
        let mut cfg = AppConfig::default();
        cfg.profiles.clear();
        cfg.profiles.push(custom);
        migrate_config(&mut cfg).unwrap();
        assert!(cfg.profiles.iter().any(|p| p.id == "custom_test"));
        assert!(cfg.profiles.iter().any(|p| p.id == "preset_cloudflare"));
    }

    #[test]
    fn export_import_roundtrip() {
        let cfg = AppConfig::default();
        let json = export_config_json(&cfg).unwrap();
        assert!(!json.is_empty());
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.profiles.len(), cfg.profiles.len());
        assert_eq!(parsed.schema_version, cfg.schema_version);
    }

    #[test]
    fn import_rejects_empty() {
        assert!(import_config_json("").is_err());
        assert!(import_config_json("   ").is_err());
    }

    #[test]
    fn import_rejects_malformed_json() {
        assert!(import_config_json("{not valid json").is_err());
    }
}
