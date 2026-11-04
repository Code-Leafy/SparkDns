use crate::errors::{AppError, AppResult};
use crate::validation::sanitize_output;
use std::process::Command;
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(windows)]
fn hide_sync_window(command: &mut Command) {
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_sync_window(_: &mut Command) {}

#[cfg(windows)]
fn hide_async_window(command: &mut TokioCommand) {
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_async_window(_: &mut TokioCommand) {}

/// Check if a binary exists on PATH using the `which` crate.
pub fn which_exists(name: &str) -> bool {
    which::which(name).is_ok()
}

/// Output of a command run.
#[derive(Debug, Clone)]
pub struct RunOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl RunOutput {
    pub fn success(&self) -> bool {
        self.status == 0
    }

    pub fn combined(&self) -> String {
        let mut s = String::new();
        if !self.stdout.is_empty() {
            s.push_str(&self.stdout);
        }
        if !self.stderr.is_empty() {
            if !s.is_empty() {
                s.push('\n');
            }
            s.push_str(&self.stderr);
        }
        s
    }

    pub fn sanitized_combined(&self) -> String {
        sanitize_output(&self.combined())
    }
}

/// Synchronously run a command, capturing output. Always passes args as argv (no shell).
pub fn run_sync(program: &str, args: &[&str]) -> AppResult<RunOutput> {
    let mut command = Command::new(program);
    command.args(args);
    hide_sync_window(&mut command);

    let output = command
        .output()
        .map_err(|e| AppError::Command(format!("Failed to execute '{}': {}", program, e)))?;

    let status = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok(RunOutput { status, stdout, stderr })
}

/// Asynchronously run a command with a timeout (seconds). No shell, args as argv.
pub async fn run_async(program: &str, args: &[&str], timeout_secs: u64) -> AppResult<RunOutput> {
    let mut cmd = TokioCommand::new(program);
    cmd.args(args);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    hide_async_window(&mut cmd);

    let dur = Duration::from_secs(timeout_secs);
    let child = cmd
        .spawn()
        .map_err(|e| AppError::Command(format!("Failed to spawn '{}': {}", program, e)))?;

    let output = timeout(dur, child.wait_with_output())
        .await
        .map_err(|_| AppError::Timeout(format!("Timed out waiting for '{}'", program)))?
        .map_err(|e| AppError::Command(format!("Failed to wait for '{}': {}", program, e)))?;

    let status = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok(RunOutput { status, stdout, stderr })
}

/// Run a command and return an error if it fails, including sanitized stderr/stdout in the message.
pub async fn run_async_or_error(
    program: &str,
    args: &[&str],
    timeout_secs: u64,
) -> AppResult<RunOutput> {
    let out = run_async(program, args, timeout_secs).await?;
    if !out.success() {
        return Err(AppError::Command(format!(
            "'{}' exited with status {}: {}",
            program,
            out.status,
            sanitize_output(&out.combined())
        )));
    }
    Ok(out)
}

/// Check whether the current process is running with elevated/admin/root privileges.
pub fn is_elevated() -> bool {
    #[cfg(target_os = "windows")]
    {
        run_sync("net", &["session"]).map(|o| o.success()).unwrap_or(false)
    }
    #[cfg(unix)]
    {
        run_sync("id", &["-u"])
            .map(|o| o.stdout.trim() == "0")
            .unwrap_or(false)
    }
}