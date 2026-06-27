import type { DnsMetrics, DnsProfile } from "../types.js";
import { escapeHtml } from "../main.js";

export function formatDns(profile: DnsProfile): string {
  return profile.secondary_ipv4 ? `${escapeHtml(profile.primary_ipv4)} &bull; ${escapeHtml(profile.secondary_ipv4)}` : escapeHtml(profile.primary_ipv4);
}

export function plainDns(profile: DnsProfile): string {
  return profile.secondary_ipv4 ? `${profile.primary_ipv4} ${profile.secondary_ipv4}` : profile.primary_ipv4;
}

export function getSpeedStyles(ms: number | null | undefined): { color: string; glow: string } {
  if (ms === null || ms === undefined) return { color: "var(--text-muted)", glow: "none" };
  if (ms < 30) return { color: "var(--success)", glow: "0 0 8px rgba(16,185,129,0.5), 0 0 16px rgba(16,185,129,0.3)" };
  if (ms < 60) return { color: "var(--warning)", glow: "0 0 8px rgba(245,158,11,0.5), 0 0 16px rgba(245,158,11,0.3)" };
  return { color: "var(--danger)", glow: "0 0 8px rgba(239,68,68,0.5), 0 0 16px rgba(239,68,68,0.3)" };
}

export function renderLatency(metrics: DnsMetrics | undefined, loading: boolean): string {
  if (loading && !metrics) return `<span class="skeleton-inline" style="width: 44px; height: 14px;"></span>`;
  if (!metrics || metrics.latency_ms === null || metrics.latency_ms === undefined) return `<span class="text-muted">--</span>`;
  const ms = Math.round(metrics.latency_ms);
  const speed = getSpeedStyles(ms);
  return `<span class="metric-reveal" style="color:${speed.color}; text-shadow: ${speed.glow}; font-weight:600;">${ms}ms</span>`;
}

export function renderReachability(metrics: DnsMetrics | undefined, loading: boolean): string {
  if (loading && !metrics) return `<span class="skeleton-inline" style="width: 60px; height: 14px;"></span>`;
  if (!metrics) return `<span class="text-muted" style="font-weight:500;">--</span>`;
  const pct = metrics.reachability_percent;
  const color = pct >= 99 ? "var(--success)" : pct > 0 ? "var(--warning)" : "var(--danger)";
  return `<span class="metric-reveal" style="font-weight:600; color:${color};">${pct.toFixed(0)}% REACH</span>`;
}

export function generateSparklineSVG(): string {
  const points: string[] = [];
  let current = 25;
  for (let i = 0; i < 20; i += 1) {
    current += Math.random() * 8 - 4;
    if (current < 5) current = 5;
    if (current > 45) current = 45;
    points.push(`${i * 15},${50 - current}`);
  }
  return `<svg class="chart-svg" viewBox="0 0 285 55" preserveAspectRatio="none"><polyline class="chart-line" points="${points.join(" ")}" /></svg>`;
}

export function generateAnimatedChartSVG(color: string, seed = 1): string {
  const points: string[] = [];
  let current = 25;
  for (let i = 0; i <= 20; i += 1) {
    current += Math.sin(i * seed) * 8 + (Math.random() * 6 - 3);
    if (current < 5) current = 5;
    if (current > 45) current = 45;
    points.push(`${i * 15},${55 - current}`);
  }
  return `<svg viewBox="0 0 300 60" preserveAspectRatio="none" style="width:100%; height:100%; display:block; color:${color}; filter: drop-shadow(0 0 6px currentColor);"><polyline fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" stroke-dasharray="1000" stroke-dashoffset="1000" style="animation: drawChart 1.5s cubic-bezier(0.25, 1, 0.5, 1) forwards;" points="${points.join(" ")}" /></svg>`;
}

export function formatDate(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? escapeHtml(value) : date.toLocaleString();
}