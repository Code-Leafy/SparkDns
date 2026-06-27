//! Cross-platform privilege elevation.
//!
//! Each platform raises the *native* OS authentication prompt for a single
//! privileged operation (least-privilege, per-action) instead of relaunching
//! the whole application elevated:
//!
//! * Windows: `Start-Process -Verb RunAs` triggers the UAC consent dialog.
//! * macOS:   `osascript ... with administrator privileges` shows the native
//!   admin password prompt.
//! * Linux:   `pkexec` shows the PolicyKit graphical authentication dialog.
//!
//! All commands are passed as fixed argv / validated payloads. Callers must
//! validate any user-derived values (IP addresses, hostnames, service names)
//! before building the privileged command.

use crate::command_runner::run_async;
use crate::errors::AppResult;
use crate::validation::sanitize_output;

/// Result of an attempt to run a privileged command.
#[derive(Debug, Clone)]
pub struct ElevatedOutcome {
    /// The privileged command completed successfully.
    pub success: bool,
    /// The user dismissed/cancelled the elevation prompt.
    pub cancelled: bool,
    /// Human-readable message (error detail when `success` is false).
    pub message: String,
}

impl ElevatedOutcome {
    fn ok() -> Self {
        Self { success: true, cancelled: false, message: String::new() }
    }
    fn cancelled() -> Self {
        Self {
            success: false,
            cancelled: true,
            message: "Elevation request was cancelled.".to_string(),
        }
    }
    fn failed(message: impl Into<String>) -> Self {
        Self { success: false, cancelled: false, message: message.into() }
    }
}

/// Windows error code returned when the user declines the UAC prompt.
#[cfg_attr(not(windows), allow(dead_code))]
const ERROR_CANCELLED: i32 = 1223;

/// Wrap a PowerShell body in a try/catch that yields deterministic exit codes.
#[cfg_attr(not(windows), allow(dead_code))]
fn wrap_ps_body(body: &str) -> String {
    format!(
        "$ErrorActionPreference='Stop';\ntry {{\n{}\nexit 0\n}} catch {{ exit 1 }}",
        body
    )
}

/// Run a PowerShell script with administrator rights, raising the UAC prompt.
///
/// The script body is written to a temp file and executed by an elevated
/// PowerShell child via `Start-Process -Verb RunAs`. We wait for completion and
/// propagate the child's exit code so success/failure is accurate.
#[cfg_attr(not(windows), allow(dead_code))]
pub async fn run_powershell_elevated(body: &str) -> AppResult<ElevatedOutcome> {
    use std::io::Write;

    let script = wrap_ps_body(body);

    let mut path = std::env::temp_dir();
    path.push(format!("sparkdns_elev_{}_{}.ps1", std::process::id(), now_millis()));
    {
        let mut file = std::fs::File::create(&path)?;
        // UTF-8 BOM so PowerShell reads non-ASCII correctly.
        file.write_all(&[0xEF, 0xBB, 0xBF])?;
        file.write_all(script.as_bytes())?;
    }
    let path_str = path.to_string_lossy().replace('\'', "''");

    // Outer (non-elevated) launcher: start the elevated child, wait, exit with
    // its code. A declined UAC prompt makes Start-Process throw; we map that to
    // ERROR_CANCELLED (1223).
    let launcher = format!(
        "$ErrorActionPreference='Stop';\ntry {{ $p = Start-Process -FilePath 'powershell' -ArgumentList @('-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-WindowStyle','Hidden','-File','{path}') -Verb RunAs -WindowStyle Hidden -PassThru -Wait; exit $p.ExitCode }} catch {{ exit {cancel} }}",
        path = path_str,
        cancel = ERROR_CANCELLED
    );

    let out = run_async(
        "powershell",
        &["-NoProfile", "-NonInteractive", "-Command", &launcher],
        120,
    )
    .await;

    let _ = std::fs::remove_file(&path);

    match out {
        Ok(o) => {
            if o.status == ERROR_CANCELLED {
                Ok(ElevatedOutcome::cancelled())
            } else if o.success() {
                Ok(ElevatedOutcome::ok())
            } else {
                Ok(ElevatedOutcome::failed(format!(
                    "Elevated command failed (exit {}).",
                    o.status
                )))
            }
        }
        Err(e) => Ok(ElevatedOutcome::failed(sanitize_output(&e.to_string()))),
    }
}


/// Run a shell command with administrator rights on macOS via AppleScript,
/// which shows the native admin authentication dialog.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub async fn run_shell_elevated(command: &str) -> AppResult<ElevatedOutcome> {
    // Escape for embedding inside an AppleScript double-quoted string.
    let escaped = command.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!("do shell script \"{}\" with administrator privileges", escaped);

    let out = run_async("osascript", &["-e", &script], 120).await;
    match out {
        Ok(o) => {
            if o.success() {
                Ok(ElevatedOutcome::ok())
            } else {
                let combined = o.combined();
                // osascript returns -128 / "User canceled" when the prompt is dismissed.
                if combined.contains("-128") || combined.to_lowercase().contains("cancel") {
                    Ok(ElevatedOutcome::cancelled())
                } else {
                    Ok(ElevatedOutcome::failed(sanitize_output(&combined)))
                }
            }
        }
        Err(e) => Ok(ElevatedOutcome::failed(sanitize_output(&e.to_string()))),
    }
}

/// Run a command with root rights on Linux via `pkexec` (PolicyKit), which
/// shows the graphical authentication dialog in a desktop session.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub async fn run_argv_elevated(program: &str, args: &[&str]) -> AppResult<ElevatedOutcome> {
    if !crate::command_runner::which_exists("pkexec") {
        return Ok(ElevatedOutcome::failed(
            "pkexec (PolicyKit) is required to authorize this change but was not found.",
        ));
    }

    let mut pk_args: Vec<&str> = vec![program];
    pk_args.extend_from_slice(args);

    let out = run_async("pkexec", &pk_args, 120).await;
    match out {
        Ok(o) => {
            if o.success() {
                Ok(ElevatedOutcome::ok())
            } else if o.status == 126 || o.status == 127 {
                // 126 = authorization dialog dismissed, 127 = not authorized.
                Ok(ElevatedOutcome::cancelled())
            } else {
                Ok(ElevatedOutcome::failed(sanitize_output(&o.combined())))
            }
        }
        Err(e) => Ok(ElevatedOutcome::failed(sanitize_output(&e.to_string()))),
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[cfg(windows)]
    #[test]
    fn wrap_ps_body_has_trycatch_and_exit_codes() {
        let wrapped = wrap_ps_body("Set-DnsClientServerAddress -InterfaceIndex 1");
        assert!(wrapped.contains("try {"));
        assert!(wrapped.contains("exit 0"));
        assert!(wrapped.contains("exit 1"));
        assert!(wrapped.contains("$ErrorActionPreference='Stop'"));
    }

    #[test]
    fn outcome_states_are_distinct() {
        let ok = ElevatedOutcome::ok();
        assert!(ok.success && !ok.cancelled);
        let cancelled = ElevatedOutcome::cancelled();
        assert!(!cancelled.success && cancelled.cancelled);
        let failed = ElevatedOutcome::failed("boom");
        assert!(!failed.success && !failed.cancelled);
        assert_eq!(failed.message, "boom");
    }
}
