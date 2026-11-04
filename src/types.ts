export type OsKind = "windows" | "linux" | "macos";

export interface DnsProfile {
  id: string;
  name: string;
  primary_ipv4: string;
  secondary_ipv4: string | null;
  primary_ipv6: string | null;
  secondary_ipv6: string | null;
  doh_url: string | null;
  dot_host: string | null;
  favorite: boolean;
  preset: boolean;
}

export type Theme = "dark" | "light";

export interface DiagnosticTarget {
  id: string;
  name: string;
  host: string;
  icon?: string;
}

export interface AppSettings {
  theme: Theme;
  doh_enabled: boolean;
  dns_encryption_type: "doh" | "dot";
  ipv6_enabled: boolean;
  minimize_to_tray: boolean;
  start_on_boot: boolean;
  active_adapter_id: string | null;
  diagnostic_targets: DiagnosticTarget[];
  auto_ping_enabled: boolean;
  auto_update: boolean;
}

export interface PlatformCapabilities {
  os: OsKind;
  arch: string;
  supports_ipv4_dns: boolean;
  supports_ipv6_dns: boolean;
  supports_doh: boolean;
  supports_dns_apply: boolean;
  supports_flush_dns: boolean;
  supports_dhcp_renew: boolean;
  supports_adapter_reset: boolean;
  supports_traceroute: boolean;
  supports_app_rules: boolean;
  supports_auto_switch: boolean;
  requires_elevation_for_dns: boolean;
  dns_backend: string;
  hidden_features: string[];
}

export interface NetworkAdapter {
  id: string;
  name: string;
  description: string | null;
  is_up: boolean;
  is_primary: boolean;
  ipv4_dns: string[];
  ipv6_dns: string[];
}

export interface DnsApplyRequest {
  profile_id: string;
  adapter_id: string | null;
  primary_ipv4: string;
  secondary_ipv4: string | null;
  primary_ipv6: string | null;
  secondary_ipv6: string | null;
  enable_doh: boolean;
}

export interface CommandResult {
  ok: boolean;
  message: string;
  requires_elevation: boolean;
  details: string | null;
}

export interface DnsMetrics {
  profile_id: string;
  server: string;
  latency_ms: number | null;
  reachability_percent: number;
  packet_loss_percent: number;
  tested_at: string;
  error: string | null;
}

export interface TargetProbeResult {
  id: string;
  name: string;
  host: string;
  latency_ms: number | null;
  reachable: boolean;
  error: string | null;
}

export interface DiagnosticResult {
  latency_ms: number | null;
  dnssec_valid: boolean | null;
  leak_secure: boolean | null;
  packet_loss_percent: number;
  targets: TargetProbeResult[];
}

export interface AppRule {
  id: string;
  app_name: string;
  app_path: string | null;
  profile_id: string;
  enabled: boolean;
}

export interface AutoSwitchRule {
  id: string;
  app_label: string;
  match_name: string;
  app_path: string | null;
  profile_id: string;
  enabled: boolean;
}

export interface RunningProcess {
  name: string;
  path: string | null;
}


export interface HistoryEntry {
  name: string;
  ip: string;
  time: string;
}

export interface AppConfig {
  active_profile_id: string | null;
  is_connected: boolean;
  profiles: DnsProfile[];
  history: HistoryEntry[];
  rules: AppRule[];
  auto_switch_rules: AutoSwitchRule[];
  auto_switch_enabled: boolean;
  settings: AppSettings;
  schema_version: number;
}

export type SystemToolKey =
  | "flush_cache"
  | "reset_dhcp"
  | "reset_adapter"
  | "run_traceroute";

export interface TracerouteResult {
  ok: boolean;
  host: string;
  hops: TracerouteHop[];
  raw: string | null;
  error: string | null;
}

export interface TracerouteHop {
  index: number;
  host: string | null;
  latency_ms: number | null;
}

export interface ValidationResult {
  ok: boolean;
  errors: string[];
}

export type ViewKey =
  | "home"
  | "profiles"
  | "rules"
  | "autoswitch"
  | "diagnostics"
  | "tools"
  | "settings"
  | "about";

export interface DiagnosticState {
  running: boolean;
  result: DiagnosticResult | null;
  targetResults: Record<string, { latency_ms: number | null; reachable: boolean }>;
}

export interface AppRuntimeState {
  config: AppConfig;
  capabilities: PlatformCapabilities | null;
  adapters: NetworkAdapter[];
  selectedAdapterId: string | null;
  metrics: Record<string, DnsMetrics>;
  probing: string[];
  loading: boolean;
  statusMessage: string;
  statusKind: "info" | "success" | "error";
  diagnostics: DiagnosticState;
}