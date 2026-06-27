import type { ViewContext } from "../main.js";
import { escapeHtml } from "../main.js";

export function renderAutoSwitch(ctx: ViewContext): string {
  const { state, capabilities } = ctx;

  if (!capabilities.supports_auto_switch) {
    return `
      <div class="page-header">
        <h2>Auto-Switch</h2>
        <span class="tag">Hidden when unsupported</span>
      </div>
      <div class="grid-container">
        <div class="card col-span-4 row-span-1">
          <div class="card-label">Unavailable</div>
          <div class="card-content justify-between">
            <p class="text-error">Background DNS auto-switching is not supported on this platform backend.</p>
          </div>
        </div>
      </div>
    `;
  }

  const rules = state.config.auto_switch_rules;
  const profiles = state.config.profiles;

  const rows = rules
    .map((r) => {
      const options = profiles.map((p) => `<option value="${p.id}" ${p.id === r.profile_id ? "selected" : ""}>${escapeHtml(p.name)}</option>`).join("");
      return `
        <div class="list-row" style="padding: 16px 24px; border-bottom: 1px solid var(--border-light); background: var(--bg-surface);">
          <div class="flex items-center gap-4">
            <label class="toggle">
              <input type="checkbox" data-action="toggle-auto-rule" data-rule-id="${r.id}" ${r.enabled ? "checked" : ""}>
              <span class="toggle-slider"></span>
            </label>
            <div class="flex-col">
              <span style="font-weight: 600; font-size: 15px; color: var(--text-primary); margin-bottom: 2px;">${escapeHtml(r.app_label)}</span>
              <span class="text-muted font-mono" style="font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 250px;">${escapeHtml(r.match_name)} ${r.app_path ? `(${escapeHtml(r.app_path)})` : ""}</span>
            </div>
          </div>
          <div class="flex items-center gap-4">
            <div style="position: relative;">
              <select class="rule-select" data-action="update-auto-rule-profile" data-rule-id="${r.id}" style="appearance: none; padding: 10px 36px 10px 16px; border-radius: 10px; background: var(--bg-surface-elevated); border: 1px solid var(--border-highlight); color: var(--text-primary); font-weight: 500; font-size: 13px; cursor: pointer; outline: none; min-width: 140px; box-shadow: 0 2px 4px rgba(0,0,0,0.02);">${options}</select>
              <svg viewBox="0 0 24 24" width="14" height="14" stroke="var(--text-muted)" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round" style="position: absolute; right: 12px; top: 50%; transform: translateY(-50%); pointer-events: none;"><polyline points="6 9 12 15 18 9"></polyline></svg>
            </div>
            <button class="btn-icon" data-action="delete-auto-rule" data-rule-id="${r.id}" style="color: var(--text-muted);"><svg width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg></button>
          </div>
        </div>
      `;
    })
    .join("");

  return `
    <div class="page-header">
      <h2>Auto-Switch</h2>
    </div>

    <div class="grid-container" style="grid-template-rows: 1fr;">
      <div class="card col-span-4 row-span-3" style="padding: 0;">
        <div class="card-label" style="padding: 16px 24px 0;">DNS Triggers</div>
        <div class="card-content" style="padding: 0 0 90px;">${rules.length === 0 ? '<div style="padding: 48px; text-align: center; color:var(--text-muted); font-size:14px;">No auto-switch rules configured. Add an app to switch DNS automatically when it runs.</div>' : rows}</div>
      </div>
    </div>

    <div class="floating-action-bar">
      <div class="flex items-center gap-2">
        <span style="font-size: 13px; color: var(--text-muted); white-space:nowrap;">Watcher</span>
        <label class="toggle">
          <input type="checkbox" data-action="toggle-auto-switch-master" ${state.config.auto_switch_enabled ? "checked" : ""}>
          <span class="toggle-slider"></span>
        </label>
      </div>
      <button class="btn btn-secondary" data-action="add-auto-rule-running" style="padding: 8px 16px; font-size:13px;">Pick Running App</button>
      <button class="btn btn-primary" data-action="add-auto-rule-browse" style="padding: 8px 16px; font-size:13px;">Browse Executable</button>
    </div>
  `;
}
