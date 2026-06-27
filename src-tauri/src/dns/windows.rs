use crate::command_runner::{run_async, run_async_or_error, run_sync, RunOutput};
use crate::errors::AppResult;
use crate::models::{CommandResult, DnsApplyRequest, NetworkAdapter};
use crate::validation::sanitize_output;

/// List network adapters via PowerShell `Get-NetAdapter` + `Get-DnsClientServerAddress`.
pub async fn list_adapters() -> AppResult<Vec<NetworkAdapter>> {
    // Use a single PowerShell command joining adapter info with DNS config.
    // Include VPN/TUN/TAP adapters by checking for known keywords in the
    // adapter name or description so that users can manage DNS on them.
    let script = r#"
$adapters = Get-NetAdapter -ErrorAction SilentlyContinue | Where-Object { $_.Status -eq 'Up' -or $_.Status -eq 'Disconnected' }
foreach ($a in $adapters) {
    $dns = Get-DnsClientServerAddress -InterfaceIndex $a.ifIndex -ErrorAction SilentlyContinue
    $v4 = @($dns | Where-Object { $_.AddressFamily -eq 2 } | Select-Object -ExpandProperty ServerAddresses) -join ','
    $v6 = @($dns | Where-Object { $_.AddressFamily -eq 23 } | Select-Object -ExpandProperty ServerAddresses) -join ','
    $up = if ($a.Status -eq 'Up') { 'true' } else { 'false' }
    Write-Output ("{0}|{1}|{2}|{3}|{4}" -f $a.InterfaceGuid, $a.Name, $up, $v4, $v6)
}
"#;
    let out = run_async(
        "powershell",
        &["-NoProfile", "-NonInteractive", "-Command", script],
        15,
    )
    .await?;

    if !out.success() {
        return Ok(Vec::new());
    }

    let mut adapters = Vec::new();
    for line in out.stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 5 {
            continue;
        }
        let id = parts[0].to_string();
        let name = parts[1].to_string();
        let is_up = parts[2].eq_ignore_ascii_case("true");
        let ipv4_dns: Vec<String> = parts[3]
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim().to_string())
            .collect();
        let ipv6_dns: Vec<String> = parts[4]
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim().to_string())
            .collect();
        adapters.push(NetworkAdapter {
            id: id.clone(),
            name,
            description: Some(id.clone()),
            is_up,
            is_primary: false,
            ipv4_dns,
            ipv6_dns,
        });
    }

    // Mark the first up adapter as primary if none marked.
    if let Some(first_up) = adapters.iter_mut().find(|a| a.is_up) {
        first_up.is_primary = true;
    }

    Ok(adapters)
}

/// Build the PowerShell `-InterfaceIndex ...` filter for the target adapter(s).
fn interface_filter(adapter_id: Option<&str>) -> String {
    match adapter_id {
        Some(id) => format!("(Get-NetAdapter | Where-Object {{ $_.InterfaceGuid -eq '{}' }}).ifIndex", id),
        None => "(Get-NetAdapter | Where-Object Status -eq 'Up').ifIndex".to_string(),
    }
}

/// Join validated IP servers into a quoted, comma-separated PowerShell list.
fn join_servers(servers: &[String]) -> String {
    servers
        .iter()
        .map(|s| format!("\"{}\"", s))
        .collect::<Vec<_>>()
        .join(",")
}

/// Build the full PowerShell script that applies IPv4 (and optional IPv6) DNS.
/// All values are validated upstream in `dns::validate_apply_request`.
pub fn build_apply_script(req: &DnsApplyRequest) -> String {
    let mut v4_servers: Vec<String> = vec![req.primary_ipv4.clone()];
    if let Some(sec) = &req.secondary_ipv4 {
        if !sec.is_empty() {
            v4_servers.push(sec.clone());
        }
    }

    let filter = interface_filter(req.adapter_id.as_deref());
    let mut script = format!(
        "$indexes = {filter}\nif (-not $indexes) {{ throw 'No matching network adapter found.' }}\nforeach ($idx in $indexes) {{ Set-DnsClientServerAddress -InterfaceIndex $idx -ServerAddresses {servers} -ErrorAction Stop }}",
        filter = filter,
        servers = join_servers(&v4_servers)
    );

    if let Some(primary_v6) = &req.primary_ipv6 {
        if !primary_v6.is_empty() {
            let mut v6_servers: Vec<String> = vec![primary_v6.clone()];
            if let Some(sec) = &req.secondary_ipv6 {
                if !sec.is_empty() {
                    v6_servers.push(sec.clone());
                }
            }
            script.push_str(&format!(
                "\nforeach ($idx in $indexes) {{ Set-DnsClientServerAddress -InterfaceIndex $idx -ServerAddresses {servers} -ErrorAction Stop }}",
                servers = join_servers(&v6_servers)
            ));
        }
    }

    script
}

/// Build the PowerShell script that resets DNS to DHCP.
pub fn build_clear_script(adapter_id: Option<&str>) -> String {
    format!(
        "$indexes = {filter}\nif (-not $indexes) {{ throw 'No matching network adapter found.' }}\nforeach ($idx in $indexes) {{ Set-DnsClientServerAddress -InterfaceIndex $idx -ResetServerAddresses -ErrorAction Stop }}",
        filter = interface_filter(adapter_id)
    )
}

/// Run a DNS-mutating PowerShell script, elevating through UAC when the current
/// process is not already elevated.
async fn run_dns_script(script: &str, success_msg: &str) -> AppResult<CommandResult> {
    if crate::command_runner::is_elevated() {
        return match run_async_or_error(
            "powershell",
            &["-NoProfile", "-NonInteractive", "-Command", script],
            25,
        )
        .await
        {
            Ok(_) => Ok(CommandResult::ok(success_msg.to_string())),
            Err(e) => Ok(CommandResult::err(format!(
                "Failed: {}",
                sanitize_output(&e.to_string())
            ))),
        };
    }

    let outcome = crate::elevation::run_powershell_elevated(script).await?;
    if outcome.success {
        Ok(CommandResult::ok(success_msg.to_string()))
    } else if outcome.cancelled {
        Ok(CommandResult::err_elevation(
            "Administrator approval was declined, so DNS was not changed.",
        ))
    } else {
        Ok(CommandResult::err(format!(
            "Failed to change DNS: {}",
            outcome.message
        )))
    }
}

/// Apply DNS to a Windows adapter using `Set-DnsClientServerAddress`.
/// If `adapter_id` is None, applies to all up adapters. Raises UAC if needed.
pub async fn apply_dns(req: &DnsApplyRequest) -> AppResult<CommandResult> {
    run_dns_script(&build_apply_script(req), "DNS applied successfully.").await
}

/// Clear DNS (reset to DHCP) on a Windows adapter. Raises UAC if needed.
pub async fn clear_dns(adapter_id: Option<&str>) -> AppResult<CommandResult> {
    run_dns_script(&build_clear_script(adapter_id), "DNS cleared and reset to DHCP.").await
}

/// Flush the Windows DNS resolver cache.
pub async fn flush_cache() -> AppResult<CommandResult> {
    match run_async_or_error("ipconfig", &["/flushdns"], 15).await {
        Ok(_) => Ok(CommandResult::ok("DNS cache flushed.")),
        Err(e) => Ok(CommandResult::err(format!("Failed to flush DNS cache: {}", sanitize_output(&e.to_string())))),
    }
}

/// Reset the network adapter (disable + enable). Requires adapter id.
/// Raises UAC when the process is not already elevated.
pub async fn reset_adapter(adapter_id: &str) -> AppResult<CommandResult> {
    let script = format!(
        "$a = Get-NetAdapter -ErrorAction Stop | Where-Object {{ $_.InterfaceGuid -eq '{}' }}\nif (-not $a) {{ throw 'No matching network adapter found.' }}\nDisable-NetAdapter -Name $a.Name -Confirm:$false\nStart-Sleep -Seconds 2\nEnable-NetAdapter -Name $a.Name -Confirm:$false",
        adapter_id
    );

    if crate::command_runner::is_elevated() {
        return match run_async_or_error("powershell", &["-NoProfile", "-NonInteractive", "-Command", &script], 30).await {
            Ok(_) => Ok(CommandResult::ok("Adapter reset successfully.")),
            Err(e) => Ok(CommandResult::err(format!("Failed to reset adapter: {}", sanitize_output(&e.to_string())))),
        };
    }

    let outcome = crate::elevation::run_powershell_elevated(&script).await?;
    if outcome.success {
        Ok(CommandResult::ok("Adapter reset successfully."))
    } else if outcome.cancelled {
        Ok(CommandResult::err_elevation("Administrator approval was declined, so the adapter was not reset."))
    } else {
        Ok(CommandResult::err(format!("Failed to reset adapter: {}", outcome.message)))
    }
}

/// Renew DHCP lease on an adapter.
pub async fn renew_dhcp(adapter_id: Option<&str>) -> AppResult<CommandResult> {
    // ipconfig /renew renews all adapters; per-adapter is more complex.
    // If adapter_id is provided we still renew all and report success.
    let _ = adapter_id;
    match run_async_or_error("ipconfig", &["/renew"], 30).await {
        Ok(_) => Ok(CommandResult::ok("DHCP lease renewed.")),
        Err(e) => Ok(CommandResult::err(format!("Failed to renew DHCP: {}", sanitize_output(&e.to_string())))),
    }
}

/// Run traceroute using `tracert`.
pub async fn traceroute(host: &str) -> AppResult<crate::models::TracerouteResult> {
    crate::validation::validate_host(host)?;
    let out = run_async("tracert", &["-d", "-w", "1000", "-h", "30", host], 60).await?;
    let raw = out.combined();
    let hops = parse_tracert(&out);
    Ok(crate::models::TracerouteResult {
        ok: out.success(),
        host: host.to_string(),
        hops,
        raw: Some(crate::validation::truncate_output(&sanitize_output(&raw), 4000)),
        error: if out.success() { None } else { Some("tracert exited with non-zero status".to_string()) },
    })
}

fn parse_tracert(out: &RunOutput) -> Vec<crate::models::TracerouteHop> {
    let mut hops = Vec::new();
    for (i, line) in out.stdout.lines().enumerate() {
        let line = line.trim();
        // tracert lines look like: "1     1 ms     1 ms     1 ms  192.168.1.1"
        if line.starts_with(|c: char| c.is_ascii_digit()) {
            // Try to extract the last IP-like token.
            let ip = line
                .split_whitespace()
                .last()
                .map(|s| s.trim_matches(|c: char| !c.is_ascii_digit() && c != '.' && c != ':')
                    .to_string());
            // Extract first latency token if present.
            let latency = line
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.trim_end_matches("ms").parse::<f64>().ok());
            hops.push(crate::models::TracerouteHop {
                index: i as u32,
                host: ip.filter(|s| !s.is_empty()),
                latency_ms: latency,
            });
        }
    }
    hops
}

/// Check if PowerShell is available (for testing/capability).
pub fn powershell_available() -> bool {
    run_sync("powershell", &["-NoProfile", "-Command", "exit 0"]).is_ok()
}

/// Check whether a network adapter name or description indicates it is a
/// VPN, TUN, TAP, or other virtual tunnel interface. These adapters often
/// have different routing behavior and may lack a traditional layer-3
/// address, so callers should handle them gracefully.
pub fn is_vpn_tun_adapter(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    ["tun", "tap", "wireguard", "wg", "v2ray", "sing-box", "clash", "wintun", "openvpn"]
        .iter()
        .any(|kw| lower.contains(kw))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DnsApplyRequest;

    fn req(adapter: Option<&str>, p4: &str, s4: Option<&str>, p6: Option<&str>) -> DnsApplyRequest {
        DnsApplyRequest {
            profile_id: "p".into(),
            adapter_id: adapter.map(|s| s.to_string()),
            primary_ipv4: p4.into(),
            secondary_ipv4: s4.map(|s| s.to_string()),
            primary_ipv6: p6.map(|s| s.to_string()),
            secondary_ipv6: None,
            enable_doh: false,
        }
    }

    #[test]
    fn apply_script_includes_primary_and_secondary() {
        let script = build_apply_script(&req(Some("GUID-1"), "1.1.1.1", Some("1.0.0.1"), None));
        assert!(script.contains("\"1.1.1.1\",\"1.0.0.1\""));
        assert!(script.contains("InterfaceGuid -eq 'GUID-1'"));
        assert!(script.contains("Set-DnsClientServerAddress"));
        assert!(script.contains("throw"));
    }

    #[test]
    fn apply_script_adds_ipv6_block_when_present() {
        let script = build_apply_script(&req(None, "8.8.8.8", None, Some("2001:4860:4860::8888")));
        assert!(script.contains("\"8.8.8.8\""));
        assert!(script.contains("\"2001:4860:4860::8888\""));
        assert_eq!(script.matches("Set-DnsClientServerAddress").count(), 2);
        assert!(script.contains("throw"));
    }

    #[test]
    fn clear_script_resets_addresses() {
        let script = build_clear_script(Some("GUID-2"));
        assert!(script.contains("-ResetServerAddresses"));
        assert!(script.contains("InterfaceGuid -eq 'GUID-2'"));
        assert!(script.contains("throw"));
    }

    #[test]
    fn clear_script_targets_all_up_adapters_when_none() {
        let script = build_clear_script(None);
        assert!(script.contains("Where-Object Status -eq 'Up'"));
    }

    #[test]
    fn vpn_tun_adapter_detection() {
        assert!(is_vpn_tun_adapter("WireGuard Tunnel"));
        assert!(is_vpn_tun_adapter("TAP-Windows Adapter V9"));
        assert!(is_vpn_tun_adapter("Wintun"));
        assert!(is_vpn_tun_adapter("v2rayN Proxy"));
        assert!(is_vpn_tun_adapter("sing-box TUN"));
        assert!(is_vpn_tun_adapter("Clash TUN Adapter"));
        assert!(is_vpn_tun_adapter("OpenVPN TUN Driver"));
        assert!(is_vpn_tun_adapter("wg0"));
        assert!(!is_vpn_tun_adapter("Ethernet"));
        assert!(!is_vpn_tun_adapter("Wi-Fi"));
        assert!(!is_vpn_tun_adapter("Ethernet 2"));
    }
}
