use crate::models::{OsKind, PlatformCapabilities};

pub fn detect_os() -> OsKind {
    #[cfg(target_os = "windows")]
    {
        OsKind::Windows
    }
    #[cfg(target_os = "macos")]
    {
        OsKind::Macos
    }
    #[cfg(target_os = "linux")]
    {
        OsKind::Linux
    }
}

/// Detect which DNS backend is available on Linux.
/// Returns one of: "networkmanager", "systemd-resolved", "resolvconf", "resolv.conf", "unknown".
#[cfg(target_os = "linux")]
pub fn detect_linux_dns_backend() -> String {
    use crate::command_runner::which_exists;
    use std::path::Path;

    if which_exists("nmcli") {
        return "networkmanager".to_string();
    }
    if which_exists("resolvectl") {
        return "systemd-resolved".to_string();
    }
    if which_exists("resolvconf") {
        return "resolvconf".to_string();
    }
    if Path::new("/etc/resolv.conf").exists() {
        return "resolv.conf".to_string();
    }
    "unknown".to_string()
}

/// Detect capabilities for the current platform.
pub fn detect_capabilities() -> PlatformCapabilities {
    match detect_os() {
        OsKind::Windows => PlatformCapabilities {
            os: OsKind::Windows,
            supports_ipv4_dns: true,
            supports_ipv6_dns: true,
            supports_doh: true,
            supports_flush_dns: true,
            supports_dhcp_renew: true,
            supports_adapter_reset: true,
            supports_traceroute: true,
            supports_app_rules: false,
            supports_auto_switch: true,
            requires_elevation_for_dns: true,
            dns_backend: "netsh".to_string(),
            hidden_features: vec!["app_rules".to_string()],
        },
        OsKind::Macos => PlatformCapabilities {
            os: OsKind::Macos,
            supports_ipv4_dns: true,
            supports_ipv6_dns: true,
            supports_doh: false,
            supports_flush_dns: true,
            supports_dhcp_renew: true,
            supports_adapter_reset: false,
            supports_traceroute: true,
            supports_app_rules: false,
            supports_auto_switch: true,
            requires_elevation_for_dns: true,
            dns_backend: "networksetup".to_string(),
            hidden_features: vec!["app_rules".to_string(), "adapter_reset".to_string()],
        },
        OsKind::Linux => {
            #[cfg(target_os = "linux")]
            {
                let backend = detect_linux_dns_backend();
                let hidden = build_linux_hidden_features(&backend);
                PlatformCapabilities {
                    os: OsKind::Linux,
                    supports_ipv4_dns: backend != "unknown",
                    supports_ipv6_dns: backend != "unknown",
                    supports_doh: false,
                    supports_flush_dns: true,
                    supports_dhcp_renew: backend == "networkmanager",
                    supports_adapter_reset: false,
                    supports_traceroute: true,
                    supports_app_rules: false,
                    // Process listing via /proc is always available on Linux; the
                    // watcher becomes meaningful only when DNS can also be set.
                    supports_auto_switch: backend != "unknown",
                    requires_elevation_for_dns: true,
                    dns_backend: backend.clone(),
                    hidden_features: hidden,
                }
            }
            #[cfg(not(target_os = "linux"))]
            {
                PlatformCapabilities {
                    os: OsKind::Linux,
                    supports_ipv4_dns: false,
                    supports_ipv6_dns: false,
                    supports_doh: false,
                    supports_flush_dns: false,
                    supports_dhcp_renew: false,
                    supports_adapter_reset: false,
                    supports_traceroute: false,
                    supports_app_rules: false,
                    supports_auto_switch: false,
                    requires_elevation_for_dns: true,
                    dns_backend: "unknown".to_string(),
                    hidden_features: vec![
                        "app_rules".to_string(),
                        "adapter_reset".to_string(),
                        "dhcp_renew".to_string(),
                        "traceroute".to_string(),
                        "flush_cache".to_string(),
                        "auto_switch".to_string(),
                    ],
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn build_linux_hidden_features(backend: &str) -> Vec<String> {
    let mut hidden = vec![
        "app_rules".to_string(),
        "adapter_reset".to_string(),
        "doh".to_string(),
    ];
    if backend != "networkmanager" {
        hidden.push("dhcp_renew".to_string());
    }
    if backend == "unknown" {
        hidden.push("flush_cache".to_string());
    }
    hidden
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_os_returns_current_platform() {
        let os = detect_os();
        let any_supported = matches!(os, OsKind::Windows | OsKind::Macos | OsKind::Linux);
        assert!(any_supported, "detect_os should return a supported OS kind");
    }

    #[test]
    fn windows_capabilities_hide_app_rules() {
        let caps = detect_capabilities();
        if matches!(caps.os, OsKind::Windows) {
            assert!(caps.supports_ipv4_dns);
            assert!(caps.supports_ipv6_dns);
            assert!(caps.requires_elevation_for_dns);
            assert!(caps.hidden_features.contains(&"app_rules".to_string()));
            assert!(!caps.supports_app_rules);
            assert!(caps.supports_doh);
            assert_eq!(caps.dns_backend, "netsh");
        }
    }

    #[test]
    fn macos_capabilities_hide_adapter_reset_and_app_rules() {
        let caps = detect_capabilities();
        if matches!(caps.os, OsKind::Macos) {
            assert!(caps.supports_ipv4_dns);
            assert!(caps.supports_ipv6_dns);
            assert!(!caps.supports_adapter_reset);
            assert!(!caps.supports_app_rules);
            assert!(!caps.supports_doh);
            assert!(caps.hidden_features.contains(&"app_rules".to_string()));
            assert!(caps.hidden_features.contains(&"adapter_reset".to_string()));
            assert_eq!(caps.dns_backend, "networksetup");
        }
    }

    #[test]
    fn linux_capabilities_hide_doh_and_app_rules() {
        let caps = detect_capabilities();
        if matches!(caps.os, OsKind::Linux) {
            assert!(!caps.supports_app_rules);
            assert!(!caps.supports_doh);
            assert!(!caps.supports_adapter_reset);
            assert!(caps.hidden_features.contains(&"app_rules".to_string()));
        }
    }

    #[test]
    fn every_platform_hides_app_rules_initially() {
        let caps = detect_capabilities();
        assert!(caps.hidden_features.contains(&"app_rules".to_string()));
    }
}