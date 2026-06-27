use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OsKind {
    Windows,
    Linux,
    Macos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsProfile {
    pub id: String,
    pub name: String,
    pub primary_ipv4: String,
    pub secondary_ipv4: Option<String>,
    pub primary_ipv6: Option<String>,
    pub secondary_ipv6: Option<String>,
    pub doh_url: Option<String>,
    pub dot_host: Option<String>,
    pub favorite: bool,
    pub preset: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticTarget {
    pub id: String,
    pub name: String,
    pub host: String,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: String,
    pub doh_enabled: bool,
    /// "doh" or "dot" — the encryption protocol when doh_enabled is true.
    #[serde(default = "default_dns_encryption_type")]
    pub dns_encryption_type: String,
    pub ipv6_enabled: bool,
    pub minimize_to_tray: bool,
    pub start_on_boot: bool,
    pub active_adapter_id: Option<String>,
    pub diagnostic_targets: Vec<DiagnosticTarget>,
    /// Automatically ping all profiles on startup to show latency.
    #[serde(default = "default_true")]
    pub auto_ping_enabled: bool,
    /// Automatically check for updates on startup.
    #[serde(default)]
    pub auto_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformCapabilities {
    pub os: OsKind,
    pub supports_ipv4_dns: bool,
    pub supports_ipv6_dns: bool,
    pub supports_doh: bool,
    pub supports_flush_dns: bool,
    pub supports_dhcp_renew: bool,
    pub supports_adapter_reset: bool,
    pub supports_traceroute: bool,
    pub supports_app_rules: bool,
    /// Whether the background process watcher (auto-switch) is supported.
    pub supports_auto_switch: bool,
    pub requires_elevation_for_dns: bool,
    pub dns_backend: String,
    pub hidden_features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAdapter {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_up: bool,
    pub is_primary: bool,
    pub ipv4_dns: Vec<String>,
    pub ipv6_dns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsApplyRequest {
    pub profile_id: String,
    pub adapter_id: Option<String>,
    pub primary_ipv4: String,
    pub secondary_ipv4: Option<String>,
    pub primary_ipv6: Option<String>,
    pub secondary_ipv6: Option<String>,
    pub enable_doh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub ok: bool,
    pub message: String,
    pub requires_elevation: bool,
    pub details: Option<String>,
}

impl CommandResult {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            ok: true,
            message: message.into(),
            requires_elevation: false,
            details: None,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            message: message.into(),
            requires_elevation: false,
            details: None,
        }
    }

    pub fn err_elevation(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            message: message.into(),
            requires_elevation: true,
            details: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsMetrics {
    pub profile_id: String,
    pub server: String,
    pub latency_ms: Option<f64>,
    pub reachability_percent: f64,
    pub packet_loss_percent: f64,
    pub tested_at: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetProbeResult {
    pub id: String,
    pub name: String,
    pub host: String,
    pub latency_ms: Option<f64>,
    pub reachable: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticResult {
    pub latency_ms: Option<f64>,
    pub dnssec_valid: Option<bool>,
    pub leak_secure: Option<bool>,
    pub packet_loss_percent: f64,
    pub targets: Vec<TargetProbeResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRule {
    pub id: String,
    pub app_name: String,
    pub app_path: Option<String>,
    pub profile_id: String,
    pub enabled: bool,
}

/// A process-watcher rule: when a process whose executable matches `match_name`
/// (or `app_path`) starts running, the system DNS is switched to `profile_id`.
/// When the last running watched app exits, DNS reverts to whatever was active
/// before the auto-switch took over.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSwitchRule {
    pub id: String,
    /// Display label shown in the UI.
    pub app_label: String,
    /// Executable file name to match, lowercased and without directory
    /// (e.g. "chrome.exe", "steam", "firefox"). This is the primary matcher.
    pub match_name: String,
    /// Optional full path chosen via the file picker. Informational / used to
    /// derive `match_name`; matching is done on the file name.
    pub app_path: Option<String>,
    pub profile_id: String,
    pub enabled: bool,
}

/// A process currently running on the system, used to populate the
/// "pick from running apps" list in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningProcess {
    /// Lowercased executable file name (e.g. "chrome.exe").
    pub name: String,
    /// Full executable path when available.
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub name: String,
    pub ip: String,
    pub time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub active_profile_id: Option<String>,
    pub is_connected: bool,
    pub profiles: Vec<DnsProfile>,
    pub history: Vec<HistoryEntry>,
    pub rules: Vec<AppRule>,
    /// Process-watcher auto-switch rules.
    #[serde(default)]
    pub auto_switch_rules: Vec<AutoSwitchRule>,
    /// Master on/off for the background process watcher.
    #[serde(default)]
    pub auto_switch_enabled: bool,
    pub settings: AppSettings,
    pub schema_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerouteHop {
    pub index: u32,
    pub host: Option<String>,
    pub latency_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerouteResult {
    pub ok: bool,
    pub host: String,
    pub hops: Vec<TracerouteHop>,
    pub raw: Option<String>,
    pub error: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            active_profile_id: None,
            is_connected: false,
            profiles: default_presets(),
            history: Vec::new(),
            rules: Vec::new(),
            auto_switch_rules: Vec::new(),
            auto_switch_enabled: false,
            settings: AppSettings {
                theme: "dark".to_string(),
                doh_enabled: false,
                dns_encryption_type: "doh".to_string(),
                ipv6_enabled: false,
                minimize_to_tray: false,
                start_on_boot: false,
                active_adapter_id: None,
                diagnostic_targets: default_diag_targets(),
                auto_ping_enabled: true,
                auto_update: false,
            },
            schema_version: 3,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_dns_encryption_type() -> String {
    "doh".to_string()
}

pub const SCHEMA_VERSION: u32 = 3;

fn default_diag_targets() -> Vec<DiagnosticTarget> {
    vec![
        DiagnosticTarget { id: "cf".into(), name: "Cloudflare".into(), host: "1.1.1.1".into(), icon: None },
        DiagnosticTarget { id: "epic".into(), name: "Epic Games".into(), host: "www.epicgames.com".into(), icon: None },
        DiagnosticTarget { id: "gemini".into(), name: "Gemini".into(), host: "www.google.com".into(), icon: None },
        DiagnosticTarget { id: "steam".into(), name: "Steam".into(), host: "store.steampowered.com".into(), icon: None },
        DiagnosticTarget { id: "yt".into(), name: "YouTube".into(), host: "www.youtube.com".into(), icon: None },
        DiagnosticTarget { id: "gpt".into(), name: "ChatGPT".into(), host: "chat.openai.com".into(), icon: None },
        DiagnosticTarget { id: "git".into(), name: "GitHub".into(), host: "github.com".into(), icon: None },
    ]
}

fn default_presets() -> Vec<DnsProfile> {
    vec![
        DnsProfile {
            id: "preset_cloudflare".into(),
            name: "Cloudflare".into(),
            primary_ipv4: "1.1.1.1".into(),
            secondary_ipv4: Some("1.0.0.1".into()),
            primary_ipv6: Some("2606:4700:4700::1111".into()),
            secondary_ipv6: Some("2606:4700:4700::1001".into()),
            doh_url: Some("https://cloudflare-dns.com/dns-query".into()),
            dot_host: Some("one.one.one.one".into()),
            favorite: true,
            preset: true,
        },
        DnsProfile {
            id: "preset_google".into(),
            name: "Google".into(),
            primary_ipv4: "8.8.8.8".into(),
            secondary_ipv4: Some("8.8.4.4".into()),
            primary_ipv6: Some("2001:4860:4860::8888".into()),
            secondary_ipv6: Some("2001:4860:4860::8844".into()),
            doh_url: Some("https://dns.google/dns-query".into()),
            dot_host: Some("dns.google".into()),
            favorite: false,
            preset: true,
        },
        DnsProfile {
            id: "preset_quad9".into(),
            name: "Quad9".into(),
            primary_ipv4: "9.9.9.9".into(),
            secondary_ipv4: Some("149.112.112.112".into()),
            primary_ipv6: Some("2620:fe::fe".into()),
            secondary_ipv6: Some("2620:fe::9".into()),
            doh_url: Some("https://dns.quad9.net/dns-query".into()),
            dot_host: Some("dns.quad9.net".into()),
            favorite: false,
            preset: true,
        },
        DnsProfile {
            id: "preset_opendns".into(),
            name: "OpenDNS".into(),
            primary_ipv4: "208.67.222.222".into(),
            secondary_ipv4: Some("208.67.220.220".into()),
            primary_ipv6: Some("2620:119:35::35".into()),
            secondary_ipv6: Some("2620:119:53::53".into()),
            doh_url: None,
            dot_host: None,
            favorite: false,
            preset: true,
        },
        DnsProfile {
            id: "preset_adguard".into(),
            name: "AdGuard".into(),
            primary_ipv4: "94.140.14.14".into(),
            secondary_ipv4: Some("94.140.15.15".into()),
            primary_ipv6: Some("2a10:50c0::ad1:ff".into()),
            secondary_ipv6: Some("2a10:50c0::ad2:ff".into()),
            doh_url: Some("https://dns.adguard-dns.com/dns-query".into()),
            dot_host: Some("dns.adguard-dns.com".into()),
            favorite: false,
            preset: true,
        },
        DnsProfile {
            id: "preset_mullvad".into(),
            name: "Mullvad".into(),
            primary_ipv4: "194.242.2.2".into(),
            secondary_ipv4: Some("194.242.2.3".into()),
            primary_ipv6: Some("2a07:e3400::2".into()),
            secondary_ipv6: Some("2a07:e3400::3".into()),
            doh_url: Some("https://doh.mullvad.net/dns-query".into()),
            dot_host: Some("doh.mullvad.net".into()),
            favorite: false,
            preset: true,
        },
    ]
}