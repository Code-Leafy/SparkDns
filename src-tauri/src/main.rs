#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use sparkdns_lib::AppState;

mod commands {
    use sparkdns_lib::models::{
        AppConfig, AppRule, AppSettings, CommandResult, DnsApplyRequest, DnsMetrics,
        DnsProfile, DiagnosticResult, NetworkAdapter, PlatformCapabilities, TargetProbeResult,
        AutoSwitchRule, RunningProcess,

        TracerouteResult,
    };
    use sparkdns_lib::{self, AppState};

    #[tauri::command]
    pub fn get_capabilities(state: tauri::State<'_, AppState>) -> PlatformCapabilities {
        sparkdns_lib::get_capabilities(state)
    }

    #[tauri::command]
    pub fn get_config(state: tauri::State<'_, AppState>) -> AppConfig {
        sparkdns_lib::get_config(state)
    }

    #[tauri::command]
    pub fn save_config(
        state: tauri::State<'_, AppState>,
        config: AppConfig,
    ) -> sparkdns_lib::errors::AppResult<()> {
        sparkdns_lib::save_config(state, config)
    }

    #[tauri::command]
    pub fn reset_config(
        state: tauri::State<'_, AppState>,
    ) -> sparkdns_lib::errors::AppResult<AppConfig> {
        sparkdns_lib::reset_config(state)
    }

    #[tauri::command]
    pub fn export_config(state: tauri::State<'_, AppState>) -> sparkdns_lib::errors::AppResult<String> {
        sparkdns_lib::export_config(state)
    }

    #[tauri::command]
    pub fn import_config(
        state: tauri::State<'_, AppState>,
        json: String,
    ) -> sparkdns_lib::errors::AppResult<AppConfig> {
        sparkdns_lib::import_config(state, json)
    }

    #[tauri::command]
    pub async fn list_adapters(
        state: tauri::State<'_, AppState>,
    ) -> sparkdns_lib::errors::AppResult<Vec<NetworkAdapter>> {
        sparkdns_lib::list_adapters(state).await
    }

    #[tauri::command]
    pub async fn apply_dns(
        state: tauri::State<'_, AppState>,
        req: DnsApplyRequest,
    ) -> sparkdns_lib::errors::AppResult<CommandResult> {
        sparkdns_lib::apply_dns(state, req).await
    }

    #[tauri::command]
    pub async fn clear_dns(
        state: tauri::State<'_, AppState>,
        adapter_id: Option<String>,
    ) -> sparkdns_lib::errors::AppResult<CommandResult> {
        sparkdns_lib::clear_dns(state, adapter_id).await
    }

    #[tauri::command]
    pub async fn flush_dns_cache(
        state: tauri::State<'_, AppState>,
    ) -> sparkdns_lib::errors::AppResult<CommandResult> {
        sparkdns_lib::flush_dns_cache(state).await
    }

    #[tauri::command]
    pub async fn reset_adapter(adapter_id: String) -> sparkdns_lib::errors::AppResult<CommandResult> {
        sparkdns_lib::reset_adapter(adapter_id).await
    }

    #[tauri::command]
    pub async fn renew_dhcp(
        state: tauri::State<'_, AppState>,
        adapter_id: Option<String>,
    ) -> sparkdns_lib::errors::AppResult<CommandResult> {
        sparkdns_lib::renew_dhcp(state, adapter_id).await
    }

    #[tauri::command]
    pub async fn run_traceroute(host: String) -> sparkdns_lib::errors::AppResult<TracerouteResult> {
        sparkdns_lib::run_traceroute(host).await
    }

    #[tauri::command]
    pub async fn ping_target(
        state: tauri::State<'_, AppState>,
        host: String,
        adapter_id: Option<String>,
    ) -> sparkdns_lib::errors::AppResult<TargetProbeResult> {
        sparkdns_lib::ping_target(state, host, adapter_id).await
    }

    #[tauri::command]
    pub async fn comprehensive_check(
        state: tauri::State<'_, AppState>,
    ) -> sparkdns_lib::errors::AppResult<DiagnosticResult> {
        sparkdns_lib::comprehensive_check(state).await
    }

    #[tauri::command]
    pub async fn probe_server(
        state: tauri::State<'_, AppState>,
        server: String,
        adapter_id: Option<String>,
    ) -> sparkdns_lib::errors::AppResult<DnsMetrics> {
        sparkdns_lib::probe_server(state, server, adapter_id).await
    }

    #[tauri::command]
    pub async fn resolve_hostname(
        state: tauri::State<'_, AppState>,
        hostname: String,
        dns_server: Option<String>,
    ) -> sparkdns_lib::errors::AppResult<Vec<String>> {
        sparkdns_lib::resolve_hostname(state, hostname, dns_server).await
    }

    #[tauri::command]
    pub fn save_profile(
        state: tauri::State<'_, AppState>,
        profile: DnsProfile,
    ) -> sparkdns_lib::errors::AppResult<()> {
        sparkdns_lib::save_profile(state, profile)
    }

    #[tauri::command]
    pub fn delete_profile(
        state: tauri::State<'_, AppState>,
        profile_id: String,
    ) -> sparkdns_lib::errors::AppResult<()> {
        sparkdns_lib::delete_profile(state, profile_id)
    }

    #[tauri::command]
    pub fn update_settings(
        state: tauri::State<'_, AppState>,
        settings: AppSettings,
    ) -> sparkdns_lib::errors::AppResult<()> {
        sparkdns_lib::update_settings(state, settings)
    }

    #[tauri::command]
    pub fn add_rule(
        state: tauri::State<'_, AppState>,
        rule: AppRule,
    ) -> sparkdns_lib::errors::AppResult<()> {
        sparkdns_lib::add_rule(state, rule)
    }

    #[tauri::command]
    pub fn remove_rule(
        state: tauri::State<'_, AppState>,
        rule_id: String,
    ) -> sparkdns_lib::errors::AppResult<()> {
        sparkdns_lib::remove_rule(state, rule_id)
    }

    #[tauri::command]
    pub fn toggle_rule(
        state: tauri::State<'_, AppState>,
        rule_id: String,
        enabled: bool,
    ) -> sparkdns_lib::errors::AppResult<()> {
        sparkdns_lib::toggle_rule(state, rule_id, enabled)
    }
    #[tauri::command]
    pub fn add_auto_switch_rule(
        state: tauri::State<'_, AppState>,
        rule: AutoSwitchRule,
    ) -> sparkdns_lib::errors::AppResult<()> {
        sparkdns_lib::add_auto_switch_rule(state, rule)
    }

    #[tauri::command]
    pub fn remove_auto_switch_rule(
        state: tauri::State<'_, AppState>,
        rule_id: String,
    ) -> sparkdns_lib::errors::AppResult<()> {
        sparkdns_lib::remove_auto_switch_rule(state, rule_id)
    }

    #[tauri::command]
    pub fn toggle_auto_switch_rule(
        state: tauri::State<'_, AppState>,
        rule_id: String,
        enabled: bool,
    ) -> sparkdns_lib::errors::AppResult<()> {
        sparkdns_lib::toggle_auto_switch_rule(state, rule_id, enabled)
    }

    #[tauri::command]
    pub fn set_auto_switch_enabled(
        state: tauri::State<'_, AppState>,
        enabled: bool,
    ) -> sparkdns_lib::errors::AppResult<()> {
        sparkdns_lib::set_auto_switch_enabled(state, enabled)
    }

    #[tauri::command]
    pub async fn list_running_processes() -> sparkdns_lib::errors::AppResult<Vec<RunningProcess>> {
        sparkdns_lib::list_running_processes().await
    }

    #[tauri::command]
    pub fn run_installer(path: String) -> sparkdns_lib::errors::AppResult<()> {
        sparkdns_lib::run_installer(path)
    }

    #[tauri::command]
    pub fn open_url(url: String) -> sparkdns_lib::errors::AppResult<()> {
        sparkdns_lib::open_url(url)
    }

    #[tauri::command]
    pub fn send_notification(app: tauri::AppHandle, title: String, body: String) {
        use tauri_plugin_notification::NotificationExt;
        let _ = app.notification()
            .builder()
            .title(title)
            .body(body)
            .show();
    }

    /// Enable or disable launching SparkDns on system login (cross-platform).
    #[tauri::command]
    pub fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
        use tauri_plugin_autostart::ManagerExt;
        let manager = app.autolaunch();
        let result = if enabled { manager.enable() } else { manager.disable() };
        result.map_err(|e| e.to_string())
    }

    /// Report whether autostart is currently enabled.
    #[tauri::command]
    pub fn is_autostart_enabled(app: tauri::AppHandle) -> Result<bool, String> {
        use tauri_plugin_autostart::ManagerExt;
        app.autolaunch().is_enabled().map_err(|e| e.to_string())
    }
}

fn main() {
    use tauri::Manager;
    let state = AppState::new().expect("Failed to initialize application state");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(state)
        .setup(|app| {
            #[cfg(target_os = "windows")]
            {
                register_aumid_for_notifications();
            }
            setup_tray(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Close-to-tray: when the user closes the window and the setting is
            // enabled, hide instead of exiting so the service keeps running.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let minimize_to_tray = window
                    .app_handle()
                    .try_state::<AppState>()
                    .map(|s| s.config.lock().unwrap().settings.minimize_to_tray)
                    .unwrap_or(false);
                if minimize_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_capabilities,
            commands::get_config,
            commands::save_config,
            commands::reset_config,
            commands::export_config,
            commands::import_config,
            commands::list_adapters,
            commands::apply_dns,
            commands::clear_dns,
            commands::flush_dns_cache,
            commands::reset_adapter,
            commands::renew_dhcp,
            commands::run_traceroute,
            commands::ping_target,
            commands::comprehensive_check,
            commands::probe_server,
            commands::resolve_hostname,
            commands::save_profile,
            commands::delete_profile,
            commands::update_settings,
            commands::add_rule,
            commands::remove_rule,
            commands::add_auto_switch_rule,
            commands::remove_auto_switch_rule,
            commands::toggle_auto_switch_rule,
            commands::set_auto_switch_enabled,
            commands::list_running_processes,
            commands::send_notification,
            commands::run_installer,
            commands::open_url,

            commands::toggle_rule,
            commands::set_autostart,
            commands::is_autostart_enabled,
        ])
        .run(tauri::generate_context!())
        .expect("error while running SparkDns application");
}

/// Build the system tray icon with a Show / Quit menu.
fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{TrayIconBuilder, TrayIconEvent};
    use tauri::Manager;

    let show = MenuItem::with_id(app, "show", "Show SparkDns", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("SparkDns")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Left-click the tray icon to restore the window.
            if let TrayIconEvent::Click { button, .. } = event {
                if button == tauri::tray::MouseButton::Left {
                    let app = tray.app_handle();
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn register_aumid_for_notifications() {
    use std::os::windows::ffi::OsStrExt;

    extern "system" {
        fn RegCreateKeyExW(
            hkey: *mut core::ffi::c_void,
            subkey: *const u16,
            reserved: u32,
            class: *const u16,
            options: u32,
            desired: u32,
            security: *const core::ffi::c_void,
            result: *mut *mut core::ffi::c_void,
            disposition: *mut u32,
        ) -> i32;
        fn RegSetValueExW(
            hkey: *mut core::ffi::c_void,
            valuename: *const u16,
            reserved: u32,
            dwtype: u32,
            data: *const u8,
            cbdata: u32,
        ) -> i32;
        fn RegCloseKey(hkey: *mut core::ffi::c_void) -> i32;
    }

    const HKEY_CURRENT_USER: isize = -2147483647;
    const REG_SZ: u32 = 1;
    const KEY_WRITE: u32 = 0x20006;
    const OPEN_ALWAYS: u32 = 4;

    let aumid = std::ffi::OsStr::new("Software\\Classes\\AppUserModelId\\com.sparkdns.app");
    let name = std::ffi::OsStr::new("DisplayName");
    let value = std::ffi::OsStr::new("SparkDns");

    unsafe {
        let mut hkey: *mut core::ffi::c_void = std::ptr::null_mut();
        let path_wide: Vec<u16> = aumid.encode_wide().chain(std::iter::once(0)).collect();
        let name_wide: Vec<u16> = name.encode_wide().chain(std::iter::once(0)).collect();
        let value_wide: Vec<u16> = value.encode_wide().chain(std::iter::once(0)).collect();

        let rc = RegCreateKeyExW(
            HKEY_CURRENT_USER as *mut core::ffi::c_void,
            path_wide.as_ptr(),
            0,
            std::ptr::null(),
            OPEN_ALWAYS,
            KEY_WRITE,
            std::ptr::null(),
            &mut hkey,
            std::ptr::null_mut(),
        );
        if rc == 0 && !hkey.is_null() {
            let _ = RegSetValueExW(
                hkey,
                name_wide.as_ptr(),
                0,
                REG_SZ,
                value_wide.as_ptr() as *const u8,
                (value_wide.len() * 2) as u32,
            );
            let _ = RegCloseKey(hkey);
        }
    }
}