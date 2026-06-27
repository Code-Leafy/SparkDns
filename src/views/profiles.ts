import type { ViewContext } from "../main.js";
import { escapeHtml } from "../main.js";
import { formatDns, renderLatency, renderReachability } from "./prototype.js";

function rowSearchText(parts: Array<string | null | undefined>): string {
  return escapeHtml(parts.filter(Boolean).join(" ").toLowerCase());
}

export function renderProfiles(ctx: ViewContext): string {
  const { state } = ctx;
  const profiles = state.config.profiles;
  const activeId = state.config.active_profile_id;
  const connected = state.config.is_connected;
  const loading = state.loading;
  const probing = state.probing;

  const profileRows = profiles
    .map((p) => {
      const metrics = state.metrics[p.id];
      const active = p.id === activeId && connected;
      // A profile shows skeletons on both columns while it is being probed.
      const isProbing = probing.includes(p.id);
      return `
        <div class="list-row profile-row" data-search="${rowSearchText([p.name, p.primary_ipv4, p.secondary_ipv4])}" style="${active ? "background: rgba(16, 185, 129, 0.05);" : ""}">
          <div>
            <div class="flex items-center gap-2 mb-1">
              <span style="font-weight: 600; font-size: 15px; color: var(--text-primary);">${escapeHtml(p.name)}</span>
            </div>
            <div class="font-mono text-secondary" style="font-size: 13px;">${formatDns(p)}</div>
          </div>
          <div class="flex items-center gap-8">
            <div style="text-align:right; font-size: 13px;">${renderLatency(metrics, isProbing)} <span style="color:var(--border-light); margin:0 12px;">|</span> ${renderReachability(metrics, isProbing)}</div>
            <div class="flex items-center gap-2">
              <button class="btn-icon" data-action="toggle-favorite" data-profile-id="${p.id}" title="Favorite"><svg viewBox="0 0 24 24" width="18" height="18" fill="${p.favorite ? "var(--text-primary)" : "none"}" stroke="currentColor" stroke-width="2"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg></button>
              <button class="btn-icon" data-action="evaluate" data-profile-id="${p.id}" title="Ping" ${isProbing ? "disabled" : ""}><svg class="${isProbing ? "spinning" : ""}" viewBox="0 0 24 24" width="18" height="18" stroke="currentColor" fill="none" stroke-width="2"><path d="M21 2v6h-6M3 12a9 9 0 0 1 15-6.7L21 8M3 22v-6h6M21 12a9 9 0 0 1-15 6.7L3 16"/></svg></button>
              ${p.preset ? "" : `<button class="btn-icon" data-action="delete-profile" data-profile-id="${p.id}" title="Delete"><svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg></button>`}
              ${active ? `<button class="btn" disabled style="width: 110px; margin-left: 8px;">Active</button>` : `<button class="btn btn-primary" data-action="connect" data-profile-id="${p.id}" ${loading ? "disabled" : ""} style="width: 110px; margin-left: 8px;">Apply</button>`}
            </div>
          </div>
        </div>
      `;
    })
    .join("");

  return `
    <div class="page-header">
      <h2>Network Library</h2>
    </div>

    <div class="grid-container" style="grid-template-rows: 1fr;">
      <div class="card col-span-4 row-span-3" style="padding: 0;">
        <div class="flex justify-between items-center" style="padding: 16px 24px; border-bottom: 1px solid var(--border-light);">
          <div class="card-label" style="margin:0;">Available Configurations</div>
        </div>
        <div class="card-content" style="padding-bottom: 90px;">
          ${profileRows || '<p class="sub" style="padding: 24px;">No DNS profiles configured.</p>'}
        </div>
      </div>
    </div>

    <div class="floating-action-bar">
      <div class="search-wrapper">
        <svg viewBox="0 0 24 24" stroke="currentColor" fill="none" stroke-width="2"><circle cx="11" cy="11" r="8"/><path d="M21 21l-4.3-4.3"/></svg>
        <input id="profile-search" type="text" placeholder="Search targets..." />
      </div>
      <button class="fab-icon-btn" data-action="refresh-pings" title="Refresh Pings" ${loading ? "disabled" : ""}>
        <svg class="${loading ? "spinning" : ""}" viewBox="0 0 24 24" width="18" height="18" stroke="currentColor" fill="none" stroke-width="2.5"><path d="M21 2v6h-6M3 12a9 9 0 0 1 15-6.7L21 8M3 22v-6h6M21 12a9 9 0 0 1-15 6.7L3 16"/></svg>
      </button>
      <button class="btn btn-primary" data-action="toggle-add-profile-modal">Add Custom Node</button>
    </div>
  `;
}