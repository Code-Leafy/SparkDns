import type {
  AppConfig,
  DnsMetrics,
  DnsProfile,
  NetworkAdapter,
  PlatformCapabilities,
  DiagnosticResult,
} from "./types.js";
import { DEFAULT_CONFIG } from "./defaults.js";

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

export type StateListener = (state: AppRuntimeState) => void;

class StateStore {
  private state: AppRuntimeState = {
    config: structuredClone(DEFAULT_CONFIG),
    capabilities: null,
    adapters: [],
    selectedAdapterId: null,
    metrics: {},
    probing: [],
    loading: false,
    statusMessage: "",
    statusKind: "info",
    diagnostics: { running: false, result: null, targetResults: {} },
  };

  private listeners = new Set<StateListener>();
  private batchDepth = 0;
  private dirty = false;

  get(): AppRuntimeState {
    return this.state;
  }

  set(patch: Partial<AppRuntimeState>): void {
    this.state = { ...this.state, ...patch };
    this.emit();
  }

  setConfig(config: AppConfig): void {
    this.state = { ...this.state, config };
    this.emit();
  }

  setStatus(
    message: string,
    kind: "info" | "success" | "error" = "info",
  ): void {
    this.state = { ...this.state, statusMessage: message, statusKind: kind };
    this.emit();
  }

  setLoading(loading: boolean): void {
    this.state = { ...this.state, loading };
    this.emit();
  }

  setDiagnostics(d: Partial<DiagnosticState>): void {
    this.state = { ...this.state, diagnostics: { ...this.state.diagnostics, ...d } };
    this.emit();
  }

  upsertProfile(profile: DnsProfile): void {
    const profiles = [...this.state.config.profiles];
    const idx = profiles.findIndex((p) => p.id === profile.id);
    if (idx >= 0) {
      profiles[idx] = profile;
    } else {
      profiles.push(profile);
    }
    this.setConfig({ ...this.state.config, profiles });
  }

  removeProfile(id: string): void {
    const profiles = this.state.config.profiles.filter((p) => p.id !== id);
    const nextActiveProfileId =
      this.state.config.active_profile_id === id
        ? null
        : this.state.config.active_profile_id;
    this.setConfig({
      ...this.state.config,
      profiles,
      active_profile_id: nextActiveProfileId,
    });
  }

  setMetrics(metrics: DnsMetrics): void {
    this.state = {
      ...this.state,
      metrics: { ...this.state.metrics, [metrics.profile_id]: metrics },
    };
    this.emit();
  }

  /** Mark a set of profile ids as currently probing (shows skeletons). */
  setProbing(ids: string[]): void {
    this.state = { ...this.state, probing: ids };
    this.emit();
  }

  /** Remove a single profile id from the probing set. */
  clearProbing(id: string): void {
    this.state = { ...this.state, probing: this.state.probing.filter((p) => p !== id) };
    this.emit();
  }

  /** Start batching — defers emit until endBatch(). Nestable. */
  beginBatch(): void {
    this.batchDepth++;
  }

  /** End batching — emit once if any changes were made. */
  endBatch(): void {
    this.batchDepth--;
    if (this.batchDepth <= 0) {
      this.batchDepth = 0;
      if (this.dirty) {
        this.dirty = false;
        this.emit();
      }
    }
  }

  subscribe(listener: StateListener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private emit(): void {
    if (this.batchDepth > 0) {
      this.dirty = true;
      return;
    }
    for (const listener of this.listeners) {
      listener(this.state);
    }
  }
}

export const store = new StateStore();