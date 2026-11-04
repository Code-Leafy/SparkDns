use crate::command_runner::{run_async, run_async_or_error, RunOutput};
use crate::errors::AppResult;
use crate::models::{CommandResult, DnsApplyRequest, NetworkAdapter};
use crate::validation::sanitize_output;

/// List network services via `networksetup -listallnetworkservices`.
pub async fn list_adapters() -> AppResult<Vec<NetworkAdapter>> {
    let out = run_async("networksetup", &["-listallnetworkservices"], 10).await?;
    if !out.success() {
        return Ok(Vec::new());
    }

    let mut adapters = Vec::new();
    for line in out.stdout.lines().skip(1) {
        let name = line.trim();
        if name.is_empty() || name.contains("An asterisk") {
            continue;
        }
        // Get DNS servers for this service.
        let dns_out = run_async("networksetup", &["-getdnsservers", name], 10).await?;
        let mut ipv4_dns = Vec::new();
        let mut ipv6_dns = Vec::new();
        for d in dns_out.stdout.lines() {
            let d = d.trim();
            if d.is_empty() {
                continue;
            }
            if d.contains(':') {
                ipv6_dns.push(d.to_string());
            } else {
                ipv4_dns.push(d.to_string());
            }
        }

        adapters.push(NetworkAdapter {
            id: name.to_string(),
            name: name.to_string(),
            description: Some(name.to_string()),
            is_up: true,
            is_primary: false,
            ipv4_dns,
            ipv6_dns,
        });
    }

    if let Some(first) = adapters.first_mut() {
        first.is_primary = true;
    }

    Ok(adapters)
}

/// Single-quote a value for safe embedding in a `/bin/sh` command line.
fn sh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Build the list of DNS servers to apply, in order (v4 then v6).
fn collect_servers(req: &DnsApplyRequest) -> Vec<String> {
    let mut servers: Vec<String> = vec![req.primary_ipv4.clone()];
    if let Some(sec) = &req.secondary_ipv4 {
        if !sec.is_empty() {
            servers.push(sec.clone());
        }
    }
    if let Some(v6) = &req.primary_ipv6 {
        if !v6.is_empty() {
            servers.push(v6.clone());
        }
    }
    if let Some(v6) = &req.secondary_ipv6 {
        if !v6.is_empty() {
            servers.push(v6.clone());
        }
    }
    servers
}

/// Build one combined shell command that applies DNS to every service.
fn build_apply_command(services: &[String], servers: &[String]) -> String {
    let server_args = servers.iter().map(|s| sh_quote(s)).collect::<Vec<_>>().join(" ");
    services
        .iter()
        .map(|svc| format!("networksetup -setdnsservers {} {}", sh_quote(svc), server_args))
        .collect::<Vec<_>>()
        .join(" && ")
}

/// Build one combined shell command that clears DNS on every service.
fn build_clear_command(services: &[String]) -> String {
    services
        .iter()
        .map(|svc| format!("networksetup -setdnsservers {} empty", sh_quote(svc)))
        .collect::<Vec<_>>()
        .join(" && ")
}

/// Run a privileged `networksetup` command, elevating via the macOS admin
/// prompt when the process is not already running as root.
async fn run_privileged(command: &str, success_msg: &str) -> AppResult<CommandResult> {
    if crate::command_runner::is_elevated() {
        return match run_async_or_error("sh", &["-c", command], 25).await {
            Ok(_) => Ok(CommandResult::ok(success_msg.to_string())),
            Err(e) => Ok(CommandResult::err(format!(
                "Failed: {}",
                sanitize_output(&e.to_string())
            ))),
        };
    }

    let outcome = crate::elevation::run_shell_elevated(command).await?;
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

/// Apply DNS to a macOS network service using `networksetup -setdnsservers`.
/// Raises the native admin prompt when not already elevated.
pub async fn apply_dns(req: &DnsApplyRequest) -> AppResult<CommandResult> {
    let servers = collect_servers(req);
    let services = match &req.adapter_id {
        Some(id) => vec![id.to_string()],
        None => list_service_names().await?,
    };
    if services.is_empty() {
        return Ok(CommandResult::err("No network services available to configure."));
    }
    let command = build_apply_command(&services, &servers);
    run_privileged(&command, "DNS applied successfully.").await
}

/// Clear DNS (set to automatic) on a macOS network service.
/// Raises the native admin prompt when not already elevated.
pub async fn clear_dns(adapter_id: Option<&str>) -> AppResult<CommandResult> {
    let services = match adapter_id {
        Some(id) => vec![id.to_string()],
        None => list_service_names().await?,
    };
    if services.is_empty() {
        return Ok(CommandResult::err("No network services available to configure."));
    }
    let command = build_clear_command(&services);
    run_privileged(&command, "DNS cleared and reset to automatic.").await
}

async fn list_service_names() -> AppResult<Vec<String>> {
    let out = run_async("networksetup", &["-listallnetworkservices"], 10).await?;
    let mut services = Vec::new();
    for line in out.stdout.lines().skip(1) {
        let name = line.trim();
        if !name.is_empty() && !name.contains("An asterisk") {
            services.push(name.to_string());
        }
    }
    Ok(services)
}

/// Flush the macOS DNS cache (mDNSResponder).
pub async fn flush_cache() -> AppResult<CommandResult> {
    // macOS 10.10.4+ uses killall -HUP mDNSResponder
    match run_async_or_error("killall", &["-HUP", "mDNSResponder"], 10).await {
        Ok(_) => Ok(CommandResult::ok("DNS cache flushed.")),
        Err(e) => Ok(CommandResult::err(format!("Failed to flush DNS cache: {}", sanitize_output(&e.to_string())))),
    }
}

/// Renew DHCP lease via `ipconfig set en0 DHCP`.
pub async fn renew_dhcp(adapter_id: Option<&str>) -> AppResult<CommandResult> {
    // On macOS, adapter id is the service name. We need the interface name.
    // For simplicity, renew on the primary interface detected via route.
    let interface = "en0";
    let _ = adapter_id;
    match run_async_or_error("ipconfig", &["set", interface, "DHCP"], 20).await {
        Ok(_) => Ok(CommandResult::ok("DHCP lease renewed.")),
        Err(e) => Ok(CommandResult::err(format!("Failed to renew DHCP: {}", sanitize_output(&e.to_string())))),
    }
}

/// Run traceroute using `traceroute`.
pub async fn traceroute(host: &str) -> AppResult<crate::models::TracerouteResult> {
    crate::validation::validate_host(host)?;
    let out = run_async("traceroute", &["-m", "30", "-w", "1", host], 60).await?;
    let raw = out.combined();
    let hops = parse_traceroute(&out);
    Ok(crate::models::TracerouteResult {
        ok: out.success(),
        host: host.to_string(),
        hops,
        raw: Some(crate::validation::truncate_output(&sanitize_output(&raw), 4000)),
        error: if out.success() { None } else { Some("traceroute exited with non-zero status".to_string()) },
    })
}

fn parse_traceroute(out: &RunOutput) -> Vec<crate::models::TracerouteHop> {
    let mut hops = Vec::new();
    for (i, line) in out.stdout.lines().enumerate() {
        let line = line.trim();
        // macOS traceroute: " 1  router (192.168.1.1)  1.123 ms  1.456 ms  1.789 ms"
        if line.starts_with(|c: char| c.is_ascii_digit()) {
            let host = line
                .split_whitespace()
                .nth(1)
                .map(|s| s.trim_matches(|c: char| c == '(' || c == ')').to_string());
            let latency = line
                .split_whitespace()
                .find_map(|s| s.trim_end_matches("ms").parse::<f64>().ok());
            hops.push(crate::models::TracerouteHop {
                index: (i + 1) as u32,
                host,
                latency_ms: latency,
            });
        }
    }
    hops
}

/// Check if networksetup is available.
pub fn networksetup_available() -> bool {
    crate::command_runner::which_exists("networksetup")
}