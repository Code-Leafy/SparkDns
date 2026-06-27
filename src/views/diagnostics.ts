import type { ViewContext } from "../main.js";
import { escapeHtml } from "../main.js";
import { getSpeedStyles } from "./prototype.js";

interface BrandIcon {
  src: string;
  color: string;
  mask?: boolean;
}

const BRAND_ICONS: Record<string, BrandIcon> = {
  cloudflare: { src: "/cloudflare.svg", color: "#F38020" },
  "epic games": { src: "/epicgames.svg", color: "var(--text-primary)", mask: true },
  gemini: { src: "/googlegemini.svg", color: "#8AB4F8" },
  steam: { src: "/steam.svg", color: "#148CD2" },
  youtube: { src: "/youtube.svg", color: "#FF0000" },
  chatgpt: { src: "/chatgpt.svg", color: "#10A37F", mask: true },
  github: { src: "/github.svg", color: "var(--text-primary)", mask: true },
};

export { BRAND_ICONS };

function renderBrandIcon(name: string): string {
  const key = name.toLowerCase();
  const brand = BRAND_ICONS[key];
  if (!brand) {
    return `<span class="brand-icon" style="display:flex;align-items:center;justify-content:center;font-size:18px;font-weight:800;color:var(--text-secondary);">${escapeHtml(name.slice(0, 1))}</span>`;
  }

  if (brand.mask) {
    return `<div class="brand-icon" style="background-color: ${brand.color}; -webkit-mask: url('${brand.src}') center/contain no-repeat; mask: url('${brand.src}') center/contain no-repeat;"></div>`;
  }
  return `<img class="brand-icon" src="${brand.src}" alt="${escapeHtml(name)}">`;
}

export function renderDiagnostics(ctx: ViewContext): string {
  const { state } = ctx;
  const targets = state.config.settings.diagnostic_targets;
  const diag = state.diagnostics;

  const targetCards = targets
    .map((t) => {
      const key = t.name.toLowerCase();
      const brand = BRAND_ICONS[key];
      const color = brand?.color ?? "var(--text-secondary)";
      const tr = diag.targetResults[t.id];
      let targetClass = "brand-target";
      let latencyHtml = "";
      if (diag.running && !tr) {
        targetClass += " probing";
      } else if (tr) {
        targetClass += tr.reachable ? " success" : " fail";
        if (tr.latency_ms !== null) {
          const ping = Math.round(tr.latency_ms);
          const speed = getSpeedStyles(tr.latency_ms);
          latencyHtml = `<div class="target-latency show" style="color:${speed.color}; text-shadow:${speed.glow};">${ping}ms</div>`;
        } else {
          latencyHtml = `<div class="target-latency show" style="color:var(--danger);">ERR</div>`;
        }
      }
      return `
        <div class="${targetClass}" id="tgt-${t.id}" style="--brand-color:${color};">
          <div class="brand-icon-wrapper">${renderBrandIcon(t.name)}</div>
          <div class="brand-name">${escapeHtml(t.name)}</div>
          ${latencyHtml}
        </div>
      `;
    })
    .join("");

  const result = diag.result;
  const reachableCount = result ? result.targets.filter((t: { reachable: boolean }) => t.reachable).length : 0;
  const reachPct = result ? (reachableCount / Math.max(1, result.targets.length)) * 100 : 0;

  function renderMetric(id: string, label: string, valHtml: string, isGood: boolean | null, txt: string): string {
    const color = isGood === null ? "var(--text-muted)" : isGood ? "var(--text-primary)" : "var(--danger)";
    const statColor = isGood === null ? "var(--text-muted)" : isGood ? "var(--success)" : "var(--danger)";
    return `
      <div style="padding-bottom:12px;">
        <div class="card-label">${label}</div>
        <div class="mt-2 flex-col" id="diag-${id}">
          <div class="val text-primary" style="font-size:28px; font-weight:700; color:${color};">${valHtml}</div>
          <div class="stat" style="font-size:13px; font-weight:500; margin-top:4px; color:${statColor};">${txt}</div>
        </div>
      </div>`;
  }

  let latencyHtml: string;
  let reachHtml: string;
  let dnssecHtml: string;
  let leakHtml: string;

  if (diag.running) {
    const skeleton = `<div class="skeleton-inline" style="width:70px; height:32px;"></div>`;
    latencyHtml = renderMetric("latency", "Latency", skeleton, null, "Probing...");
    reachHtml = renderMetric("reach", "Reachability", skeleton, null, "Probing...");
    dnssecHtml = renderMetric("dnssec", "DNSSEC", skeleton, null, "Probing...");
    leakHtml = renderMetric("leak", "Leak Protect", skeleton, null, "Probing...");
  } else if (result) {
    const hasLatency = result.latency_ms !== null && result.latency_ms !== undefined;
    latencyHtml = hasLatency
      ? renderMetric("latency", "Latency", `${Math.round(result.latency_ms!)}<span style="font-size:16px;color:var(--text-secondary);">ms</span>`, (result.latency_ms!) < 50, (result.latency_ms!) < 50 ? "Optimal Route" : "Reachable")
      : renderMetric("latency", "Latency", `<span style="color:var(--text-muted);">--</span>`, null, "No targets reachable");
    reachHtml = renderMetric("reach", "Reachability", `${Math.round(reachPct)}<span style="font-size:16px;color:var(--text-secondary);">%</span>`, reachPct >= 99, reachPct >= 99 ? "All targets responsive" : `${reachableCount}/${result.targets.length} reachable`);
    dnssecHtml = result.dnssec_valid !== null && result.dnssec_valid !== undefined
      ? renderMetric("dnssec", "DNSSEC", result.dnssec_valid ? "Valid" : "Off", result.dnssec_valid, result.dnssec_valid ? "Validating resolver" : "Resolver not validating")
      : renderMetric("dnssec", "DNSSEC", `<span style="color:var(--text-muted);">--</span>`, null, "Not measured");
    leakHtml = result.leak_secure !== null && result.leak_secure !== undefined
      ? renderMetric("leak", "Leak Protect", result.leak_secure ? "Secure" : "Exposed", result.leak_secure, result.leak_secure ? "Using public resolver" : "Router/ISP resolver in use")
      : renderMetric("leak", "Leak Protect", `<span style="color:var(--text-muted);">--</span>`, null, "Not measured");
  } else {
    latencyHtml = renderMetric("latency", "Latency", "--", null, "Awaiting probe...");
    reachHtml = renderMetric("reach", "Reachability", "--", null, "Awaiting probe...");
    dnssecHtml = renderMetric("dnssec", "DNSSEC", "--", null, "Awaiting probe...");
    leakHtml = renderMetric("leak", "Leak Protect", "--", null, "Awaiting probe...");
  }

  return `
    <div class="page-header">
      <h2>Diagnostics</h2>
      <button class="btn btn-primary" id="run-diag-btn" data-action="run-diagnostics" ${diag.running ? "disabled" : ""}>${diag.running ? "Analyzing..." : "Initiate Scan"}</button>
    </div>

    <div class="grid-container">
      <div class="card col-span-4 row-span-1 justify-center">
        <div class="card-label">Target Reachability Validation</div>
        <div class="brand-grid" style="padding: 10px 24px;">${targetCards || '<p class="sub">No diagnostic targets configured.</p>'}</div>
      </div>

      <div class="card col-span-1 row-span-2 flex-col justify-between" style="overflow:hidden; padding-bottom:0;">
        ${latencyHtml}
      </div>

      <div class="card col-span-1 row-span-2 flex-col justify-between" style="overflow:hidden; padding-bottom:0;">
        ${reachHtml}
      </div>

      <div class="card col-span-1 row-span-2 flex-col justify-between" style="overflow:hidden; padding-bottom:0;">
        ${dnssecHtml}
      </div>

      <div class="card col-span-1 row-span-2 flex-col justify-between" style="overflow:hidden; padding-bottom:0;">
        ${leakHtml}
      </div>
    </div>
  `;
}
