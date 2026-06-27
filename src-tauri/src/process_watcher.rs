//! Process-watcher for the auto-switch DNS feature.
//!
//! The watcher polls the OS for running processes on a short interval, then
//! applies or reverts DNS whenever a watched app's state changes:
//!
//! * When a watched app starts running, switch the system DNS to that rule's
//!   profile and remember the previous profile so we can restore it later.
//! * When the last watched app exits, revert DNS to the remembered previous
//!   profile (or to DHCP if there wasn't one).
//!
//! On each OS, process listing uses a tool that ships with the platform so we
//! don't add any heavy dependencies. Output is sanitized before parsing.

use std::collections::HashSet;
use std::time::Duration;

use crate::command_runner::run_async;
use crate::models::{AutoSwitchRule, RunningProcess};
use crate::platform::detect_os;
#[cfg(target_os = "macos")]
use crate::validation::sanitize_output;

/// Polling interval for the watcher loop.
pub const POLL_INTERVAL_SECS: u64 = 4;

/// Cap the per-platform command's own timeout.
#[allow(dead_code)]
const POLL_CMD_TIMEOUT_SECS: u64 = 10;

/// What the watcher did on a poll tick (used by tests + the UI log).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatcherDecision {
    /// No watched app is running, nothing to do.
    Idle,
    /// A watched app appeared; switch DNS to the named profile id.
    Switched(String),
    /// Last watched app exited; reverted to a previously-active profile id.
    Reverted(Option<String>),
    /// Process listing failed; the UI may want to surface this.
    #[allow(dead_code)]
    ListError(String),
}

/// List running processes, with a normalized name (lowercase file name) and,
/// when available, the full executable path.
pub async fn list_running_processes() -> Vec<RunningProcess> {
    let raw = match detect_os() {
        #[cfg(windows)]
        crate::models::OsKind::Windows => list_processes_windows().await,
        #[cfg(target_os = "macos")]
        crate::models::OsKind::Macos => list_processes_macos().await,
        #[cfg(target_os = "linux")]
        crate::models::OsKind::Linux => list_processes_linux().await,
        #[allow(unreachable_patterns)]
        _ => String::new(),
    };
    parse_processes(&raw)
}

/// Parse a multi-line process listing into normalized `RunningProcess` records.
///
/// Each line is expected to be `<name>` or `<name>|<path>`. Names are
/// lowercased, paths preserved as-is. Empty lines and unreadable entries are
/// skipped. The parser is platform-agnostic so the unit tests can run anywhere.
pub fn parse_processes(raw: &str) -> Vec<RunningProcess> {
    let mut out = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (name_part, path_part) = match line.split_once('|') {
            Some((n, p)) => (n.trim(), Some(p.trim())),
            None => (line, None),
        };
        if name_part.is_empty() {
            continue;
        }
        // Strip surrounding quotes Windows sometimes uses for paths.
        let cleaned = name_part.trim_matches('"').to_string();
        let lower = cleaned.to_lowercase();
        if !seen.insert(lower.clone()) {
            continue;
        }
        let path = path_part
            .map(|p| p.trim_matches('"').to_string())
            .filter(|p| !p.is_empty());
        out.push(RunningProcess { name: lower, path });
    }
    out
}

/// Given a set of running process names (lowercased) and a list of rules,
/// return the first rule whose `match_name` is present in the running set.
/// Disabled rules are skipped.
pub fn matching_rule<'a>(
    rules: &'a [AutoSwitchRule],
    running: &HashSet<String>,
) -> Option<&'a AutoSwitchRule> {
    rules
        .iter()
        .find(|r| r.enabled && running.contains(&r.match_name.to_lowercase()))
}
/// Decide what the watcher should do this tick, given the running set and
/// the previously-active profile id (so we can revert to it later).
///
/// * If no enabled rule matches the running set, the decision is `Idle`.
/// * If a rule matches and the previous decision was for a *different*
///   profile id (or none), we should `Switched`.
/// * If the rule that was previously switching is no longer present, but
///   another rule's app is still running, we may stay on whichever profile
///   the running rules collectively point to.
pub fn decide(
    rules: &[AutoSwitchRule],
    running: &HashSet<String>,
    _current_active: Option<&str>,
    last_switched_to: Option<&str>,
) -> WatcherDecision {
    let matched = matching_rule(rules, running);
    match matched {
        Some(rule) => {
            // A watched app is running. Emit Switched unless we're already on
            // that profile (to avoid repeated Switched events for the same profile).
            if last_switched_to == Some(rule.profile_id.as_str()) {
                WatcherDecision::Idle
            } else {
                WatcherDecision::Switched(rule.profile_id.clone())
            }
        }
        None => {
            // No watched app running. If we were previously auto-switched to a
            // profile, emit Reverted to signal the app to clear DNS.
            if last_switched_to.is_some() {
                WatcherDecision::Reverted(None)
            } else {
                WatcherDecision::Idle
            }
        }
    }
}

/// Lowercase + trim a user-supplied executable name (or path) to the matcher
/// form: just the file name, in lowercase.
pub fn normalize_match_name(input: &str) -> String {
    let trimmed = input.trim().trim_matches('"');
    let file_name = std::path::Path::new(trimmed)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(trimmed);
    file_name.to_lowercase()
}

// ---------------------------------------------------------------------------
// Per-platform process listing
// ---------------------------------------------------------------------------

#[cfg(windows)]
async fn list_processes_windows() -> String {
    // `Get-Process` returns ProcessName; `MainModule.FileName` adds the full
    // path. We emit "name|path" lines, falling back to "name" when the module
    // can't be read (access denied for some system processes).
    let script = r#"
$ErrorActionPreference = 'SilentlyContinue'
Get-Process | ForEach-Object {
    $name = $_.ProcessName
    $path = $null
    try { $path = $_.MainModule.FileName } catch {}
    if ($path) { Write-Output ("{0}|{1}" -f $name, $path) }
    else { Write-Output $name }
}
"#;
    match run_async(
        "powershell",
        &["-NoProfile", "-NonInteractive", "-Command", script],
        POLL_CMD_TIMEOUT_SECS,
    )
    .await
    {
        Ok(out) => out.combined(),
        Err(_) => String::new(),
    }
}

#[cfg(target_os = "macos")]
async fn list_processes_macos() -> String {
    // `ps -axo comm=` gives the executable path of every process. We take the
    // basename as the match name and the full comm path as the path.
    let script = r#"ps -axo comm= 2>/dev/null | awk 'NF{n=split($0,p,"/"); base=p[n]; if(base!=""){print tolower(base) "|" $0}}' | awk '!seen[$0]++'"#;
    match run_async("/bin/sh", &["-c", script], POLL_CMD_TIMEOUT_SECS).await {
        Ok(out) => sanitize_output(&out.combined()),
        Err(_) => String::new(),
    }
}

#[cfg(target_os = "linux")]
async fn list_processes_linux() -> String {
    // Walk /proc, reading each process's comm and exe symlink. The exe
    // symlink fails for kernel threads and unowned processes; skip silently.
    let mut out = String::new();
    let entries = match std::fs::read_dir("/proc") {
        Ok(e) => e,
        Err(_) => return out,
    };
    let mut seen: HashSet<String> = HashSet::new();
    for entry in entries.flatten() {
        let name = match entry.file_name().into_string() {
            Ok(s) => s,
            Err(_) => continue,
        };
        if !name.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let pid_path = entry.path();
        let comm = match std::fs::read_to_string(pid_path.join("comm")) {
            Ok(s) => s.trim().to_string(),
            Err(_) => continue,
        };
        if comm.is_empty() || !seen.insert(comm.to_lowercase()) {
            continue;
        }
        let exe = std::fs::read_link(pid_path.join("exe"))
            .ok()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        if exe.is_empty() {
            out.push_str(&comm);
        } else {
            out.push_str(&comm);
            out.push('|');
            out.push_str(&exe);
        }
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// Convenience helpers used by the background loop
// ---------------------------------------------------------------------------

/// Run a poll tick: list processes, run the decision function.
pub async fn poll_once(
    rules: &[AutoSwitchRule],
    previous_active: Option<&str>,
    last_switched_to: Option<&str>,
) -> (WatcherDecision, HashSet<String>) {
    let procs = list_running_processes().await;
    let running: HashSet<String> = procs.into_iter().map(|p| p.name).collect();
    let decision = decide(rules, &running, previous_active, last_switched_to);
    (decision, running)
}

/// Sleep for the standard poll interval. Split out so the loop reads cleanly.
pub async fn poll_sleep() {
    tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
}


#[cfg(test)]
mod tests {
    use super::*;

    fn rule(name: &str, profile: &str, enabled: bool) -> AutoSwitchRule {
        AutoSwitchRule {
            id: format!("r_{}", name),
            app_label: name.to_string(),
            match_name: name.to_lowercase(),
            app_path: None,
            profile_id: profile.to_string(),
            enabled,
        }
    }

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_processes_dedupes_and_lowercases() {
        let raw = "Chrome.EXE|\"C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe\"\n\
                   chrome.exe|duplicate\n\
                   \n\
                   firefox|some/path\n\
                   |empty-name\n";
        let procs = parse_processes(raw);
        let names: Vec<&str> = procs.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["chrome.exe", "firefox"]);
        assert_eq!(
            procs[0].path.as_deref(),
            Some("C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe")
        );
    }

    #[test]
    fn normalize_match_name_handles_paths_and_case() {
        assert_eq!(normalize_match_name("C:\\Games\\steam.exe"), "steam.exe");
        assert_eq!(
            normalize_match_name("/Applications/Steam.app/Contents/MacOS/steam"),
            "steam"
        );
        assert_eq!(normalize_match_name("CHROME.EXE"), "chrome.exe");
        assert_eq!(normalize_match_name("\"weird name.exe\""), "weird name.exe");
    }

    #[test]
    fn matching_rule_skips_disabled() {
        let rules = vec![rule("steam", "p1", false), rule("chrome", "p2", true)];
        let running = set(&["chrome", "steam"]);
        let m = matching_rule(&rules, &running).unwrap();
        assert_eq!(m.profile_id, "p2");
    }

    #[test]
    fn matching_rule_returns_none_when_no_match() {
        let rules = vec![rule("steam", "p1", true)];
        let running = set(&["chrome"]);
        assert!(matching_rule(&rules, &running).is_none());
    }

    #[test]
    fn decide_switches_when_new_match_appears() {
        let rules = vec![rule("steam", "p_steam", true)];
        let running = set(&["steam", "chrome"]);
        let d = decide(&rules, &running, Some("p_old"), None);
        assert_eq!(d, WatcherDecision::Switched("p_steam".into()));
    }

    #[test]
    fn decide_idle_when_already_on_that_profile() {
        let rules = vec![rule("steam", "p_steam", true)];
        let running = set(&["steam"]);
        let d = decide(&rules, &running, Some("p_steam"), Some("p_steam"));
        assert_eq!(d, WatcherDecision::Idle);
    }

    #[test]
    fn decide_reverts_when_last_watched_app_exits() {
        let rules = vec![rule("steam", "p_steam", true)];
        let running = set(&["chrome"]);
        let d = decide(&rules, &running, Some("p_steam"), Some("p_steam"));
        assert_eq!(d, WatcherDecision::Reverted(None));
    }

    #[test]
    fn decide_idle_when_nothing_watched_was_active() {
        let rules = vec![rule("steam", "p_steam", true)];
        let running = set(&["chrome"]);
        let d = decide(&rules, &running, Some("p_old"), None);
        assert_eq!(d, WatcherDecision::Idle);
    }
}

