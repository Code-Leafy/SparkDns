use crate::command_runner::{run_async, run_async_or_error};
use crate::errors::AppResult;
use crate::models::{
    DiagnosticResult, DiagnosticTarget, DnsMetrics, NetworkAdapter, OsKind, PlatformCapabilities,
    TargetProbeResult,
};
use crate::validation::sanitize_output;
use std::time::Instant;

/// Look up the IPv4 address assigned to a specific network adapter by its
/// InterfaceGuid. Returns `None` when the adapter cannot be found or has no
/// IPv4 address (e.g. a TUN/TAP tunnel without a layer-3 address).
async fn adapter_ipv4_address(adapter_guid: &str, caps: &PlatformCapabilities) -> Option<String> {
    match caps.os {
        OsKind::Windows => {
            let script = format!(
                r#"Get-NetIPAddress -InterfaceIndex ((Get-NetAdapter -InterfaceGuid '{guid}').ifIndex) -AddressFamily IPv4 -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty IPAddress"#,
                guid = adapter_guid
            );
            let out = run_async(
                "powershell",
                &["-NoProfile", "-NonInteractive", "-Command", &script],
                10,
            )
            .await
            .ok()?;
            if out.success() {
                let ip = out.stdout.trim().to_string();
                if !ip.is_empty() {
                    return Some(ip);
                }
            }
            None
        }
        _ => None,
    }
}

/// Ping a single host and return a probe result.
pub async fn ping_host(host: &str, caps: &PlatformCapabilities) -> TargetProbeResult {
    ping_host_on_adapter(host, None, caps).await
}

/// Ping a single host, optionally binding to a specific adapter.
/// When `adapter_id` is provided the ping uses the `-S` source-address flag
/// on Windows (or equivalent) so the packet is routed through the correct
/// interface — critical when a VPN/TUN adapter is active.
pub async fn ping_host_on_adapter(
    host: &str,
    adapter_id: Option<&str>,
    caps: &PlatformCapabilities,
) -> TargetProbeResult {
    if let Err(e) = crate::validation::validate_host(host) {
        return TargetProbeResult {
            id: host.to_string(),
            name: host.to_string(),
            host: host.to_string(),
            latency_ms: None,
            reachable: false,
            error: Some(e.to_string()),
        };
    }

    let source_addr = if let Some(guid) = adapter_id {
        adapter_ipv4_address(guid, caps).await
    } else {
        None
    };

    let mut args: Vec<&str> = match caps.os {
        OsKind::Windows => vec!["-n", "4", "-w", "2000"],
        _ => vec!["-c", "4", "-W", "2"],
    };

    if let Some(ref src) = source_addr {
        match caps.os {
            OsKind::Windows => {
                args.push("-S");
                args.push(src.as_str());
            }
            _ => {
                args.push("-I");
                args.push(src.as_str());
            }
        }
    }

    args.push(host);

    let program = "ping";

    let start = Instant::now();
    let out = run_async(program, &args, 15).await;
    let elapsed = start.elapsed().as_millis() as f64;

    match out {
        Ok(o) => {
            if o.success() {
                let latency = parse_ping_latency(&o.stdout, caps.os).unwrap_or(elapsed);
                TargetProbeResult {
                    id: host.to_string(),
                    name: host.to_string(),
                    host: host.to_string(),
                    latency_ms: Some(latency),
                    reachable: true,
                    error: None,
                }
            } else {
                TargetProbeResult {
                    id: host.to_string(),
                    name: host.to_string(),
                    host: host.to_string(),
                    latency_ms: None,
                    reachable: false,
                    error: Some("Host unreachable".to_string()),
                }
            }
        }
        Err(e) => TargetProbeResult {
            id: host.to_string(),
            name: host.to_string(),
            host: host.to_string(),
            latency_ms: None,
            reachable: false,
            error: Some(sanitize_output(&e.to_string())),
        },
    }
}

/// Parse the average latency from ping output.
fn parse_ping_latency(stdout: &str, os: OsKind) -> Option<f64> {
    match os {
        OsKind::Windows => {
            for line in stdout.lines() {
                if line.contains("Average") {
                    let parts: Vec<&str> = line.split('=').collect();
                    if parts.len() >= 3 {
                        let avg = parts[2].trim();
                        let num: String = avg
                            .chars()
                            .take_while(|c| c.is_ascii_digit() || *c == '.')
                            .collect();
                        if let Ok(v) = num.parse::<f64>() {
                            return Some(v);
                        }
                    }
                }
            }
            None
        }
        _ => {
            for line in stdout.lines() {
                if line.contains("rtt") || line.contains("round-trip") {
                    if let Some(eq_pos) = line.rfind('=') {
                        let after = line[eq_pos + 1..].trim();
                        let parts: Vec<&str> = after.split('/').collect();
                        if parts.len() >= 4 {
                            if let Ok(v) = parts[1].parse::<f64>() {
                                return Some(v);
                            }
                        }
                    }
                }
            }
            None
        }
    }
}

/// Determine whether an IPv4 string is in a private/RFC1918 range, which usually
/// indicates the local router/ISP is answering DNS (a leak risk for this app).
fn is_private_ipv4(ip: &str) -> bool {
    let parts: Vec<u8> = ip.split('.').filter_map(|p| p.parse::<u8>().ok()).collect();
    if parts.len() != 4 {
        return false;
    }
    match (parts[0], parts[1]) {
        (10, _) => true,
        (127, _) => true,
        (192, 168) => true,
        (172, b) if (16..=31).contains(&b) => true,
        (169, 254) => true,
        _ => false,
    }
}

/// Resolve a hostname through a *specific* DNS server (or the system default
/// when `server` is None), returning whether an A record came back.
async fn resolves_via_server(server: Option<&str>, hostname: &str, os: OsKind) -> bool {
    if crate::validation::validate_host(hostname).is_err() {
        return false;
    }
    let out = match os {
        OsKind::Windows => {
            // nslookup -type=A <host> [server]; short timeout, single retry.
            let mut args: Vec<&str> = vec!["-type=A", "-timeout=3", "-retry=1", hostname];
            if let Some(s) = server {
                args.push(s);
            }
            run_async("nslookup", &args, 12).await
        }
        _ => {
            // dig +short <host> A [@server]
            let at = server.map(|s| format!("@{}", s));
            let mut args: Vec<&str> = vec!["+short", "+time=3", "+tries=1", hostname, "A"];
            if let Some(at) = at.as_deref() {
                args.push(at);
            }
            run_async("dig", &args, 12).await
        }
    };

    match out {
        Ok(o) => match os {
            OsKind::Windows => {
                let lower = o.combined().to_lowercase();
                // SERVFAIL / refused / timeout => not resolved.
                if lower.contains("server failed")
                    || lower.contains("servfail")
                    || lower.contains("query refused")
                    || lower.contains("request to")
                    || lower.contains("timed out")
                    || lower.contains("no response")
                    || lower.contains("non-existent")
                {
                    return false;
                }
                // A successful A lookup prints an "addresses:"/"name:" answer block.
                lower.contains("name:") || lower.contains("addresses:")
            }
            _ => {
                o.stdout
                    .lines()
                    .any(|l| l.trim().parse::<std::net::Ipv4Addr>().is_ok())
            }
        },
        Err(_) => false,
    }
}

/// Real DNSSEC validation probe.
///
/// A validating resolver returns SERVFAIL for `dnssec-failed.org` while still
/// resolving a control domain. If the bad domain resolves, the resolver is not
/// validating. We try the active resolver first, then fall back to the system
/// default resolver so the result is almost always measurable.
pub async fn check_dnssec(server: Option<&str>, os: OsKind) -> Option<bool> {
    // Establish that DNS works at all (control lookup). Try the supplied
    // resolver, then the system default.
    let mut control_ok = resolves_via_server(server, "cloudflare.com", os).await;
    let mut effective = server;
    if !control_ok && server.is_some() {
        control_ok = resolves_via_server(None, "cloudflare.com", os).await;
        effective = None;
    }
    if !control_ok {
        return None;
    }
    let bad_ok = resolves_via_server(effective, "dnssec-failed.org", os).await;
    Some(!bad_ok)
}

/// Real DNS-leak check based on the active adapters' configured resolvers.
///
/// Returns `Some(true)` when every active adapter has explicit public DNS
/// servers, `Some(false)` when any active adapter relies on a private/router
/// resolver or has no DNS configured (queries can leak to the ISP), or `None`
/// when no active adapter information is available.
pub fn check_leak(adapters: &[NetworkAdapter]) -> Option<bool> {
    let active: Vec<&NetworkAdapter> = adapters.iter().filter(|a| a.is_up).collect();
    if active.is_empty() {
        return None;
    }
    for adapter in active {
        if adapter.ipv4_dns.is_empty() {
            return Some(false);
        }
        if adapter.ipv4_dns.iter().any(|ip| is_private_ipv4(ip)) {
            return Some(false);
        }
    }
    Some(true)
}

/// Pick the resolver currently in use (first DNS of the first active adapter).
fn active_resolver(adapters: &[NetworkAdapter]) -> Option<String> {
    adapters
        .iter()
        .find(|a| a.is_up && !a.ipv4_dns.is_empty())
        .and_then(|a| a.ipv4_dns.first().cloned())
        .or_else(|| adapters.iter().find_map(|a| a.ipv4_dns.first().cloned()))
}

/// Run a comprehensive diagnostic check against multiple targets, including
/// real DNSSEC validation and DNS-leak detection.
pub async fn comprehensive_check(
    targets: &[DiagnosticTarget],
    adapters: &[NetworkAdapter],
    caps: &PlatformCapabilities,
) -> AppResult<DiagnosticResult> {
    let mut results = Vec::new();
    let mut total_latency = 0.0;
    let mut count = 0;
    let mut reachable_count = 0;

    // Determine which adapter to pin pings to: the first active adapter with
    // an IPv4 address so that VPN/TUN traffic routes correctly.
    let adapter_id = adapters
        .iter()
        .find(|a| a.is_up && !a.ipv4_dns.is_empty())
        .map(|a| a.id.as_str());

    for target in targets {
        let result = ping_host_on_adapter(&target.host, adapter_id, caps).await;
        if result.reachable {
            if let Some(lat) = result.latency_ms {
                total_latency += lat;
                count += 1;
            }
            reachable_count += 1;
        }
        results.push(result);
    }

    let avg_latency = if count > 0 {
        Some(total_latency / count as f64)
    } else {
        None
    };

    let total = targets.len().max(1) as f64;
    let packet_loss = ((total - reachable_count as f64) / total) * 100.0;

    // Real DNSSEC probe through the resolver currently in use, falling back to
    // the system default resolver when no adapter resolver could be detected.
    let resolver = active_resolver(adapters);
    let dnssec_valid = check_dnssec(resolver.as_deref(), caps.os).await;
    let leak_secure = check_leak(adapters);

    Ok(DiagnosticResult {
        latency_ms: avg_latency,
        dnssec_valid,
        leak_secure,
        packet_loss_percent: packet_loss,
        targets: results,
    })
}

/// Probe a DNS server by pinging it and measuring latency.
pub async fn probe_dns_server(
    server: &str,
    adapter_id: Option<&str>,
    caps: &PlatformCapabilities,
) -> AppResult<DnsMetrics> {
    crate::validation::validate_ipv4(server)?;
    let probe = ping_host_on_adapter(server, adapter_id, caps).await;
    let tested_at = chrono::Utc::now().to_rfc3339();

    Ok(DnsMetrics {
        profile_id: String::new(),
        server: server.to_string(),
        latency_ms: probe.latency_ms,
        reachability_percent: if probe.reachable { 100.0 } else { 0.0 },
        packet_loss_percent: if probe.reachable { 0.0 } else { 100.0 },
        tested_at,
        error: probe.error,
    })
}

/// Resolve a hostname using nslookup/dig to verify DNS is working.
pub async fn resolve_host(
    hostname: &str,
    dns_server: Option<&str>,
    caps: &PlatformCapabilities,
) -> AppResult<Vec<String>> {
    crate::validation::validate_host(hostname)?;

    let mut arg_store: Vec<String> = Vec::new();
    let program: &str;
    match caps.os {
        OsKind::Windows => {
            program = "nslookup";
            arg_store.push(hostname.to_string());
            if let Some(server) = dns_server {
                arg_store.push(server.to_string());
            }
        }
        _ => {
            program = "dig";
            arg_store.push("+short".to_string());
            if let Some(server) = dns_server {
                arg_store.push(format!("@{server}"));
            }
            arg_store.push(hostname.to_string());
        }
    }
    let args: Vec<&str> = arg_store.iter().map(|s| s.as_str()).collect();

    let out = run_async_or_error(program, &args, 10).await?;
    let mut addresses = Vec::new();
    for line in out.stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let addr = if let Some(rest) = line.strip_prefix("Address:") {
            rest.trim()
        } else if line.starts_with("Addresses:") {
            line["Addresses:".len()..].trim()
        } else {
            line
        };
        if addr.parse::<std::net::IpAddr>().is_ok() {
            addresses.push(addr.to_string());
        }
    }
    Ok(addresses)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::NetworkAdapter;

    fn adapter(is_up: bool, dns: &[&str]) -> NetworkAdapter {
        NetworkAdapter {
            id: "test".into(),
            name: "Test".into(),
            description: None,
            is_up,
            is_primary: true,
            ipv4_dns: dns.iter().map(|s| s.to_string()).collect(),
            ipv6_dns: vec![],
        }
    }

    #[test]
    fn detects_private_ipv4_ranges() {
        assert!(is_private_ipv4("10.0.0.1"));
        assert!(is_private_ipv4("192.168.1.1"));
        assert!(is_private_ipv4("172.16.0.1"));
        assert!(is_private_ipv4("172.31.255.255"));
        assert!(is_private_ipv4("127.0.0.1"));
        assert!(!is_private_ipv4("8.8.8.8"));
        assert!(!is_private_ipv4("1.1.1.1"));
        assert!(!is_private_ipv4("172.32.0.1"));
        assert!(!is_private_ipv4("not-an-ip"));
    }

    #[test]
    fn leak_check_flags_router_resolver() {
        // Public resolvers on the only active adapter => secure.
        assert_eq!(check_leak(&[adapter(true, &["1.1.1.1", "8.8.8.8"])]), Some(true));
        // Private resolver => exposed.
        assert_eq!(check_leak(&[adapter(true, &["192.168.1.1"])]), Some(false));
        // No DNS configured => exposed.
        assert_eq!(check_leak(&[adapter(true, &[])]), Some(false));
        // No active adapters => not measurable.
        assert_eq!(check_leak(&[adapter(false, &["1.1.1.1"])]), None);
        assert_eq!(check_leak(&[]), None);
    }

    #[test]
    fn active_resolver_prefers_up_adapter() {
        let adapters = vec![adapter(false, &["9.9.9.9"]), adapter(true, &["1.1.1.1"])];
        assert_eq!(active_resolver(&adapters), Some("1.1.1.1".to_string()));
    }
}
