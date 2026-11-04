use crate::command_runner::{run_async, run_async_or_error, RunOutput};
use crate::errors::AppResult;
use crate::models::{CommandResult, DnsApplyRequest, NetworkAdapter};
use crate::validation::sanitize_output;

/// List network adapters based on the detected backend.
pub async fn list_adapters(backend: &str) -> AppResult<Vec<NetworkAdapter>> {
    match backend {
        "networkmanager" => list_nm_adapters().await,
        "systemd-resolved" => list_resolvectl_adapters().await,
        "resolv.conf" => list_resolv_conf().await,
        _ => Ok(Vec::new()),
    }
}

async fn list_nm_adapters() -> AppResult<Vec<NetworkAdapter>> {
    let out = run_async(
        "nmcli",
        &["-t", "-f", "NAME,UUID,DEVICE,STATE", "con", "show"],
        10,
    )
    .await?;
    if !out.success() {
        return Ok(Vec::new());
    }

    let mut adapters = Vec::new();
    for line in out.stdout.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 4 {
            continue;
        }
        let name = parts[0].to_string();
        let uuid = parts[1].to_string();
        let device = parts[2].to_string();
        let state = parts[3].to_string();
        let is_up = state == "activated";

        let dns_out = run_async(
            "nmcli",
            &["-t", "-f", "IP4.DNS,IP6.DNS", "con", "show", &uuid],
            10,
        )
        .await?;
        let mut ipv4_dns = Vec::new();
        let mut ipv6_dns = Vec::new();
        for d in dns_out.stdout.lines() {
            if let Some((_, val)) = d.split_once(':') {
                for ip in val.split(',') {
                    let ip = ip.trim();
                    if ip.is_empty() {
                        continue;
                    }
                    if ip.contains(':') {
                        ipv6_dns.push(ip.to_string());
                    } else {
                        ipv4_dns.push(ip.to_string());
                    }
                }
            }
        }

        adapters.push(NetworkAdapter {
            id: uuid.clone(),
            name: if !device.is_empty() { device } else { name.clone() },
            description: Some(name),
            is_up,
            is_primary: false,
            ipv4_dns,
            ipv6_dns,
        });
    }

    if let Some(first_up) = adapters.iter_mut().find(|a| a.is_up) {
        first_up.is_primary = true;
    }

    Ok(adapters)
}

async fn list_resolvectl_adapters() -> AppResult<Vec<NetworkAdapter>> {
    let out = run_async("resolvectl", &["status"], 10).await?;
    if !out.success() {
        return Ok(Vec::new());
    }

    let mut adapters = Vec::new();
    let mut current: Option<NetworkAdapter> = None;
    for line in out.stdout.lines() {
        let line = line.trim();
        if line.starts_with("Link ") {
            if let Some(a) = current.take() {
                adapters.push(a);
            }
            let name = line
                .split_whitespace()
                .nth(2)
                .map(|s| s.trim_matches(|c: char| c == '(' || c == ')').to_string())
                .unwrap_or_default();
            current = Some(NetworkAdapter {
                id: name.clone(),
                name,
                description: None,
                is_up: true,
                is_primary: false,
                ipv4_dns: Vec::new(),
                ipv6_dns: Vec::new(),
            });
        } else if line.starts_with("DNS Servers:") {
            if let Some(a) = current.as_mut() {
                let servers: Vec<&str> = line["DNS Servers:".len()..].split_whitespace().collect();
                for s in servers {
                    let s = s.trim();
                    if s.contains(':') {
                        a.ipv6_dns.push(s.to_string());
                    } else {
                        a.ipv4_dns.push(s.to_string());
                    }
                }
            }
        }
    }
    if let Some(a) = current.take() {
        adapters.push(a);
    }

    if let Some(first) = adapters.first_mut() {
        first.is_primary = true;
    }

    Ok(adapters)
}

async fn list_resolv_conf() -> AppResult<Vec<NetworkAdapter>> {
    let content = tokio::fs::read_to_string("/etc/resolv.conf").await.unwrap_or_default();
    let mut ipv4_dns = Vec::new();
    let mut ipv6_dns = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("nameserver") {
            let ip = rest.trim();
            if ip.is_empty() {
                continue;
            }
            if ip.contains(':') {
                ipv6_dns.push(ip.to_string());
            } else {
                ipv4_dns.push(ip.to_string());
            }
        }
    }
    Ok(vec![NetworkAdapter {
        id: "resolv.conf".to_string(),
        name: "System (resolv.conf)".to_string(),
        description: Some("/etc/resolv.conf".to_string()),
        is_up: true,
        is_primary: true,
        ipv4_dns,
        ipv6_dns,
    }])
}

/// Single-quote a value for safe embedding in a `/bin/sh -c` command line.
fn sh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Run a privileged shell command, elevating through pkexec (PolicyKit) when
/// the current process is not already running as root.
async fn run_privileged(command: &str, success_msg: &str) -> AppResult<CommandResult> {
    if crate::command_runner::is_elevated() {
        return match run_async_or_error("sh", &["-c", command], 30).await {
            Ok(_) => Ok(CommandResult::ok(success_msg.to_string())),
            Err(e) => Ok(CommandResult::err(format!(
                "Failed: {}",
                sanitize_output(&e.to_string())
            ))),
        };
    }

    let outcome = crate::elevation::run_argv_elevated("sh", &["-c", command]).await?;
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

/// Apply DNS based on the Linux backend.
pub async fn apply_dns(req: &DnsApplyRequest, backend: &str) -> AppResult<CommandResult> {
    match backend {
        "networkmanager" => apply_nm_dns(req).await,
        "systemd-resolved" => apply_resolvectl_dns(req).await,
        "resolv.conf" => apply_resolv_conf_dns(req).await,
        "resolvconf" => apply_resolvconf_dns(req).await,
        _ => Ok(CommandResult::err(format!("Unsupported Linux DNS backend: {}", backend))),
    }
}

async fn apply_nm_dns(req: &DnsApplyRequest) -> AppResult<CommandResult> {
    let mut dns_servers: Vec<String> = vec![req.primary_ipv4.clone()];
    if let Some(sec) = &req.secondary_ipv4 {
        if !sec.is_empty() {
            dns_servers.push(sec.clone());
        }
    }
    let dns_joined = dns_servers.join(" ");

    // Connection discovery is read-only and runs unprivileged.
    let connections: Vec<String> = match &req.adapter_id {
        Some(id) => vec![id.to_string()],
        None => {
            let out = run_async("nmcli", &["-t", "-f", "UUID", "con", "show", "--active"], 10).await?;
            out.stdout.lines().map(|l| l.trim().to_string()).filter(|s| !s.is_empty()).collect()
        }
    };
    if connections.is_empty() {
        return Ok(CommandResult::err("No active NetworkManager connections found."));
    }

    let mut steps: Vec<String> = Vec::new();
    for conn in &connections {
        steps.push(format!(
            "nmcli con mod {} ipv4.dns {} ipv4.ignore-auto-dns yes",
            sh_quote(conn),
            sh_quote(&dns_joined)
        ));
        if let Some(v6) = &req.primary_ipv6 {
            if !v6.is_empty() {
                let mut v6s = vec![v6.clone()];
                if let Some(sec) = &req.secondary_ipv6 {
                    if !sec.is_empty() {
                        v6s.push(sec.clone());
                    }
                }
                steps.push(format!(
                    "nmcli con mod {} ipv6.dns {} ipv6.ignore-auto-dns yes",
                    sh_quote(conn),
                    sh_quote(&v6s.join(" "))
                ));
            }
        }
        steps.push(format!("nmcli con up {}", sh_quote(conn)));
    }

    run_privileged(&steps.join(" && "), "DNS applied successfully.").await
}

async fn apply_resolvectl_dns(req: &DnsApplyRequest) -> AppResult<CommandResult> {
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

    let interface = match &req.adapter_id {
        Some(id) => id.to_string(),
        None => {
            let out = run_async("ip", &["route", "show", "default"], 5).await?;
            out.stdout
                .lines()
                .next()
                .and_then(|l| {
                    let parts: Vec<&str> = l.split_whitespace().collect();
                    let pos = parts.iter().position(|p| *p == "dev")?;
                    parts.get(pos + 1).map(|s| s.to_string())
                })
                .unwrap_or_else(|| "lo".to_string())
        }
    };

    let server_args = servers.iter().map(|s| sh_quote(s)).collect::<Vec<_>>().join(" ");
    let command = format!("resolvectl dns {} {}", sh_quote(&interface), server_args);
    run_privileged(&command, "DNS applied successfully.").await
}

async fn apply_resolv_conf_dns(req: &DnsApplyRequest) -> AppResult<CommandResult> {
    let mut lines: Vec<String> = vec!["# Generated by SparkDns".to_string()];
    lines.push(format!("nameserver {}", req.primary_ipv4));
    if let Some(sec) = &req.secondary_ipv4 {
        if !sec.is_empty() {
            lines.push(format!("nameserver {}", sec));
        }
    }
    if let Some(v6) = &req.primary_ipv6 {
        if !v6.is_empty() {
            lines.push(format!("nameserver {}", v6));
        }
    }
    if let Some(v6) = &req.secondary_ipv6 {
        if !v6.is_empty() {
            lines.push(format!("nameserver {}", v6));
        }
    }
    let content = format!("{}\n", lines.join("\n"));
    // `tee` lets the elevated child write the protected file.
    let command = format!("printf %s {} | tee /etc/resolv.conf > /dev/null", sh_quote(&content));
    run_privileged(&command, "DNS written to /etc/resolv.conf.").await
}

async fn apply_resolvconf_dns(req: &DnsApplyRequest) -> AppResult<CommandResult> {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("nameserver {}", req.primary_ipv4));
    if let Some(sec) = &req.secondary_ipv4 {
        if !sec.is_empty() {
            lines.push(format!("nameserver {}", sec));
        }
    }
    if let Some(v6) = &req.primary_ipv6 {
        if !v6.is_empty() {
            lines.push(format!("nameserver {}", v6));
        }
    }
    let content = format!("{}\n", lines.join("\n"));
    let command = format!("printf %s {} | resolvconf -a sparkdns", sh_quote(&content));
    run_privileged(&command, "DNS applied via resolvconf.").await
}


/// Clear DNS based on the Linux backend.
pub async fn clear_dns(adapter_id: Option<&str>, backend: &str) -> AppResult<CommandResult> {
    match backend {
        "networkmanager" => clear_nm_dns(adapter_id).await,
        "systemd-resolved" => clear_resolvectl_dns(adapter_id).await,
        "resolv.conf" => clear_resolv_conf_dns().await,
        "resolvconf" => clear_resolvconf_dns().await,
        _ => Ok(CommandResult::err(format!("Unsupported Linux DNS backend: {}", backend))),
    }
}

async fn clear_nm_dns(adapter_id: Option<&str>) -> AppResult<CommandResult> {
    let connections: Vec<String> = match adapter_id {
        Some(id) => vec![id.to_string()],
        None => {
            let out = run_async("nmcli", &["-t", "-f", "UUID", "con", "show", "--active"], 10).await?;
            out.stdout.lines().map(|l| l.trim().to_string()).filter(|s| !s.is_empty()).collect()
        }
    };
    if connections.is_empty() {
        return Ok(CommandResult::err("No active NetworkManager connections found."));
    }

    let mut steps: Vec<String> = Vec::new();
    for conn in &connections {
        steps.push(format!("nmcli con mod {} ipv4.dns '' ipv4.ignore-auto-dns no", sh_quote(conn)));
        steps.push(format!("nmcli con mod {} ipv6.dns '' ipv6.ignore-auto-dns no", sh_quote(conn)));
        steps.push(format!("nmcli con up {}", sh_quote(conn)));
    }
    run_privileged(&steps.join(" && "), "DNS cleared and reset to automatic.").await
}

async fn clear_resolvectl_dns(adapter_id: Option<&str>) -> AppResult<CommandResult> {
    let interface = adapter_id.unwrap_or("lo");
    let command = format!("resolvectl revert {}", sh_quote(interface));
    run_privileged(&command, &format!("DNS reverted on {}.", interface)).await
}

async fn clear_resolv_conf_dns() -> AppResult<CommandResult> {
    let command = "printf %s '# Generated by SparkDns - cleared\n' | tee /etc/resolv.conf > /dev/null";
    run_privileged(command, "resolv.conf cleared.").await
}

async fn clear_resolvconf_dns() -> AppResult<CommandResult> {
    run_privileged("resolvconf -d sparkdns", "resolvconf entry removed.").await
}

/// Flush DNS cache. On Linux this depends on the resolver.
pub async fn flush_cache(backend: &str) -> AppResult<CommandResult> {
    match backend {
        "systemd-resolved" => {
            match run_async_or_error("resolvectl", &["flush-caches"], 10).await {
                Ok(_) => Ok(CommandResult::ok("DNS cache flushed.")),
                Err(e) => Ok(CommandResult::err(format!("Failed to flush cache: {}", sanitize_output(&e.to_string())))),
            }
        }
        "networkmanager" => {
            let _ = run_async_or_error("nscd", &["-i", "hosts"], 5).await;
            Ok(CommandResult::ok("DNS cache flush attempted."))
        }
        _ => {
            if crate::command_runner::which_exists("nscd") {
                let _ = run_async_or_error("nscd", &["-i", "hosts"], 5).await;
                Ok(CommandResult::ok("DNS cache flushed via nscd."))
            } else {
                Ok(CommandResult::ok("No DNS cache flush mechanism detected."))
            }
        }
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