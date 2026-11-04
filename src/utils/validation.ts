import type { DnsProfile, ValidationResult } from "../types.js";

const IPV4_OCTET = "(25[0-5]|2[0-4]\\d|1?\\d?\\d)";
const IPV4_RE = new RegExp(`^${IPV4_OCTET}(\\.${IPV4_OCTET}){3}$`);

// Simplified but reasonable IPv6 validation (RFC 4291 subset).
const IPV6_RE =
  /^(([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}|(::([0-9a-fA-F]{1,4}:){0,5}[0-9a-fA-F]{1,4})|(([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5})|(([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4})|(([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3})|(([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2})|(([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4})|(([0-9a-fA-F]{1,4}:){1,7}:))$/;

export function isValidIpv4(value: string): boolean {
  return IPV4_RE.test(value.trim());
}

export function isValidIpv6(value: string): boolean {
  const v = value.trim();
  if (!v) return false;
  return IPV6_RE.test(v);
}

export function isValidIp(value: string): boolean {
  return isValidIpv4(value) || isValidIpv6(value);
}

export function isValidDohUrl(value: string): boolean {
  const v = value.trim();
  if (!v) return false;
  try {
    const url = new URL(v);
    return url.protocol === "https:";
  } catch {
    return false;
  }
}

export function isValidHostname(value: string): boolean {
  const v = value.trim();
  if (!v || v.length > 253) return false;
  return /^[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$/.test(
    v,
  );
}

export function validateProfileInput(
  input: Partial<DnsProfile>,
  existingIds: readonly string[] = [],
): ValidationResult {
  const errors: string[] = [];

  const name = (input.name ?? "").trim();
  if (name.length < 1 || name.length > 80) {
    errors.push("Name must be 1-80 characters.");
  }

  if (!input.primary_ipv4 || !isValidIpv4(input.primary_ipv4)) {
    errors.push("Primary IPv4 is required and must be a valid IPv4 address.");
  }

  if (input.secondary_ipv4 && !isValidIpv4(input.secondary_ipv4)) {
    errors.push("Secondary IPv4 must be a valid IPv4 address when provided.");
  }

  if (input.primary_ipv6 && !isValidIpv6(input.primary_ipv6)) {
    errors.push("Primary IPv6 must be a valid IPv6 address when provided.");
  }

  if (input.secondary_ipv6 && !isValidIpv6(input.secondary_ipv6)) {
    errors.push("Secondary IPv6 must be a valid IPv6 address when provided.");
  }

  if (input.doh_url && !isValidDohUrl(input.doh_url)) {
    errors.push("DNS over HTTPS URL must use the https:// protocol.");
  }

  if (input.dot_host && !isValidHostname(input.dot_host)) {
    errors.push("DNS-over-TLS host must be a valid hostname.");
  }

  if (input.id && existingIds.includes(input.id)) {
    errors.push("A profile with this id already exists.");
  }

  return { ok: errors.length === 0, errors };
}

export function isFeatureHidden(
  hidden: readonly string[],
  key: string,
): boolean {
  return hidden.includes(key);
}