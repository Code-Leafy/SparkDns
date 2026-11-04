import type { ViewContext } from "../main.js";
import { escapeHtml } from "../main.js";
import { formatDate } from "./prototype.js";

function toolCard(action: string, title: string, description: string, icon: string, color = "var(--text-primary)", bg = "var(--bg-surface-hover)", border = "1px solid var(--border-light)", disabled = false): string {
  return `
    <div class="card col-span-1 row-span-1 interactive flex-col justify-center ${disabled ? "disabled" : ""}" style="padding: 18px;" ${disabled ? "" : `data-action="${action}"`}>
      <div class="flex items-center" style="gap: 16px; margin-bottom: 12px;">
        <div style="color: ${color}; background: ${bg}; border: ${border}; padding: 7px; border-radius: 8px; display: flex;">
          ${icon}
        </div>
        <span style="font-weight: 600; font-size: 14px; color: var(--text-primary);">${title}</span>
      </div>
      <div style="font-size: 12px; color: var(--text-secondary); line-height: 1.4;">${description}</div>
    </div>`;
}

export function renderTools(ctx: ViewContext): string {
  const { state, capabilities } = ctx;
  const histRows = state.config.history.slice(0, 20).map((h) => `
    <div class="list-row" style="padding: 16px 20px; border-bottom: 1px solid var(--border-light); display: flex; align-items: center; justify-content: space-between;">
      <div style="display: flex; flex-direction: column; position: relative; padding-left: 20px;">
        <div style="position: absolute; left: 0; top: 6px; width: 8px; height: 8px; border-radius: 50%; background: var(--success); box-shadow: 0 0 0 4px rgba(16, 185, 129, 0.15);"></div>
        <div style="font-weight: 600; font-size: 14px; color: var(--text-primary); line-height: 1;">${escapeHtml(h.name)}</div>
        <div class="font-mono text-secondary" style="font-size: 12px; margin-top: 4px;">${escapeHtml(h.ip)}</div>
      </div>
      <div class="text-muted" style="font-size: 12px; font-weight: 500;">${escapeHtml(formatDate(h.time))}</div>
    </div>`).join("");

  const icons = {
    flush: '<svg width="20" height="20" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24"><path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/><path d="M16 21v-5h5"/></svg>',
    dhcp: '<svg width="20" height="20" stroke="currentColor" stroke-width="2.5" fill="none" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24"><path d="M22 12h-4l-3 9L9 3l-3 9H2"/></svg>',
    power: '<svg width="20" height="20" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24"><path d="M18.36 6.64a9 9 0 1 1-12.73 0"/><line x1="12" y1="2" x2="12" y2="12"/></svg>',
    trace: '<svg width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24"><circle cx="18" cy="5" r="3"/><circle cx="6" cy="12" r="3"/><circle cx="18" cy="19" r="3"/><line x1="8.59" y1="13.51" x2="15.42" y2="17.49"/><line x1="15.41" y1="6.51" x2="8.59" y2="10.49"/></svg>',
    export: '<svg width="20" height="20" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>',
    import: '<svg width="20" height="20" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg>',
    trash: '<svg width="20" height="20" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/><line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/></svg>',
    reset: '<svg width="20" height="20" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M12 2v4"/><path d="M12 18v4"/><path d="M4.93 4.93l2.83 2.83"/><path d="M16.24 16.24l2.83 2.83"/></svg>',
  };

  return `
    <div class="page-header">
      <h2>System Tools</h2>
    </div>

    <div class="grid-container">
      ${toolCard("flush-dns", "Flush Cache", "Clear stale and poisoned DNS entries from local resolver.", icons.flush, "#007AFF", "rgba(0, 122, 255, 0.1)", "none", !capabilities.supports_flush_dns)}
      ${toolCard("renew-dhcp", "Renew DHCP", "Force local interface to request a fresh IP lease.", icons.dhcp, "#10B981", "rgba(16, 185, 129, 0.1)", "none", !capabilities.supports_dhcp_renew)}
      <select id="reset-adapter-select" class="visually-hidden">${state.adapters.map((a) => `<option value="${escapeHtml(a.id)}" ${a.id === state.selectedAdapterId ? "selected" : ""}>${escapeHtml(a.name)}</option>`).join("")}</select>
      ${toolCard("reset-adapter", "Reset Adapter", "Soft-restart the primary network interface card.", icons.power, "#F59E0B", "rgba(245, 158, 11, 0.1)", "none", !capabilities.supports_adapter_reset)}
      <input id="traceroute-host" class="visually-hidden" value="1.1.1.1" />
      ${toolCard("traceroute", "Traceroute", "Map node route paths and measure transit delays.", icons.trace, "#8B5CF6", "rgba(139, 92, 246, 0.1)", "none", !capabilities.supports_traceroute)}
      ${toolCard("export-data", "Export Backup", "Save current targets and app settings to disk.", icons.export)}
      ${toolCard("import-data", "Import Backup", "Load configurations from an existing backup file.", icons.import)}
      ${toolCard("clear-logs", "Clear Logs", "Erase all connection history and audit trails.", icons.trash)}
      ${toolCard("factory-reset", "Factory Reset", "Purge all settings and custom network profiles.", icons.reset, "#EF4444", "rgba(239, 68, 68, 0.1)", "none")}
      <div class="card col-span-4 row-span-1 flex-col" style="padding:0;">
        <div class="flex justify-between items-center" style="padding: 16px 24px; border-bottom: 1px solid var(--border-light); background: var(--bg-surface-hover);">
          <div class="card-label" style="margin: 0; font-size: 11px;">Connection Audit Log</div>
          <div class="text-primary" style="font-size: 12px; font-weight: 600;">Last 20 Events</div>
        </div>
        <div class="card-content">${histRows || '<div style="padding: 48px; text-align: center; color:var(--text-muted); font-size:14px;">No DNS changes recorded yet.</div>'}</div>
      </div>
    </div>
  `;
}