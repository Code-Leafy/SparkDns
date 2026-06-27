import type {
  AppConfig,
  CommandResult,
  DiagnosticResult,
  DiagnosticTarget,
  DnsApplyRequest,
  DnsMetrics,
  DnsProfile,
  NetworkAdapter,
  PlatformCapabilities,
  SystemToolKey,
  TargetProbeResult,
  TracerouteResult,
  AutoSwitchRule,
  RunningProcess
} from "./types.js";
import { DEFAULT_CONFIG } from "./defaults.js";

const IS_TAURI =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function invoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (IS_TAURI) {
    const tauriApi = await import("@tauri-apps/api/core");
    return tauriApi.invoke<T>(command, args);
  }
  return devFallback<T>(command, args);
}

function normalizeCapabilities(
  capabilities: PlatformCapabilities,
): PlatformCapabilities {
  return {
    ...capabilities,
    supports_dns_apply:
      capabilities.supports_dns_apply ?? capabilities.supports_ipv4_dns,
  };
}

// Clearly marked browser-only development fallback. Never used in production builds.
async function devFallback<T>(
  command: string,
  _args?: Record<string, unknown>,
): Promise<T> {
  switch (command) {
    case "get_capabilities":
      return {
        os: "windows",
        arch: "x86_64",
        supports_ipv4_dns: true,
        supports_ipv6_dns: true,
        supports_doh: true,
        supports_dns_apply: true,
        supports_flush_dns: true,
        supports_dhcp_renew: true,
        supports_adapter_reset: true,
        supports_traceroute: true,
        supports_app_rules: false,
        supports_auto_switch: true,
        requires_elevation_for_dns: true,
        dns_backend: "windows-netsh",
        hidden_features: ["app_rules"],
      } as unknown as T;
    case "get_config":
      return structuredClone(DEFAULT_CONFIG) as unknown as T;
    case "list_adapters":
      return [
        {
          id: "dev-adapter",
          name: "Development Adapter",
          description: "Browser dev fallback adapter",
          is_up: true,
          is_primary: true,
          ipv4_dns: ["8.8.8.8"],
          ipv6_dns: [],
        },
      ] as unknown as T;
    default:
      return { ok: false, message: "Backend unavailable in browser dev mode.", requires_elevation: false, details: null } as unknown as T;
  }
}

export async function loadPlatformCapabilities(): Promise<PlatformCapabilities> {
  const capabilities = await invoke<PlatformCapabilities>("get_capabilities");
  return normalizeCapabilities(capabilities);
}

export async function loadConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("get_config");
}

export async function saveConfig(config: AppConfig): Promise<CommandResult> {
  return invoke<CommandResult>("save_config", { config });
}

export async function listAdapters(): Promise<NetworkAdapter[]> {
  return invoke<NetworkAdapter[]>("list_adapters");
}

export async function applyDns(
  profile: DnsProfile,
  adapterId: string | null,
  enableDoh: boolean,
  enableIpv6: boolean,
): Promise<CommandResult> {
  const request: DnsApplyRequest = {
    profile_id: profile.id,
    adapter_id: adapterId,
    primary_ipv4: profile.primary_ipv4,
    secondary_ipv4: profile.secondary_ipv4,
    primary_ipv6: enableIpv6 ? profile.primary_ipv6 : null,
    secondary_ipv6: enableIpv6 ? profile.secondary_ipv6 : null,
    enable_doh: enableDoh,
  };
  return invoke<CommandResult>("apply_dns", { req: request });
}

export async function clearDns(
  adapterId: string | null,
): Promise<CommandResult> {
  return invoke<CommandResult>("clear_dns", { adapterId });
}

export async function evaluateDns(
  profile: DnsProfile,
  adapterId?: string | null,
): Promise<DnsMetrics> {
  return invoke<DnsMetrics>("probe_server", {
    server: profile.primary_ipv4,
    adapterId: adapterId ?? null,
  });
}

export async function comprehensiveCheck(
  _targets: DiagnosticTarget[],
): Promise<DiagnosticResult> {
  return invoke<DiagnosticResult>("comprehensive_check");
}

export async function runSystemTool(
  tool: SystemToolKey,
  args?: { adapterId?: string | null; host?: string },
): Promise<CommandResult | TracerouteResult> {
  if (tool === "run_traceroute") {
    return invoke<TracerouteResult>("run_traceroute", {
      host: args?.host ?? "1.1.1.1",
    });
  }
  const command =
    tool === "flush_cache"
      ? "flush_dns_cache"
      : tool === "reset_dhcp"
        ? "renew_dhcp"
        : "reset_adapter";
  return invoke<CommandResult>(command, { adapterId: args?.adapterId ?? null });
}

export async function pingTarget(host: string, adapterId?: string | null): Promise<TargetProbeResult> {
  return invoke<TargetProbeResult>("ping_target", {
    host,
    adapterId: adapterId ?? null,
  });
}

export async function exportConfigJson(): Promise<string> {
  return invoke<string>("export_config");
}

export async function importConfigJson(json: string): Promise<AppConfig> {
  return invoke<AppConfig>("import_config", { json });
}

export async function setAutostart(enabled: boolean): Promise<void> {
  return invoke<void>("set_autostart", { enabled });
}

export async function isAutostartEnabled(): Promise<boolean> {
  try {
    return await invoke<boolean>("is_autostart_enabled");
  } catch {
    return false;
  }
}

export async function addAutoSwitchRule(rule: AutoSwitchRule): Promise<void> {
  return invoke<void>("add_auto_switch_rule", { rule });
}

export async function removeAutoSwitchRule(ruleId: string): Promise<void> {
  return invoke<void>("remove_auto_switch_rule", { ruleId });
}

export async function toggleAutoSwitchRule(ruleId: string, enabled: boolean): Promise<void> {
  return invoke<void>("toggle_auto_switch_rule", { ruleId, enabled });
}

export async function setAutoSwitchEnabled(enabled: boolean): Promise<void> {
  return invoke<void>("set_auto_switch_enabled", { enabled });
}

export async function listRunningProcesses(): Promise<RunningProcess[]> {
  return invoke<RunningProcess[]>("list_running_processes");
}

export { IS_TAURI };