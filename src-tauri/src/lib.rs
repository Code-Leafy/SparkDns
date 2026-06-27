pub mod command_runner;
pub mod config;
pub mod diagnostics;
pub mod dns;
pub mod elevation;
pub mod errors;
pub mod models;
pub mod platform;
pub mod process_watcher;

pub mod validation;

use crate::errors::AppResult;
use crate::models::{
    AppConfig, AppRule, AppSettings, CommandResult, DnsApplyRequest, DnsProfile,
    NetworkAdapter, PlatformCapabilities, TracerouteResult,
};
use std::sync::Mutex;

/// Shared application state managed by Tauri.
pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub caps: PlatformCapabilities,
}

impl AppState {
    pub fn new() -> AppResult<Self> {
        let config = config::load_config()?;
        let caps = platform::detect_capabilities();
        Ok(Self {
            config: Mutex::new(config),
            caps,
        })
    }
}

// ---------------------------------------------------------------------------
// Tauri Commands
// ---------------------------------------------------------------------------

pub fn get_capabilities(state: tauri::State<'_, AppState>) -> PlatformCapabilities {
    state.caps.clone()
}

pub fn get_config(state: tauri::State<'_, AppState>) -> AppConfig {
    state.config.lock().unwrap().clone()
}

pub fn save_config(state: tauri::State<'_, AppState>, config: AppConfig) -> AppResult<()> {
    config::save_config(&config)?;
    *state.config.lock().unwrap() = config;
    Ok(())
}

pub fn reset_config(state: tauri::State<'_, AppState>) -> AppResult<AppConfig> {
    let cfg = config::reset_config()?;
    *state.config.lock().unwrap() = cfg.clone();
    Ok(cfg)
}

pub fn export_config(state: tauri::State<'_, AppState>) -> AppResult<String> {
    let cfg = state.config.lock().unwrap().clone();
    config::export_config_json(&cfg)
}

pub fn import_config(state: tauri::State<'_, AppState>, json: String) -> AppResult<AppConfig> {
    let cfg = config::import_config_json(&json)?;
    *state.config.lock().unwrap() = cfg.clone();
    Ok(cfg)
}

pub async fn list_adapters(state: tauri::State<'_, AppState>) -> AppResult<Vec<NetworkAdapter>> {
    dns::list_adapters(&state.caps).await
}

pub async fn apply_dns(
    state: tauri::State<'_, AppState>,
    req: DnsApplyRequest,
) -> AppResult<CommandResult> {
    dns::validate_apply_request(&req)?;
    dns::require_elevation(&state.caps)?;
    let result = dns::apply_dns(&req, &state.caps).await?;

    if result.ok {
        let mut cfg = state.config.lock().unwrap();
        cfg.active_profile_id = Some(req.profile_id.clone());
        cfg.is_connected = true;
        let _ = config::save_config(&cfg);
    }
    Ok(result)
}

pub async fn clear_dns(
    state: tauri::State<'_, AppState>,
    adapter_id: Option<String>,
) -> AppResult<CommandResult> {
    dns::require_elevation(&state.caps)?;
    let result = dns::clear_dns(adapter_id.as_deref(), &state.caps).await?;

    if result.ok {
        let mut cfg = state.config.lock().unwrap();
        cfg.active_profile_id = None;
        cfg.is_connected = false;
        let _ = config::save_config(&cfg);
    }
    Ok(result)
}

pub async fn flush_dns_cache(state: tauri::State<'_, AppState>) -> AppResult<CommandResult> {
    if !state.caps.supports_flush_dns {
        return Ok(CommandResult::err("Flushing DNS cache is not supported on this platform."));
    }
    match state.caps.os {
        models::OsKind::Windows => dns::windows::flush_cache().await,
        models::OsKind::Macos => dns::macos::flush_cache().await,
        models::OsKind::Linux => dns::linux::flush_cache(&state.caps.dns_backend).await,
    }
}

pub async fn reset_adapter(adapter_id: String) -> AppResult<CommandResult> {
    crate::validation::validate_host(&adapter_id)?;
    #[cfg(target_os = "windows")]
    {
        dns::windows::reset_adapter(&adapter_id).await
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(CommandResult::err("Adapter reset is only supported on Windows."))
    }
}

pub async fn renew_dhcp(
    state: tauri::State<'_, AppState>,
    adapter_id: Option<String>,
) -> AppResult<CommandResult> {
    if !state.caps.supports_dhcp_renew {
        return Ok(CommandResult::err("DHCP renewal is not supported on this platform."));
    }
    match state.caps.os {
        models::OsKind::Windows => dns::windows::renew_dhcp(adapter_id.as_deref()).await,
        models::OsKind::Macos => dns::macos::renew_dhcp(adapter_id.as_deref()).await,
        models::OsKind::Linux => {
            if crate::command_runner::which_exists("nmcli") {
                let conn = adapter_id.unwrap_or_default();
                if conn.is_empty() {
                    let out = crate::command_runner::run_async(
                        "nmcli",
                        &["-t", "-f", "UUID", "con", "show", "--active"],
                        10,
                    )
                    .await?;
                    let mut msgs = Vec::new();
                    for line in out.stdout.lines() {
                        let id = line.trim();
                        if !id.is_empty() {
                            let _ = crate::command_runner::run_async_or_error(
                                "nmcli",
                                &["con", "up", id],
                                20,
                            )
                            .await;
                            msgs.push(format!("{}: renewed", id));
                        }
                    }
                    Ok(CommandResult::ok(format!("DHCP renewed: {}", msgs.join("; "))))
                } else {
                    crate::command_runner::run_async_or_error("nmcli", &["con", "up", &conn], 20)
                        .await
                        .map(|_| CommandResult::ok("DHCP lease renewed."))
                        .or_else(|e| {
                            Ok(CommandResult::err(format!(
                                "Failed to renew DHCP: {}",
                                crate::validation::sanitize_output(&e.to_string())
                            )))
                        })
                }
            } else {
                Ok(CommandResult::err("NetworkManager (nmcli) is required for DHCP renewal on Linux."))
            }
        }
    }
}

pub async fn run_traceroute(host: String) -> AppResult<TracerouteResult> {
    #[cfg(target_os = "windows")]
    {
        dns::windows::traceroute(&host).await
    }
    #[cfg(target_os = "macos")]
    {
        dns::macos::traceroute(&host).await
    }
    #[cfg(target_os = "linux")]
    {
        dns::linux::traceroute(&host).await
    }
}

pub async fn ping_target(
    state: tauri::State<'_, AppState>,
    host: String,
    adapter_id: Option<String>,
) -> AppResult<models::TargetProbeResult> {
    Ok(diagnostics::ping_host_on_adapter(&host, adapter_id.as_deref(), &state.caps).await)
}

pub async fn comprehensive_check(
    state: tauri::State<'_, AppState>,
) -> AppResult<models::DiagnosticResult> {
    let targets = {
        let cfg = state.config.lock().unwrap();
        cfg.settings.diagnostic_targets.clone()
    };
    diagnostics::comprehensive_check(&targets, &dns::list_adapters(&state.caps).await.unwrap_or_default(), &state.caps).await
}

pub async fn probe_server(
    state: tauri::State<'_, AppState>,
    server: String,
    adapter_id: Option<String>,
) -> AppResult<models::DnsMetrics> {
    diagnostics::probe_dns_server(&server, adapter_id.as_deref(), &state.caps).await
}

pub async fn resolve_hostname(
    state: tauri::State<'_, AppState>,
    hostname: String,
    dns_server: Option<String>,
) -> AppResult<Vec<String>> {
    diagnostics::resolve_host(&hostname, dns_server.as_deref(), &state.caps).await
}

pub fn save_profile(state: tauri::State<'_, AppState>, profile: DnsProfile) -> AppResult<()> {
    validation::validate_profile_id(&profile.id)?;
    validation::validate_profile_name(&profile.name)?;
    validation::validate_ipv4(&profile.primary_ipv4)?;
    if let Some(sec) = &profile.secondary_ipv4 {
        if !sec.is_empty() {
            validation::validate_ipv4(sec)?;
        }
    }
    if let Some(v6) = &profile.primary_ipv6 {
        if !v6.is_empty() {
            validation::validate_ipv6(v6)?;
        }
    }
    if let Some(v6) = &profile.secondary_ipv6 {
        if !v6.is_empty() {
            validation::validate_ipv6(v6)?;
        }
    }
    if let Some(url) = &profile.doh_url {
        if !url.is_empty() {
            validation::validate_doh_url(url)?;
        }
    }
    if let Some(host) = &profile.dot_host {
        if !host.is_empty() {
            validation::validate_dot_host(host)?;
        }
    }

    let mut cfg = state.config.lock().unwrap();
    if let Some(existing) = cfg.profiles.iter_mut().find(|p| p.id == profile.id) {
        *existing = profile.clone();
    } else {
        cfg.profiles.push(profile);
    }
    config::save_config(&cfg)?;
    Ok(())
}

pub fn delete_profile(state: tauri::State<'_, AppState>, profile_id: String) -> AppResult<()> {
    let mut cfg = state.config.lock().unwrap();
    cfg.profiles.retain(|p| p.id != profile_id);
    if cfg.active_profile_id.as_deref() == Some(&profile_id) {
        cfg.active_profile_id = None;
        cfg.is_connected = false;
    }
    config::save_config(&cfg)?;
    Ok(())
}

pub fn update_settings(state: tauri::State<'_, AppState>, settings: AppSettings) -> AppResult<()> {
    let mut cfg = state.config.lock().unwrap();
    cfg.settings = settings;
    config::save_config(&cfg)?;
    Ok(())
}

pub fn add_rule(state: tauri::State<'_, AppState>, rule: AppRule) -> AppResult<()> {
    validation::validate_profile_id(&rule.id)?;
    validation::validate_profile_name(&rule.app_name)?;
    let mut cfg = state.config.lock().unwrap();
    if cfg.rules.iter().any(|r| r.id == rule.id) {
        return Err(errors::AppError::Validation("Rule ID already exists".to_string()));
    }
    cfg.rules.push(rule);
    config::save_config(&cfg)?;
    Ok(())
}

pub fn remove_rule(state: tauri::State<'_, AppState>, rule_id: String) -> AppResult<()> {
    let mut cfg = state.config.lock().unwrap();
    cfg.rules.retain(|r| r.id != rule_id);
    config::save_config(&cfg)?;
    Ok(())
}

pub fn toggle_rule(state: tauri::State<'_, AppState>, rule_id: String, enabled: bool) -> AppResult<()> {
    let mut cfg = state.config.lock().unwrap();
    if let Some(rule) = cfg.rules.iter_mut().find(|r| r.id == rule_id) {
        rule.enabled = enabled;
    }
    config::save_config(&cfg)?;
    Ok(())
}

pub fn add_auto_switch_rule(
    state: tauri::State<'_, AppState>,
    rule: models::AutoSwitchRule,
) -> AppResult<()> {
    validation::validate_profile_id(&rule.profile_id)?;
    if rule.app_label.is_empty() || rule.match_name.is_empty() {
        return Err(errors::AppError::Validation(
            "App label and match name are required".to_string(),
        ));
    }
    let mut cfg = state.config.lock().unwrap();
    if cfg.auto_switch_rules.iter().any(|r| r.id == rule.id) {
        return Err(errors::AppError::Validation(
            "Auto-switch rule ID already exists".to_string(),
        ));
    }
    cfg.auto_switch_rules.push(rule);
    config::save_config(&cfg)?;
    Ok(())
}

pub fn remove_auto_switch_rule(
    state: tauri::State<'_, AppState>,
    rule_id: String,
) -> AppResult<()> {
    let mut cfg = state.config.lock().unwrap();
    cfg.auto_switch_rules.retain(|r| r.id != rule_id);
    config::save_config(&cfg)?;
    Ok(())
}

pub fn toggle_auto_switch_rule(
    state: tauri::State<'_, AppState>,
    rule_id: String,
    enabled: bool,
) -> AppResult<()> {
    let mut cfg = state.config.lock().unwrap();
    if let Some(rule) = cfg.auto_switch_rules.iter_mut().find(|r| r.id == rule_id) {
        rule.enabled = enabled;
    }
    config::save_config(&cfg)?;
    Ok(())
}

pub fn set_auto_switch_enabled(
    state: tauri::State<'_, AppState>,
    enabled: bool,
) -> AppResult<()> {
    let mut cfg = state.config.lock().unwrap();
    cfg.auto_switch_enabled = enabled;
    config::save_config(&cfg)?;
    Ok(())
}

pub async fn list_running_processes() -> AppResult<Vec<models::RunningProcess>> {
    Ok(process_watcher::list_running_processes().await)
}

/// Launch an installer executable. The path should point to a downloaded
/// installer file in the user's Downloads directory.
pub fn run_installer(path: String) -> AppResult<()> {
    if path.trim().is_empty() {
        return Err(errors::AppError::Validation("Installer path is empty".to_string()));
    }
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err(errors::AppError::Validation(format!(
            "Installer not found at: {}",
            crate::validation::sanitize_output(&path)
        )));
    }
    std::process::Command::new(p)
        .spawn()
        .map_err(|e| errors::AppError::Command(format!("Failed to run installer: {}", e)))?;
    Ok(())
}

/// Open a URL in the default system browser.
pub fn open_url(url: String) -> AppResult<()> {
    if url.trim().is_empty() {
        return Err(errors::AppError::Validation("URL is empty".to_string()));
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &url])
            .spawn()
            .map_err(|e| errors::AppError::Command(format!("Failed to open URL: {}", e)))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| errors::AppError::Command(format!("Failed to open URL: {}", e)))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| errors::AppError::Command(format!("Failed to open URL: {}", e)))?;
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_ipv4_valid() {
        assert!(validation::validate_ipv4("8.8.8.8").is_ok());
        assert!(validation::validate_ipv4("1.1.1.1").is_ok());
    }

    #[test]
    fn test_validation_ipv4_invalid() {
        assert!(validation::validate_ipv4("999.999.999.999").is_err());
        assert!(validation::validate_ipv4("not-an-ip").is_err());
        assert!(validation::validate_ipv4("").is_err());
        assert!(validation::validate_ipv4("1.2.3").is_err());
    }

    #[test]
    fn test_validation_ipv6_valid() {
        assert!(validation::validate_ipv6("::1").is_ok());
        assert!(validation::validate_ipv6("2606:4700:4700::1111").is_ok());
    }

    #[test]
    fn test_validation_ipv6_invalid() {
        assert!(validation::validate_ipv6("not-an-ip").is_err());
        assert!(validation::validate_ipv6("").is_err());
    }

    #[test]
    fn test_validation_profile_id() {
        assert!(validation::validate_profile_id("preset_cloudflare").is_ok());
        assert!(validation::validate_profile_id("my-profile-1").is_ok());
        assert!(validation::validate_profile_id("").is_err());
        assert!(validation::validate_profile_id("has spaces").is_err());
        assert!(validation::validate_profile_id("has;semicolon").is_err());
    }

    #[test]
    fn test_validation_profile_name() {
        assert!(validation::validate_profile_name("Cloudflare").is_ok());
        assert!(validation::validate_profile_name("My Custom DNS").is_ok());
        assert!(validation::validate_profile_name("").is_err());
        assert!(validation::validate_profile_name(&"a".repeat(65)).is_err());
    }

    #[test]
    fn test_validation_doh_url() {
        assert!(validation::validate_doh_url("https://cloudflare-dns.com/dns-query").is_ok());
        assert!(validation::validate_doh_url("http://cloudflare-dns.com/dns-query").is_err());
        assert!(validation::validate_doh_url("").is_err());
    }

    #[test]
    fn test_sanitize_output() {
        assert_eq!(validation::sanitize_output("hello\u{0}world"), "helloworld");
        assert_eq!(validation::sanitize_output("hello\nworld"), "hello world");
        assert_eq!(validation::sanitize_output("  clean  "), "clean");
    }

    #[test]
    fn test_truncate_output() {
        let long = "a".repeat(100);
        let truncated = validation::truncate_output(&long, 10);
        assert!(truncated.contains("[truncated]"));
        assert!(truncated.len() < 100);
        assert_eq!(validation::truncate_output("short", 10), "short");
    }

    #[test]
    fn test_command_result_ok() {
        let r = models::CommandResult::ok("success");
        assert!(r.ok);
        assert!(!r.requires_elevation);
    }

    #[test]
    fn test_command_result_err() {
        let r = models::CommandResult::err("failure");
        assert!(!r.ok);
        assert!(!r.requires_elevation);
    }

    #[test]
    fn test_command_result_err_elevation() {
        let r = models::CommandResult::err_elevation("need admin");
        assert!(!r.ok);
        assert!(r.requires_elevation);
    }

    #[test]
    fn test_config_default() {
        let cfg = models::AppConfig::default();
        assert!(!cfg.profiles.is_empty());
        assert!(cfg.profiles.iter().any(|p| p.id == "preset_cloudflare"));
        assert!(!cfg.settings.diagnostic_targets.is_empty());
        assert_eq!(cfg.schema_version, models::SCHEMA_VERSION);
    }

    #[test]
    fn test_config_round_trip() {
        let cfg = models::AppConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: models::AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.profiles.len(), cfg.profiles.len());
        assert_eq!(parsed.schema_version, cfg.schema_version);
    }

    #[test]
    fn test_platform_detect_os() {
        let os = platform::detect_os();
        let _ = match os {
            models::OsKind::Windows | models::OsKind::Linux | models::OsKind::Macos => (),
        };
    }

    #[test]
    fn test_platform_capabilities() {
        let caps = platform::detect_capabilities();
        assert!(!caps.supports_app_rules);
        assert!(caps.hidden_features.contains(&"app_rules".to_string()));
    }

    #[test]
    fn test_validate_apply_request_valid() {
        let req = models::DnsApplyRequest {
            profile_id: "test_profile".to_string(),
            adapter_id: None,
            primary_ipv4: "1.1.1.1".to_string(),
            secondary_ipv4: Some("1.0.0.1".to_string()),
            primary_ipv6: None,
            secondary_ipv6: None,
            enable_doh: false,
        };
        assert!(dns::validate_apply_request(&req).is_ok());
    }

    #[test]
    fn test_validate_apply_request_invalid_ip() {
        let req = models::DnsApplyRequest {
            profile_id: "test_profile".to_string(),
            adapter_id: None,
            primary_ipv4: "not-an-ip".to_string(),
            secondary_ipv4: None,
            primary_ipv6: None,
            secondary_ipv6: None,
            enable_doh: false,
        };
        assert!(dns::validate_apply_request(&req).is_err());
    }

    #[test]
    fn test_validate_apply_request_invalid_profile_id() {
        let req = models::DnsApplyRequest {
            profile_id: "has spaces".to_string(),
            adapter_id: None,
            primary_ipv4: "1.1.1.1".to_string(),
            secondary_ipv4: None,
            primary_ipv6: None,
            secondary_ipv6: None,
            enable_doh: false,
        };
        assert!(dns::validate_apply_request(&req).is_err());
    }

    #[test]
    fn test_error_serialization() {
        let err = errors::AppError::Validation("test error".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("Validation error"));
    }

    #[test]
    fn test_run_output() {
        let out = command_runner::RunOutput {
            status: 0,
            stdout: "hello".to_string(),
            stderr: String::new(),
        };
        assert!(out.success());
        assert_eq!(out.combined(), "hello");
    }

    #[test]
    fn test_run_output_failure() {
        let out = command_runner::RunOutput {
            status: 1,
            stdout: String::new(),
            stderr: "error".to_string(),
        };
        assert!(!out.success());
        assert_eq!(out.combined(), "error");
    }
}
