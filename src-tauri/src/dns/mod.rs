pub mod linux;
pub mod macos;
pub mod windows;

use crate::errors::AppResult;
use crate::models::{DnsApplyRequest, NetworkAdapter, OsKind, PlatformCapabilities};
use crate::platform::detect_os;

/// List network adapters and their current DNS settings.
pub async fn list_adapters(caps: &PlatformCapabilities) -> AppResult<Vec<NetworkAdapter>> {
    match caps.os {
        OsKind::Windows => windows::list_adapters().await,
        OsKind::Macos => macos::list_adapters().await,
        OsKind::Linux => linux::list_adapters(&caps.dns_backend).await,
    }
}

/// Apply DNS settings from a validated request.
pub async fn apply_dns(
    req: &DnsApplyRequest,
    caps: &PlatformCapabilities,
) -> AppResult<crate::models::CommandResult> {
    if !caps.supports_ipv4_dns {
        return Ok(crate::models::CommandResult::err(
            "DNS modification is not supported on this platform/backend.",
        ));
    }
    match caps.os {
        OsKind::Windows => windows::apply_dns(req).await,
        OsKind::Macos => macos::apply_dns(req).await,
        OsKind::Linux => linux::apply_dns(req, &caps.dns_backend).await,
    }
}

/// Clear/reset DNS to automatic (DHCP) on a given adapter (or all if None).
pub async fn clear_dns(
    adapter_id: Option<&str>,
    caps: &PlatformCapabilities,
) -> AppResult<crate::models::CommandResult> {
    if !caps.supports_ipv4_dns {
        return Ok(crate::models::CommandResult::err(
            "DNS modification is not supported on this platform/backend.",
        ));
    }
    match caps.os {
        OsKind::Windows => windows::clear_dns(adapter_id).await,
        OsKind::Macos => macos::clear_dns(adapter_id).await,
        OsKind::Linux => linux::clear_dns(adapter_id, &caps.dns_backend).await,
    }
}

/// Elevation is now requested per-action by each platform backend through the
/// native OS prompt (UAC / admin / pkexec). This helper is retained for the
/// command layer but no longer pre-blocks: a non-elevated process will trigger
/// the prompt when the privileged command runs.
pub fn require_elevation(_caps: &PlatformCapabilities) -> AppResult<()> {
    Ok(())
}

/// Validate the apply request fields before passing to the OS backend.
pub fn validate_apply_request(req: &DnsApplyRequest) -> AppResult<()> {
    crate::validation::validate_ipv4(&req.primary_ipv4)?;
    if let Some(sec) = &req.secondary_ipv4 {
        crate::validation::validate_ipv4(sec)?;
    }
    if let Some(v6) = &req.primary_ipv6 {
        crate::validation::validate_ipv6(v6)?;
    }
    if let Some(v6) = &req.secondary_ipv6 {
        crate::validation::validate_ipv6(v6)?;
    }
    crate::validation::validate_profile_id(&req.profile_id)?;
    Ok(())
}

/// Current OS kind shortcut.
pub fn current_os() -> OsKind {
    detect_os()
}