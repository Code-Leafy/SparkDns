import type { DnsProfile } from "../types.js";
import type { ViewContext } from "../main.js";
import { escapeHtml } from "../main.js";
import { formatDns, generateSparklineSVG, getSpeedStyles, renderLatency } from "./prototype.js";

function pickConnectTarget(profiles: DnsProfile[]): DnsProfile | undefined {
  return profiles.find((p) => p.favorite) ?? profiles.find((p) => p.name.toLowerCase().includes("cloudflare")) ?? profiles[0];
}

function rankedProfiles(ctx: ViewContext): DnsProfile[] {
  return [...ctx.state.config.profiles]
    .filter((p) => ctx.state.metrics[p.id])
    .sort((a, b) => {
      const ma = ctx.state.metrics[a.id];
      const mb = ctx.state.metrics[b.id];
      if (ma.reachability_percent !== mb.reachability_percent) return mb.reachability_percent - ma.reachability_percent;
      return (ma.latency_ms ?? Number.POSITIVE_INFINITY) - (mb.latency_ms ?? Number.POSITIVE_INFINITY);
    })
    .slice(0, 2);
}

function renderRecommendations(ctx: ViewContext): string {
  const ranked = rankedProfiles(ctx);
  if (ctx.state.loading && ranked.length < 2) {
    return [1, 2].map(() => `<div class="card col-span-1 row-span-1 skeleton" style="min-height:0;"></div>`).join("");
  }

  return ranked
    .map((p, idx) => {
      const metrics = ctx.state.metrics[p.id];
      const speed = getSpeedStyles(metrics.latency_ms);
      const ms = metrics.latency_ms === null || metrics.latency_ms === undefined ? "--" : Math.round(metrics.latency_ms);
      return `
        <div class="card col-span-1 row-span-1 interactive flex-col justify-between" data-action="connect" data-profile-id="${p.id}">
          <div class="card-label">${idx === 0 ? "Top Choice" : "Alternative"}</div>
          <div class="flex-col">
            <h4 style="font-size: 15px; font-weight: 600; margin-bottom: 2px;">${escapeHtml(p.name)}</h4>
            <span class="font-mono text-muted" style="font-size: 12px;">${formatDns(p)}</span>
          </div>
          <div class="flex items-end justify-between mt-auto">
            <div style="color:${speed.color}; text-shadow: ${speed.glow}; font-size:24px; font-weight:700; letter-spacing: -0.02em;">${ms}<span style="font-size: 14px;">ms</span></div>
            <div style="font-size: 10px; color:var(--text-muted); font-weight:700; letter-spacing: 0.05em;">${metrics.reachability_percent.toFixed(0)}% REACH</div>
          </div>
        </div>`;
    })
    .join("");
}

function renderFavorites(ctx: ViewContext): string {
  const favorites = ctx.state.config.profiles.filter((p) => p.favorite);
  return favorites
    .map((p) => `
      <div class="list-row interactive" data-action="connect" data-profile-id="${p.id}" style="padding: 12px 16px; border:none; margin-bottom:4px; border-radius: 8px;">
        <span style="font-weight: 600; font-size: 14px; color: var(--text-primary);">${escapeHtml(p.name)}</span>
        <div class="flex items-center" style="gap: 8px;">
          ${renderLatency(ctx.state.metrics[p.id], ctx.state.loading)}
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" class="text-muted"><line x1="5" y1="12" x2="19" y2="12"></line><polyline points="12 5 19 12 12 19"></polyline></svg>
        </div>
      </div>`)
    .join("");
}

export function renderHome(ctx: ViewContext): string {
  const { state, capabilities } = ctx;
  const active = state.config.profiles.find((p) => p.id === state.config.active_profile_id);
  const connected = state.config.is_connected && !!active;
  const connectTarget = pickConnectTarget(state.config.profiles);
  const activeMetrics = active ? state.metrics[active.id] : undefined;
  const latencyValue = connected && activeMetrics?.latency_ms !== null && activeMetrics?.latency_ms !== undefined ? Math.round(activeMetrics.latency_ms) : "--";
  const recommendations = renderRecommendations(ctx);
  const favorites = renderFavorites(ctx);

  return `
    <div class="page-header">
      <h2>Overview</h2>
    </div>

    <div class="grid-container">
      <div class="card col-span-2 row-span-2 flex-col" style="padding: 32px;">
        <div class="card-label" style="margin-bottom: 24px;">Current Connection</div>
        <div class="flex items-center" style="gap: 12px; margin-bottom: 32px;">
          <div class="status-orb ${connected ? "active" : ""}" style="width: 12px; height: 12px;"></div>
          <h3 style="font-size: 24px; font-weight: 600; color: var(--text-primary); margin: 0;">${connected ? "Custom DNS Active" : "System Default"}</h3>
        </div>

        <div class="flex-col" style="flex:1;">
          <div class="flex justify-between items-center" style="padding: 16px 0; border-bottom: 1px solid var(--border-light);">
            <span class="text-secondary" style="font-size: 13px;">Active Node</span>
            <span class="text-primary font-weight-500 font-sans" style="font-size: 14px; font-weight: 700;">${active ? escapeHtml(active.name) : "ISP Managed"}</span>
          </div>
          <div class="flex justify-between items-center" style="padding: 16px 0; border-bottom: 1px solid var(--border-light);">
            <span class="text-secondary" style="font-size: 13px;">IPv4 Address</span>
            <span class="text-primary font-weight-500 font-sans" style="font-size: 14px; font-weight: 700;">${active ? formatDns(active) : "Auto (DHCP)"}</span>
          </div>
          <div class="flex justify-between items-center" style="padding: 16px 0; border-bottom: 1px solid var(--border-light);">
            <span class="text-secondary" style="font-size: 13px;">Encryption</span>
            <span class="flex items-center" style="gap: 8px;">
              ${state.config.settings.doh_enabled && connected && capabilities.supports_doh
                ? `<svg viewBox="0 0 24 24" width="18" height="18" fill="none" style="filter: drop-shadow(0 0 6px rgba(50, 215, 75, 0.7)) drop-shadow(0 0 12px rgba(50, 215, 75, 0.4));"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" fill="var(--success)" stroke="var(--success)" stroke-width="1.5" stroke-linejoin="round"/><path d="M9 12l2 2 4-4" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>`
                : `<svg viewBox="0 0 24 24" width="18" height="18" fill="none" style="filter: drop-shadow(0 0 4px rgba(107, 114, 128, 0.5));"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" fill="var(--border-highlight)" stroke="var(--text-muted)" stroke-width="1.5" stroke-linejoin="round"/><path d="M9 12l2 2 4-4" stroke="var(--bg-surface)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>`}
              <span class="text-primary font-weight-500 font-sans" style="font-size: 14px; font-weight: 700;">${state.config.settings.doh_enabled && connected && capabilities.supports_doh ? `${state.config.settings.dns_encryption_type === "doh" ? "DoH" : "DoT"} Active` : "None"}</span>
            </span>
          </div>
        </div>

        <div style="margin-top:auto; padding-top:24px;">
          ${connected
            ? `<button class="btn" data-action="disconnect" style="width:100%; height:44px; font-size:14px;">Disconnect Node</button>`
            : `<button class="btn btn-primary" data-action="connect" data-profile-id="${connectTarget?.id ?? ""}" style="width:100%; height:44px; font-size:14px;" ${connectTarget ? "" : "disabled"}>Apply DNS</button>`}
        </div>
      </div>

      <div class="card col-span-2 row-span-1" style="padding-bottom:0;">
        <div class="card-label">Resolver Latency</div>
        <div class="flex items-baseline gap-1" style="margin-top: 4px;">
          <span style="font-size: 28px; font-weight: 700; letter-spacing: -0.02em;">${latencyValue}</span>
          <span style="color:var(--text-muted); font-weight:600; font-size: 14px;">ms</span>
        </div>
        <div class="chart-container" style="flex:1; display:flex; align-items:flex-end;">${connected ? generateSparklineSVG() : '<div style="margin:auto; padding-bottom:24px; color:var(--text-muted); font-size:12px;">Apply a profile, then run diagnostics to measure latency.</div>'}</div>
      </div>

      <div class="card col-span-1 row-span-1 flex-col justify-center">
        <div class="card-label">DNS Changes</div>
        <div style="font-size: 28px; font-weight: 700; letter-spacing: -0.02em; margin-bottom: 4px;" id="live-queries">${connected ? state.config.history.length.toLocaleString() : "0"}</div>
        <div style="font-size: 12px; color:var(--text-muted);">Profiles applied this session</div>
      </div>

      <div class="card col-span-1 row-span-1 flex-col justify-center">
        <div class="card-label">Session Uptime</div>
        <div style="font-size: 28px; font-weight: 700; letter-spacing: -0.02em; margin-bottom: 4px;" id="live-uptime">${connected ? "Live" : "--"}</div>
        <div style="font-size: 12px; color:var(--text-muted);">Continuous tunneling</div>
      </div>

      ${recommendations}

      <div class="card col-span-2 row-span-1" style="padding: 0;">
        <div class="card-label" style="padding: 24px 24px 0 24px; margin-bottom:8px;">Quick Connect (Favorites)</div>
        <div class="card-content" style="padding: 0 12px 12px 12px;">
          ${favorites || '<div style="text-align:center; padding:20px; color:var(--text-muted); font-size:13px;">Star profiles in the Library.</div>'}
        </div>
      </div>
    </div>
  `;
}