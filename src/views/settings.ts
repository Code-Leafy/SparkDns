import type { AppConfig } from "../types.js";
import type { ViewContext } from "../main.js";
import { escapeHtml } from "../main.js";

function toggleRow(key: keyof AppConfig["settings"], label: string, description: string, checked: boolean): string {
  return `
    <div class="list-row" style="padding: 24px;">
      <div>
        <div style="font-weight:600; font-size:15px; margin-bottom:4px; color:var(--text-primary);">${escapeHtml(label)}</div>
        <div class="text-secondary" style="font-size:13px;">${escapeHtml(description)}</div>
      </div>
      <label class="toggle">
        <input type="checkbox" data-setting="${String(key)}" ${checked ? "checked" : ""} />
        <span class="toggle-slider"></span>
      </label>
    </div>
  `;
}

export function renderSettings(ctx: ViewContext): string {
  const { state, capabilities } = ctx;
  const s = state.config.settings;

  const adapterOptions = state.adapters.map((a) =>
    `<option value="${escapeHtml(a.id)}" ${s.active_adapter_id === a.id ? "selected" : ""}>${escapeHtml(a.name)}${a.is_primary ? " (Primary)" : ""}</option>`
  ).join("");

  const isWindows = capabilities.os === "windows";
  const showDoh = capabilities.supports_doh && isWindows;

  const dohRow = showDoh ? `
    <div class="list-row" style="padding: 24px;">
      <div>
        <div style="font-weight:600; font-size:15px; margin-bottom:4px; color:var(--text-primary);">DNS Encryption</div>
        <div class="text-secondary" style="font-size:13px;">Encrypt DNS queries to prevent snooping. Choose between DoH (HTTPS) or DoT (TLS).</div>
      </div>
      <div class="flex items-center gap-4">
        <div style="position:relative;">
          <select class="rule-select" data-action="select-dns-encryption" style="appearance:none; padding:10px 36px 10px 16px; border-radius:10px; background:var(--bg-surface-elevated); border:1px solid var(--border-highlight); color:var(--text-primary); font-weight:500; font-size:13px; cursor:pointer; outline:none; min-width:100px;">
            <option value="none" ${!s.doh_enabled ? "selected" : ""}>Off</option>
            <option value="doh" ${s.doh_enabled && s.dns_encryption_type === "doh" ? "selected" : ""}>DoH</option>
            <option value="dot" ${s.doh_enabled && s.dns_encryption_type === "dot" ? "selected" : ""}>DoT</option>
          </select>
          <svg viewBox="0 0 24 24" width="14" height="14" stroke="var(--text-muted)" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round" style="position:absolute; right:12px; top:50%; transform:translateY(-50%); pointer-events:none;"><polyline points="6 9 12 15 18 9"></polyline></svg>
        </div>
      </div>
    </div>
  ` : "";

  const rows = [
    dohRow,
    toggleRow("ipv6_enabled", "Force IPv6 Routing", "Prefer AAAA records when available.", s.ipv6_enabled),
    toggleRow("auto_ping_enabled", "Auto-Ping on Startup", "Automatically ping all profiles when the app opens to show latency.", s.auto_ping_enabled),
    toggleRow("minimize_to_tray", "Minimize to Tray on Close", "Keep service running silently in system tray instead of exiting.", s.minimize_to_tray),
    toggleRow("start_on_boot", "Start on Login", "Launch SparkDns automatically when you sign in.", s.start_on_boot),
    toggleRow("theme", "Light Appearance", "Toggle OS native light mode aesthetics.", s.theme === "light"),
    `
    <div class="list-row" style="padding: 24px;">
      <div>
        <div style="font-weight:600; font-size:15px; margin-bottom:4px; color:var(--text-primary);">Interface</div>
        <div class="text-secondary" style="font-size:13px;">Choose which network adapter DNS is applied to. "All" targets every active adapter.</div>
      </div>
      <div style="position:relative;">
        <select class="rule-select" data-action="select-settings-adapter" style="appearance:none; padding:10px 36px 10px 16px; border-radius:10px; background:var(--bg-surface-elevated); border:1px solid var(--border-highlight); color:var(--text-primary); font-weight:500; font-size:13px; cursor:pointer; outline:none; min-width:180px;">
          <option value="" ${!s.active_adapter_id ? "selected" : ""}>All Adapters</option>
          ${adapterOptions}
        </select>
        <svg viewBox="0 0 24 24" width="14" height="14" stroke="var(--text-muted)" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round" style="position:absolute; right:12px; top:50%; transform:translateY(-50%); pointer-events:none;"><polyline points="6 9 12 15 18 9"></polyline></svg>
      </div>
    </div>
    `,
  ].join("");

  return `
    <div class="page-header">
      <h2>Preferences</h2>
    </div>

    <div class="grid-container" style="grid-template-rows: 1fr;">
      <div class="card col-span-4 row-span-3" style="padding: 0;">
        <div class="card-content">${rows}</div>
      </div>
    </div>
  `;
}