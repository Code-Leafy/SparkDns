import type { AppRule, AppRuntimeState, AutoSwitchRule, DnsProfile, PlatformCapabilities, RunningProcess, TargetProbeResult, ViewKey } from "./types.js";
import { store } from "./state.js";
import {
  loadPlatformCapabilities,
  loadConfig,
  listAdapters,
  applyDns,
  clearDns,
  evaluateDns,
  comprehensiveCheck,
  runSystemTool,
  saveConfig,
  exportConfigJson,
  importConfigJson,
  pingTarget,
  setAutostart,
  isAutostartEnabled,
  listRunningProcesses,
  removeAutoSwitchRule,
  setAutoSwitchEnabled,
  toggleAutoSwitchRule,
  IS_TAURI,
} from "./api.js";
import { DEFAULT_CONFIG } from "./defaults.js";
import { showToast } from "./ui/toast.js";
import { sendNotification } from "./ui/notify.js";
import { confirmDialog, promptDialog, ensureDialogHost } from "./ui/dialog.js";
import { renderHome } from "./views/home.js";
import { renderProfiles } from "./views/profiles.js";
import { renderRules } from "./views/rules.js";
import { renderAutoSwitch } from "./views/autoswitch.js";
import { renderDiagnostics } from "./views/diagnostics.js";
import { renderTools } from "./views/tools.js";
import { renderSettings } from "./views/settings.js";
import { renderAbout } from "./views/about.js";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { checkForUpdate, downloadAndInstall } from "./update.js";

const NAV_ITEMS: { key: ViewKey; label: string; icon: string }[] = [
  { key: "home", label: "Overview", icon: icon("home") },
  { key: "profiles", label: "Network Library", icon: icon("library") },
  { key: "rules", label: "App Rules", icon: icon("rules") },
  { key: "autoswitch", label: "Auto-Switch", icon: icon("autoswitch") },
  { key: "diagnostics", label: "Diagnostics", icon: icon("pulse") },
  { key: "tools", label: "System Tools", icon: icon("tool") },
  { key: "settings", label: "Preferences", icon: icon("settings") },
  { key: "about", label: "About", icon: icon("about") },
];

let currentView: ViewKey = "home";

export interface ViewContext {
  state: AppRuntimeState;
  capabilities: PlatformCapabilities;
}

async function boot(): Promise<void> {
  const systemDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  const systemTheme: "dark" | "light" = systemDark ? "dark" : "light";

  try {
    const capabilities = await loadPlatformCapabilities();
    store.set({ capabilities });
    applyTheme(systemTheme);
  } catch (err) {
    showToast("Failed to detect platform capabilities.", "error");
    console.error(err);
  }

  try {
    const config = await loadConfig();
    store.setConfig(config);
    applyTheme(config.settings.theme);
  } catch (err) {
    showToast("Failed to load configuration.", "error");
    console.error(err);
  }

  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", (e) => {
    const newTheme: "dark" | "light" = e.matches ? "dark" : "light";
    const state = store.get();
    if (state.config.settings.theme === "dark" || state.config.settings.theme === "light") {
      applyTheme(newTheme);
    }
  });

  await refreshAdapters();
  await syncAutostartState();
  store.subscribe(render);
  render(store.get());

  if (store.get().config.settings.auto_ping_enabled) {
    autoPingAllProfiles();
  }
}

async function autoPingAllProfiles(): Promise<void> {
  const state = store.get();
  const profiles = state.config.profiles;
  const adapterId = state.config.settings.active_adapter_id;
  store.beginBatch();
  store.setProbing(profiles.map((p) => p.id));
  store.endBatch();
  store.beginBatch();
  try {
    for (const profile of profiles) {
      try {
        const metrics = await evaluateDns(profile, adapterId);
        store.setMetrics({ ...metrics, profile_id: profile.id });
      } catch {
        // Skip individual failures during auto-ping
      }
    }
  } finally {
    store.setProbing([]);
    store.endBatch();
  }
}

/** Reconcile the persisted start_on_boot flag with the actual OS login item. */
async function syncAutostartState(): Promise<void> {
  if (!IS_TAURI) return;
  try {
    const enabled = await isAutostartEnabled();
    const state = store.get();
    if (state.config.settings.start_on_boot !== enabled) {
      store.setConfig({
        ...state.config,
        settings: { ...state.config.settings, start_on_boot: enabled },
      });
    }
  } catch (err) {
    console.error(err);
  }
}

function applyTheme(theme: "dark" | "light"): void {
  document.documentElement.setAttribute("data-theme", theme);
}

async function refreshAdapters(): Promise<void> {
  try {
    const adapters = await listAdapters();
    const state = store.get();
    const selected = state.selectedAdapterId ?? adapters.find((a) => a.is_primary)?.id ?? adapters[0]?.id ?? null;
    store.set({ adapters, selectedAdapterId: selected });
  } catch (err) {
    showToast("Failed to list network adapters.", "error");
    console.error(err);
  }
}

function renderShell(): string {
  return `
    <div class="titlebar" data-tauri-drag-region>
      <div class="titlebar-text" data-tauri-drag-region><img class="titlebar-spark" src="icon.png" alt="SparkDns">SparkDns</div>
      <div class="titlebar-controls">
        <button class="titlebar-btn" id="titlebar-minimize" title="Minimize">
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5"><line x1="2" y1="6" x2="10" y2="6"/></svg>
        </button>
        <button class="titlebar-btn titlebar-close" id="titlebar-close" title="Close">
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5"><line x1="2" y1="2" x2="10" y2="10"/><line x1="10" y1="2" x2="2" y2="10"/></svg>
        </button>
      </div>
    </div>
    <div class="app-container">
      <div class="dock-container"><nav class="dock" id="dock"></nav></div>
      <main class="main-content" id="content-area"><div class="page-container page-enter" id="view-root"></div></main>
    </div>
    ${renderAddProfileModal()}
  `;
}

function renderDock(caps: PlatformCapabilities): string {
  const visibleItems = NAV_ITEMS.filter((item) => item.key !== "rules" || caps.supports_app_rules);
  const items = visibleItems
    .map((item) => {
      const separator = item.key === "settings" || item.key === "about" ? '<div class="dock-separator" aria-hidden="true"></div>' : "";
      return `${separator}<button class="dock-item ${currentView === item.key ? "active" : ""}" data-nav="${item.key}" data-tab="${prototypeTab(item.key)}" data-tooltip="${item.label}" aria-label="${item.label}">${item.icon}</button>`;
    })
    .join("");

  return `
    <div class="dock-brand" title="SparkDns"><img src="icon.png" alt="SparkDns"></div>
    ${items}
  `;
}

function renderAddProfileModal(): string {
  return `
    <div class="modal-overlay" id="addProfileModal">
      <div class="modal-content modal-themed">
        <h3 style="font-size:18px; font-weight:600; margin-bottom:20px; color:var(--text-primary);">New Custom Node</h3>
        <label style="font-size:11px; font-weight:700; color:var(--text-muted); text-transform:uppercase; letter-spacing:0.05em; margin-bottom:14px;">
          Name
          <input class="input" type="text" id="cp_name" placeholder="My Resolver" style="margin-top:6px;">
        </label>
        <label style="font-size:11px; font-weight:700; color:var(--text-muted); text-transform:uppercase; letter-spacing:0.05em; margin-bottom:14px;">
          Primary IPv4
          <input class="input" type="text" id="cp_primary" placeholder="1.1.1.1" style="margin-top:6px;">
        </label>
        <label style="font-size:11px; font-weight:700; color:var(--text-muted); text-transform:uppercase; letter-spacing:0.05em; margin-bottom:14px;">
          Secondary IPv4
          <input class="input" type="text" id="cp_secondary" placeholder="1.0.0.1" style="margin-top:6px;">
        </label>
        <label style="font-size:11px; font-weight:700; color:var(--text-muted); text-transform:uppercase; letter-spacing:0.05em; margin-bottom:14px;">
          Primary IPv6
          <input class="input" type="text" id="cp_ipv6_1" placeholder="Optional" style="margin-top:6px;">
        </label>
        <label style="font-size:11px; font-weight:700; color:var(--text-muted); text-transform:uppercase; letter-spacing:0.05em; margin-bottom:14px;">
          Secondary IPv6
          <input class="input" type="text" id="cp_ipv6_2" placeholder="Optional" style="margin-top:6px;">
        </label>
        <div class="modal-actions">
          <button class="btn" data-action="close-add-profile-modal">Cancel</button>
          <button class="btn btn-primary" data-action="save-custom-profile">Save Node</button>
        </div>
      </div>
    </div>
  `;
}

function renderView(ctx: ViewContext): string {
  switch (currentView) {
    case "home": return renderHome(ctx);
    case "profiles": return renderProfiles(ctx);
    case "rules": return renderRules(ctx);
    case "autoswitch": return renderAutoSwitch(ctx);
    case "diagnostics": return renderDiagnostics(ctx);
    case "tools": return renderTools(ctx);
    case "settings": return renderSettings(ctx);
    case "about": return renderAbout(ctx);
    default: return "";
  }
}

let lastRenderedView: ViewKey | null = null;

function render(state: AppRuntimeState): void {
  const app = document.getElementById("app");
  if (!app) return;
  const caps = state.capabilities;
  if (!caps) {
    app.innerHTML = `<div class="app-container"><main class="main-content"><div class="page-container"><div class="card">Loading SparkDns...</div></div></main></div>`;
    return;
  }
  if (!app.querySelector(".titlebar")) {
    app.innerHTML = renderShell();
    bindTitlebarControls();
    lastRenderedView = null;
  }
  const dock = document.getElementById("dock");
  const viewRoot = document.getElementById("view-root");
  if (dock) {
    // Update only the active state on the dock instead of rebuilding it, so
    // navigating never causes a flash or rebinds listeners repeatedly.
    if (dock.dataset.bound !== "true") {
      dock.innerHTML = renderDock(caps);
      bindNavigation(dock);
      dock.dataset.bound = "true";
    }
    dock.querySelectorAll<HTMLElement>("[data-nav]").forEach((el) => {
      el.classList.toggle("active", el.getAttribute("data-nav") === currentView);
    });
  }
  if (viewRoot) {
    const viewChanged = lastRenderedView !== currentView;
    // Only replay the page-enter animation when the view actually changes.
    // Data-only updates (pings, toggles, connect state) re-render content
    // without the entrance animation, eliminating the constant flash.
    if (viewChanged) {
      viewRoot.classList.remove("page-enter");
      void viewRoot.offsetWidth;
      viewRoot.classList.add("page-enter");
    }
    viewRoot.innerHTML = renderView({ state, capabilities: caps });
    bindViewEvents(viewRoot, { state, capabilities: caps });
    lastRenderedView = currentView;
  }
  bindModalEvents({ state, capabilities: caps });

  const loadingOverlay = document.getElementById("loadingOverlay");
  if (loadingOverlay) loadingOverlay.classList.toggle("active", state.loading);
}

function bindNavigation(root: HTMLElement): void {
  root.querySelectorAll<HTMLElement>("[data-nav]").forEach((el) => {
    el.addEventListener("click", () => navigate(el.getAttribute("data-nav") as ViewKey));
  });
}

function bindTitlebarControls(): void {
  const minimizeBtn = document.getElementById("titlebar-minimize");
  const closeBtn = document.getElementById("titlebar-close");

  if (minimizeBtn) {
    minimizeBtn.addEventListener("click", () => {
      getCurrentWindow().minimize();
    });
  }
  if (closeBtn) {
    closeBtn.addEventListener("click", () => {
      getCurrentWindow().close();
    });
  }
}

function bindModalEvents(ctx: ViewContext): void {
  const modal = document.getElementById("addProfileModal");
  if (!modal || modal.dataset.bound === "true") return;
  modal.dataset.bound = "true";

  document.querySelectorAll<HTMLElement>("#addProfileModal [data-action]").forEach((el) => {
    el.addEventListener("click", () => handleAction(el, ctx));
  });
  modal.addEventListener("click", (event) => {
    if (event.target === event.currentTarget) toggleAddProfileModal(false);
  });
}

function bindViewEvents(root: HTMLElement, ctx: ViewContext): void {
  root.querySelectorAll<HTMLElement>("[data-action]:not(select)").forEach((el) => {
    el.addEventListener("click", () => handleAction(el, ctx));
  });
  root.querySelectorAll<HTMLSelectElement>("select[data-action]").forEach((select) => {
    select.addEventListener("change", () => handleAction(select, ctx));
  });
  root.querySelectorAll<HTMLInputElement>("[data-setting]").forEach((input) => {
    input.addEventListener("change", () => handleSettingInput(input));
  });
  root.querySelectorAll<HTMLSelectElement>("[data-select-adapter]").forEach((select) => {
    select.addEventListener("change", () => selectAdapter(select.value));
  });
  root.querySelector<HTMLInputElement>("#profile-search")?.addEventListener("input", (event) => {
    const query = (event.currentTarget as HTMLInputElement).value.toLowerCase();
    root.querySelectorAll<HTMLElement>(".profile-row").forEach((row) => {
      row.style.display = row.dataset.search?.includes(query) ? "" : "none";
    });
  });
}

async function handleAction(el: HTMLElement, ctx: ViewContext): Promise<void> {
  const action = el.getAttribute("data-action");
  if (!action) return;
  const state = ctx.state;
  const capabilities = ctx.capabilities;

  switch (action) {
    case "connect": {
      const profileId = el.getAttribute("data-profile-id");
      if (profileId) await connectProfile(profileId);
      break;
    }
    case "disconnect": await disconnect(); break;
    case "evaluate": {
      const profileId = el.getAttribute("data-profile-id");
      if (profileId) await evaluateProfile(profileId);
      break;
    }
    case "refresh-pings": await evaluateAllProfiles(); break;
    case "run-diagnostics": await runDiagnostics(); break;
    case "flush-dns": await execSystemTool("flush_cache", capabilities.supports_flush_dns); break;
    case "renew-dhcp": await execSystemTool("reset_dhcp", capabilities.supports_dhcp_renew, state.selectedAdapterId); break;
    case "reset-adapter": {
      const select = document.getElementById("reset-adapter-select") as HTMLSelectElement | null;
      await execSystemTool("reset_adapter", capabilities.supports_adapter_reset, select?.value ?? state.selectedAdapterId);
      break;
    }
    case "traceroute": {
      const hostInput = document.getElementById("traceroute-host") as HTMLInputElement | null;
      await execTraceroute(capabilities.supports_traceroute, hostInput?.value.trim() || "1.1.1.1");
      break;
    }
    case "toggle-add-profile-modal": toggleAddProfileModal(true); break;
    case "close-add-profile-modal": toggleAddProfileModal(false); break;
    case "save-custom-profile": await saveCustomProfile(); break;
    case "toggle-favorite": {
      const profileId = el.getAttribute("data-profile-id");
      if (profileId) await toggleFavorite(profileId);
      break;
    }
    case "delete-profile": {
      const profileId = el.getAttribute("data-profile-id");
      if (profileId) await deleteCustomProfile(profileId);
      break;
    }
    case "add-rule": await addRule(); break;
    case "update-rule-profile": {
      const ruleId = el.getAttribute("data-rule-id");
      const profileId = el instanceof HTMLSelectElement ? el.value : null;
      if (ruleId && profileId) await updateRuleProfile(ruleId, profileId);
      break;
    }
    case "simulate-rule": {
      const ruleId = el.getAttribute("data-rule-id");
      if (ruleId) await simulateRule(ruleId);
      break;
    }
    case "delete-rule": {
      const ruleId = el.getAttribute("data-rule-id");
      if (ruleId) await deleteRule(ruleId);
      break;
    }
    case "toggle-auto-rule": {
      const autoRuleId = el.getAttribute("data-rule-id");
      const autoRule = state.config.auto_switch_rules.find((r) => r.id === autoRuleId);
      if (autoRuleId && autoRule) {
        await toggleAutoSwitchRule(autoRuleId, !autoRule.enabled);
        store.setConfig({
          ...state.config,
          auto_switch_rules: state.config.auto_switch_rules.map((r) =>
            r.id === autoRuleId ? { ...r, enabled: !r.enabled } : r
          ),
        });
        await persist();
      }
      break;
    }
    case "update-auto-rule-profile": {
      const autoRuleId = el.getAttribute("data-rule-id");
      const profileId = el instanceof HTMLSelectElement ? el.value : null;
      if (autoRuleId && profileId) {
        store.setConfig({
          ...state.config,
          auto_switch_rules: state.config.auto_switch_rules.map((r) =>
            r.id === autoRuleId ? { ...r, profile_id: profileId } : r
          ),
        });
        await persist();
        showToast("Rule target updated.", "success");
      }
      break;
    }
    case "delete-auto-rule": {
      const autoRuleId = el.getAttribute("data-rule-id");
      if (autoRuleId) {
        const rule = state.config.auto_switch_rules.find((r) => r.id === autoRuleId);
        const ok = await confirmDialog({
          title: "Delete auto-switch rule",
          message: `Delete the rule for "${rule?.app_label ?? "this app"}"?`,
          confirmLabel: "Delete",
          danger: true,
        });
        if (!ok) return;
        await removeAutoSwitchRule(autoRuleId);
        store.setConfig({
          ...state.config,
          auto_switch_rules: state.config.auto_switch_rules.filter((r) => r.id !== autoRuleId),
        });
        await persist();
        showToast("Auto-switch rule deleted.", "success");
      }
      break;
    }
    case "toggle-auto-switch-master": {
      const newEnabled = !state.config.auto_switch_enabled;
      await setAutoSwitchEnabled(newEnabled);
      store.setConfig({ ...state.config, auto_switch_enabled: newEnabled });
      await persist();
      showToast(newEnabled ? "Auto-switch watcher enabled." : "Auto-switch watcher disabled.", "success");
      break;
    }
    case "add-auto-rule-running": {
      await addAutoRuleFromRunning();
      break;
    }
    case "add-auto-rule-browse": {
      await addAutoRuleFromBrowse();
      break;
    }
    case "select-settings-adapter": {
      const val = el instanceof HTMLSelectElement ? el.value : null;
      const adapterId = val || null;
      await updateSetting("active_adapter_id", adapterId);
      store.set({ selectedAdapterId: adapterId });
      await refreshAdapters();
      showToast(adapterId ? "DNS will apply to this adapter." : "DNS will apply to all active adapters.", "success");
      break;
    }
    case "select-dns-encryption": {
      const val = el instanceof HTMLSelectElement ? el.value : "none";
      const dohEnabled = val !== "none";
      const encType: "doh" | "dot" = dohEnabled ? (val as "doh" | "dot") : state.config.settings.dns_encryption_type;
      store.setConfig({
        ...state.config,
        settings: { ...state.config.settings, doh_enabled: dohEnabled, dns_encryption_type: encType },
      });
      await persist();
      const label = val === "none" ? "DNS encryption disabled" : val === "doh" ? "DNS over HTTPS (DoH) enabled" : "DNS over TLS (DoT) enabled";
      showToast(label, "success");
      break;
    }
    case "export-data": await exportData(); break;
    case "import-data": await importData(); break;
    case "clear-logs": {
      store.setConfig({ ...state.config, history: [] });
      await persist();
      showToast("Connection history cleared.", "success");
      break;
    }
    case "factory-reset": {
      const ok = await confirmDialog({
        title: "Factory reset",
        message: "This resets all settings and removes custom profiles. Presets and history will be cleared. This cannot be undone.",
        confirmLabel: "Reset everything",
        danger: true,
      });
      if (!ok) return;
      store.setConfig(structuredClone(DEFAULT_CONFIG));
      await persist();
      showToast("Factory reset complete.", "success");
      break;
    }
    case "check-update": {
      await checkForUpdateAction();
      break;
    }
    case "open-github": {
      if (IS_TAURI) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          await invoke("open_url", { url: "https://github.com/Code-Leafy/SparkDns" });
        } catch {
          window.open("https://github.com/Code-Leafy/SparkDns", "_blank");
        }
      } else {
        window.open("https://github.com/Code-Leafy/SparkDns", "_blank");
      }
      break;
    }
    case "open-license": {
      const licenseText = [
        "MIT License",
        "",
        "Copyright (c) 2026 SparkDns",
        "",
        "Permission is hereby granted, free of charge, to any person obtaining a copy",
        "of this software and associated documentation files (the \"Software\"), to deal",
        "in the Software without restriction, including without limitation the rights",
        "to use, copy, modify, merge, publish, distribute, sublicense, and/or sell",
        "copies of the Software, and to permit persons to whom the Software is",
        "furnished to do so, subject to the following conditions:",
        "",
        "The above copyright notice and this permission notice shall be included in all",
        "copies or substantial portions of the Software.",
        "",
        "THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR",
        "IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,",
        "FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE",
        "AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER",
        "LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,",
        "OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE",
        "SOFTWARE.",
      ].join("\n");
      await confirmDialog({
        title: "MIT License",
        message: licenseText,
        confirmLabel: "Close",
      });
      break;
    }
  }
}

export async function connectProfile(profileId: string): Promise<void> {
  const state = store.get();
  const profile = state.config.profiles.find((p) => p.id === profileId);
  if (!profile) return;
  document.body.classList.add("dns-applying");
  store.setLoading(true);
  try {
    const adapterId = state.config.settings.active_adapter_id;
    const result = await applyDns(profile, adapterId, state.config.settings.doh_enabled, state.config.settings.ipv6_enabled);
    if (result.ok) {
      const next = {
        ...state.config,
        active_profile_id: profile.id,
        is_connected: true,
        history: [{ name: profile.name, ip: profile.primary_ipv4, time: new Date().toISOString() }, ...state.config.history].slice(0, 50),
      };
      store.setConfig(next);
      await persist();
      showToast(`Connected to ${profile.name}.`, "success");
      sendNotification("SparkDns", `DNS changed to ${profile.name}`);
      if (result.requires_elevation) showToast("Elevation was required to apply DNS.", "info");
    } else {
      showToast(result.message || "Failed to apply DNS.", "error");
    }
  } catch (err) {
    showToast("DNS apply failed.", "error");
    console.error(err);
  } finally {
    store.setLoading(false);
    document.body.classList.remove("dns-applying");
  }
}

export async function disconnect(): Promise<void> {
  const state = store.get();
  document.body.classList.add("dns-applying");
  store.setLoading(true);
  try {
    const adapterId = state.config.settings.active_adapter_id;
    const result = await clearDns(adapterId);
    if (result.ok) {
      store.setConfig({ ...state.config, active_profile_id: null, is_connected: false });
      await persist();
      showToast("DNS cleared.", "success");
      sendNotification("SparkDns", "DNS cleared — back to system defaults");
    } else {
      showToast(result.message || "Failed to clear DNS.", "error");
    }
  } catch (err) {
    showToast("DNS clear failed.", "error");
    console.error(err);
  } finally {
    store.setLoading(false);
    document.body.classList.remove("dns-applying");
  }
}

export async function evaluateProfile(profileId: string): Promise<void> {
  const state = store.get();
  const profile = state.config.profiles.find((p) => p.id === profileId);
  if (!profile) return;
  const adapterId = state.config.settings.active_adapter_id;
  const current = store.get().probing;
  if (!current.includes(profileId)) store.setProbing([...current, profileId]);
  try {
    const metrics = await evaluateDns(profile, adapterId);
    store.setMetrics({ ...metrics, profile_id: profile.id });
  } catch (err) {
    showToast("Evaluation failed.", "error");
    console.error(err);
  } finally {
    store.clearProbing(profileId);
  }
}

async function evaluateAllProfiles(): Promise<void> {
  const profiles = store.get().config.profiles;
  store.beginBatch();
  store.setProbing(profiles.map((p) => p.id));
  store.endBatch();
  store.beginBatch();
  try {
    for (const profile of profiles) await evaluateProfile(profile.id);
    showToast("All nodes evaluated.", "success");
  } finally {
    store.setProbing([]);
    store.endBatch();
  }
}

export async function runDiagnostics(): Promise<void> {
  const state = store.get();
  const targets = state.config.settings.diagnostic_targets;

  store.setDiagnostics({ running: true, targetResults: {} });

  const targetResults: Record<string, { latency_ms: number | null; reachable: boolean }> = {};

  for (const t of targets) {
    let probe: TargetProbeResult;
    try {
      probe = await pingTarget(t.host, state.selectedAdapterId);
    } catch {
      probe = { id: t.id, name: t.name, host: t.host, latency_ms: null, reachable: false, error: "probe failed" };
    }
    targetResults[t.id] = { latency_ms: probe.latency_ms, reachable: probe.reachable };
    store.setDiagnostics({ targetResults: { ...targetResults } });
  }

  try {
    const result = await comprehensiveCheck(targets);
    store.setDiagnostics({ running: false, result, targetResults });

    store.setMetrics({
      profile_id: "__diag",
      server: "multi",
      latency_ms: result.latency_ms,
      reachability_percent: result.targets.filter((t) => t.reachable).length / Math.max(1, result.targets.length) * 100,
      packet_loss_percent: result.packet_loss_percent,
      tested_at: new Date().toISOString(),
      error: null,
    });
    showToast("Diagnostics complete.", "success");
  } catch (err) {
    store.setDiagnostics({ running: false });
    showToast("Diagnostics failed.", "error");
    console.error(err);
  }
}

async function execSystemTool(tool: "flush_cache" | "reset_dhcp" | "reset_adapter", supported: boolean, adapterId?: string | null): Promise<void> {
  if (!supported) {
    showToast("This tool is not supported on your platform.", "error");
    return;
  }
  store.setLoading(true);
  try {
    const result = await runSystemTool(tool, { adapterId: adapterId ?? null }) as { ok: boolean; message: string };
    showToast(result.ok ? result.message || "Done." : result.message || "Failed.", result.ok ? "success" : "error");
  } catch (err) {
    showToast("Tool execution failed.", "error");
    console.error(err);
  } finally {
    store.setLoading(false);
  }
}

async function execTraceroute(supported: boolean, host: string): Promise<void> {
  if (!supported) {
    showToast("Traceroute is not supported on your platform.", "error");
    return;
  }
  store.setLoading(true);
  try {
    const result = await runSystemTool("run_traceroute", { host }) as { ok: boolean; hops: unknown[]; error: string | null };
    showToast(result.ok ? `Traceroute: ${result.hops.length} hops` : result.error || "Failed.", result.ok ? "success" : "error");
  } catch (err) {
    showToast("Traceroute failed.", "error");
    console.error(err);
  } finally {
    store.setLoading(false);
  }
}

async function handleSettingInput(input: HTMLInputElement): Promise<void> {
  const key = input.dataset.setting as keyof AppRuntimeState["config"]["settings"];
  const value = key === "theme" ? (input.checked ? "light" : "dark") : input.checked;
  await updateSetting(key, value as never);

  // Wire the autostart toggle to the OS-level login item.
  if (key === "start_on_boot") {
    try {
      await setAutostart(input.checked);
      showToast(input.checked ? "SparkDns will start on login." : "Startup on login disabled.", "info");
    } catch (err) {
      showToast("Could not update startup setting.", "error");
      console.error(err);
    }
  }
}

async function selectAdapter(adapterId: string): Promise<void> {
  store.set({ selectedAdapterId: adapterId });
  await updateSetting("active_adapter_id", adapterId);
}

function toggleAddProfileModal(open: boolean): void {
  document.getElementById("addProfileModal")?.classList.toggle("active", open);
}

async function saveCustomProfile(): Promise<void> {
  const name = getInputValue("cp_name");
  const primary = getInputValue("cp_primary");
  if (!name || !primary) {
    showToast("Name and Primary IPv4 are required.", "error");
    return;
  }
  const profile: DnsProfile = {
    id: `custom_${Date.now()}`,
    name,
    primary_ipv4: primary,
    secondary_ipv4: getInputValue("cp_secondary") || null,
    primary_ipv6: getInputValue("cp_ipv6_1") || null,
    secondary_ipv6: getInputValue("cp_ipv6_2") || null,
    doh_url: null,
    dot_host: null,
    favorite: true,
    preset: false,
  };
  store.upsertProfile(profile);
  await persist();
  toggleAddProfileModal(false);
  ["cp_name", "cp_primary", "cp_secondary", "cp_ipv6_1", "cp_ipv6_2"].forEach((id) => {
    const input = document.getElementById(id) as HTMLInputElement | null;
    if (input) input.value = "";
  });
  showToast("Custom node added.", "success");
}

async function toggleFavorite(profileId: string): Promise<void> {
  const state = store.get();
  const profile = state.config.profiles.find((p) => p.id === profileId);
  if (!profile) return;
  store.upsertProfile({ ...profile, favorite: !profile.favorite });
  await persist();
}

async function deleteCustomProfile(profileId: string): Promise<void> {
  const profile = store.get().config.profiles.find((p) => p.id === profileId);
  if (!profile || profile.preset) return;
  const ok = await confirmDialog({
    title: "Delete node",
    message: `Delete the custom node "${profile.name}"? This cannot be undone.`,
    confirmLabel: "Delete",
    danger: true,
  });
  if (!ok) return;
  store.removeProfile(profileId);
  await persist();
  showToast("Custom node deleted.", "success");
}

async function addRule(): Promise<void> {
  const state = store.get();
  const defaultProfile = state.config.active_profile_id ?? state.config.profiles[0]?.id;
  if (!defaultProfile) {
    showToast("Create a DNS profile before adding app rules.", "error");
    return;
  }

  const values = await promptDialog({
    title: "Add app rule",
    fields: [
      { id: "app_name", label: "Application name", placeholder: "Game.exe" },
      { id: "app_path", label: "Application path (optional)", placeholder: "C:\\\\path\\\\to\\\\app.exe" },
    ],
    confirmLabel: "Add rule",
  });
  if (!values) return;
  const appName = values.app_name?.trim();
  if (!appName) {
    showToast("Application name is required.", "error");
    return;
  }
  const rule: AppRule = {
    id: `rule_${Date.now()}`,
    app_name: appName,
    app_path: values.app_path?.trim() || null,
    profile_id: defaultProfile,
    enabled: true,
  };

  store.setConfig({ ...state.config, rules: [rule, ...state.config.rules] });
  await persist();
  showToast("App rule added.", "success");
}

async function updateRuleProfile(ruleId: string, profileId: string): Promise<void> {
  const state = store.get();
  const rules = state.config.rules.map((rule) => rule.id === ruleId ? { ...rule, profile_id: profileId } : rule);
  store.setConfig({ ...state.config, rules });
  await persist();
  showToast("Rule target updated.", "success");
}

async function simulateRule(ruleId: string): Promise<void> {
  const rule = store.get().config.rules.find((r) => r.id === ruleId);
  if (!rule) return;
  showToast(`Applying rule profile for ${rule.app_name}.`, "info");
  await connectProfile(rule.profile_id);
}

async function addAutoRuleFromRunning(): Promise<void> {
  const state = store.get();
  const defaultProfile = state.config.active_profile_id ?? state.config.profiles[0]?.id;
  if (!defaultProfile) {
    showToast("Create a DNS profile before adding auto-switch rules.", "error");
    return;
  }

  let processes: RunningProcess[];
  try {
    processes = await listRunningProcesses();
  } catch (err) {
    showToast("Failed to list running processes.", "error");
    console.error(err);
    return;
  }

  if (processes.length === 0) {
    showToast("No running processes detected.", "info");
    return;
  }

  const sorted = [...processes].sort((a, b) => a.name.localeCompare(b.name));
  const host = ensureDialogHost();
  const listHtml = sorted
    .map((p, i) => {
      const displayPath = p.path ? escapeHtml(p.path) : "N/A";
      return `<div class="list-row process-pick-row" data-idx="${i}" style="cursor:pointer; padding: 10px 16px;">
        <div class="flex-col" style="min-width:0;">
          <span style="font-weight:600; font-size:13px; color:var(--text-primary);">${escapeHtml(p.name)}</span>
          <span class="text-muted font-mono" style="font-size:11px; white-space:nowrap; overflow:hidden; text-overflow:ellipsis;">${displayPath}</span>
        </div>
      </div>`;
    })
    .join("");
  host.innerHTML = `
    <div class="modal-overlay active" id="__proc_overlay">
      <div class="modal-content" role="dialog" aria-modal="true" style="max-height:70vh; display:flex; flex-direction:column;">
        <h3>Pick a Running App</h3>
        <div class="search-wrapper" style="margin-bottom:12px;">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
          <input type="text" id="__proc_search" placeholder="Search processes..." style="width:100%;">
        </div>
        <div class="card-content" style="flex:1; overflow-y:auto; max-height:360px; border:1px solid var(--border-light); border-radius:var(--radius-md);" id="__proc_list">${listHtml}</div>
        <div class="modal-actions">
          <button class="btn" id="__proc_cancel">Cancel</button>
        </div>
      </div>
    </div>`;
  const overlay = host.querySelector<HTMLElement>("#__proc_overlay");
  const cleanup = () => { host.innerHTML = ""; };
  host.querySelector("#__proc_cancel")?.addEventListener("click", cleanup);
  overlay?.addEventListener("click", (e) => { if (e.target === overlay) cleanup(); });

  host.querySelector<HTMLInputElement>("#__proc_search")?.addEventListener("input", (e) => {
    const q = (e.currentTarget as HTMLInputElement).value.toLowerCase();
    host.querySelectorAll<HTMLElement>(".process-pick-row").forEach((row) => {
      row.style.display = row.textContent?.toLowerCase().includes(q) ? "" : "none";
    });
  });

  host.querySelectorAll<HTMLElement>(".process-pick-row").forEach((row) => {
    row.addEventListener("click", () => {
      const idx = parseInt(row.getAttribute("data-idx") ?? "-1", 10);
      if (idx < 0 || idx >= sorted.length) return;
      const proc = sorted[idx];
      cleanup();
      const name = proc.name.replace(/\.(exe|app|bin|sh)$/i, "");
      const rule: AutoSwitchRule = {
        id: `auto_${Date.now()}`,
        app_label: name,
        match_name: proc.name,
        app_path: proc.path,
        profile_id: defaultProfile,
        enabled: true,
      };
      store.setConfig({
        ...state.config,
        auto_switch_rules: [rule, ...state.config.auto_switch_rules],
      });
      persist();
      showToast(`Auto-switch rule added for ${name}.`, "success");
    });
  });
}

async function addAutoRuleFromBrowse(): Promise<void> {
  const state = store.get();
  const defaultProfile = state.config.active_profile_id ?? state.config.profiles[0]?.id;
  if (!defaultProfile) {
    showToast("Create a DNS profile before adding auto-switch rules.", "error");
    return;
  }

  if (!IS_TAURI) {
    showToast("File browsing requires the desktop app.", "error");
    return;
  }

  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      multiple: false,
      filters: [{ name: "Executables", extensions: ["exe", "app", "bin"] }],
    });
    const path = Array.isArray(selected) ? selected[0] : selected;
    if (!path) return;

    const parts = path.replace(/\\/g, "/").split("/");
    const filename = parts[parts.length - 1] ?? path;
    const rule: AutoSwitchRule = {
      id: `auto_${Date.now()}`,
      app_label: filename.replace(/\.(exe|app|bin)$/i, ""),
      match_name: filename,
      app_path: path,
      profile_id: defaultProfile,
      enabled: true,
    };
    store.setConfig({
      ...state.config,
      auto_switch_rules: [rule, ...state.config.auto_switch_rules],
    });
    await persist();
    showToast(`Auto-switch rule added for ${filename}.`, "success");
  } catch (err) {
    showToast("Failed to browse for executable.", "error");
    console.error(err);
  }
}

async function deleteRule(ruleId: string): Promise<void> {
  const state = store.get();
  const rule = state.config.rules.find((r) => r.id === ruleId);
  if (!rule) return;
  const ok = await confirmDialog({
    title: "Delete rule",
    message: `Delete the app rule for "${rule.app_name}"?`,
    confirmLabel: "Delete",
    danger: true,
  });
  if (!ok) return;
  store.setConfig({ ...state.config, rules: state.config.rules.filter((r) => r.id !== ruleId) });
  await persist();
  showToast("App rule deleted.", "success");
}

export async function updateSetting<K extends keyof AppRuntimeState["config"]["settings"]>(key: K, value: AppRuntimeState["config"]["settings"][K]): Promise<void> {
  const state = store.get();
  const settings = { ...state.config.settings, [key]: value };
  store.setConfig({ ...state.config, settings });
  if (key === "theme") applyTheme(value as "dark" | "light");
  await persist();
}

export async function persist(): Promise<void> {
  try {
    await saveConfig(store.get().config);
  } catch (err) {
    showToast("Failed to save configuration.", "error");
    console.error(err);
  }
}

async function checkForUpdateAction(): Promise<void> {
  store.setLoading(true);
  try {
    const currentVersion = "0.1.0";
    const info = await checkForUpdate(currentVersion);
    if (info.hasUpdate) {
      const ok = await confirmDialog({
        title: "Update Available",
        message: `A new version (v${info.version}) is available. Would you like to download and install it?`,
        confirmLabel: "Download & Install",
      });
      if (ok) {
        store.setLoading(true);
        try {
          await downloadAndInstall(info.downloadUrl);
          showToast("Update downloaded. Please install and restart.", "success");
        } catch (dlErr) {
          showToast("Download failed. Opening browser instead.", "info");
          window.open(info.downloadUrl, "_blank");
          console.error(dlErr);
        } finally {
          store.setLoading(false);
        }
      }
    } else {
      showToast("You are running the latest version.", "info");
    }
  } catch (err) {
    showToast("Failed to check for updates.", "error");
    console.error(err);
  } finally {
    store.setLoading(false);
  }
}

async function exportData(): Promise<void> {
  try {
    const json = await exportConfigJson();
    if (!IS_TAURI) {
      // Browser dev fallback: trigger a download.
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "sparkdns-backup.json";
      a.click();
      URL.revokeObjectURL(url);
      showToast("Export complete.", "success");
      return;
    }
    const { save } = await import("@tauri-apps/plugin-dialog");
    const { writeTextFile } = await import("@tauri-apps/plugin-fs");
    const path = await save({
      defaultPath: "sparkdns-backup.json",
      filters: [{ name: "SparkDns Backup", extensions: ["json"] }],
    });
    if (!path) return;
    await writeTextFile(path, json);
    showToast("Configuration exported.", "success");
  } catch (err) {
    showToast("Export failed.", "error");
    console.error(err);
  }
}

async function importData(): Promise<void> {
  try {
    if (!IS_TAURI) {
      showToast("Import requires the desktop app.", "error");
      return;
    }
    const { open } = await import("@tauri-apps/plugin-dialog");
    const { readTextFile } = await import("@tauri-apps/plugin-fs");
    const selected = await open({
      multiple: false,
      filters: [{ name: "SparkDns Backup", extensions: ["json"] }],
    });
    const path = Array.isArray(selected) ? selected[0] : selected;
    if (!path) return;
    const json = await readTextFile(path);
    const config = await importConfigJson(json);
    store.setConfig(config);
    applyTheme(config.settings.theme);
    await persist();
    showToast("Configuration imported.", "success");
  } catch (err) {
    showToast("Import failed. The file may be invalid.", "error");
    console.error(err);
  }
}

function getInputValue(id: string): string {
  return (document.getElementById(id) as HTMLInputElement | null)?.value.trim() ?? "";
}

export function escapeHtml(value: string): string {
  return value.replace(/[&<>"']/g, (ch) => `&#${ch.charCodeAt(0)};`);
}

export function navigate(view: ViewKey): void {
  currentView = view;
  render(store.get());
}

function icon(name: "home" | "library" | "rules" | "autoswitch" | "pulse" | "tool" | "settings" | "about"): string {
  const icons = {
    home: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor"><rect x="3" y="3" width="7" height="7" rx="1.5"></rect><rect x="14" y="3" width="7" height="7" rx="1.5"></rect><rect x="14" y="14" width="7" height="7" rx="1.5"></rect><rect x="3" y="14" width="7" height="7" rx="1.5"></rect></svg>',
    library: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"></path><polyline points="3.27 6.96 12 12.01 20.73 6.96"></polyline><line x1="12" y1="22.08" x2="12" y2="12"></line></svg>',
    rules: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><line x1="4" y1="21" x2="4" y2="14"/><line x1="4" y1="10" x2="4" y2="3"/><line x1="12" y1="21" x2="12" y2="12"/><line x1="12" y1="8" x2="12" y2="3"/><line x1="20" y1="21" x2="20" y2="16"/><line x1="20" y1="12" x2="20" y2="3"/><line x1="1" y1="14" x2="7" y2="14"/><line x1="9" y1="8" x2="15" y2="8"/><line x1="17" y1="16" x2="23" y2="16"/></svg>',
    autoswitch: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/></svg>',
    pulse: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg>',
    tool: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect><line x1="3" y1="9" x2="21" y2="9"></line><line x1="9" y1="21" x2="9" y2="9"></line></svg>',
    settings: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>',
    about: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="11"/><circle cx="12" cy="8" r="1.2" fill="currentColor" stroke="none"/></svg>',
  };
  return icons[name];
}

function prototypeTab(view: ViewKey): string {
  if (view === "diagnostics") return "check";
  if (view === "settings") return "settings";
  return view;
}

document.addEventListener("DOMContentLoaded", () => {
  boot();
});