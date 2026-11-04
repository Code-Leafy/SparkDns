import { IS_TAURI } from "./api.js";

interface GitHubReleaseAsset {
  name: string;
  browser_download_url: string;
  size: number;
}

interface GitHubRelease {
  tag_name: string;
  name: string;
  html_url: string;
  assets: GitHubReleaseAsset[];
}

export interface UpdateInfo {
  hasUpdate: boolean;
  version: string;
  downloadUrl: string;
}

function parseVersion(tag: string): number[] {
  return tag.replace(/^v/i, "").split(".").map(Number);
}

function isNewer(latest: string, current: string): boolean {
  const a = parseVersion(latest);
  const b = parseVersion(current);
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    const x = a[i] ?? 0;
    const y = b[i] ?? 0;
    if (x > y) return true;
    if (x < y) return false;
  }
  return false;
}

function findExeAsset(assets: GitHubReleaseAsset[]): GitHubReleaseAsset | undefined {
  return assets.find((a) => /x64[-_]?setup\.exe$/i.test(a.name) || /win(dows)?[-_]?x?64.*\.exe$/i.test(a.name));
}

export async function checkForUpdate(currentVersion: string): Promise<UpdateInfo> {
  const res = await fetch("https://api.github.com/repos/Code-Leafy/SparkDns/releases/latest", {
    headers: { Accept: "application/vnd.github.v3+json" },
  });
  if (!res.ok) {
    return { hasUpdate: false, version: currentVersion, downloadUrl: "" };
  }
  const release: GitHubRelease = await res.json();
  const latestVersion = release.tag_name;
  const asset = findExeAsset(release.assets);
  const downloadUrl = asset?.browser_download_url ?? release.html_url;
  return {
    hasUpdate: isNewer(latestVersion, currentVersion),
    version: latestVersion.replace(/^v/i, ""),
    downloadUrl,
  };
}

export function getAppVersion(): string {
  if (IS_TAURI) {
    const el = document.querySelector("meta[name='tauri-version']");
    if (el) return el.getAttribute("content") ?? "0.1.0";
  }
  return "0.1.0";
}

export async function downloadAndInstall(url: string): Promise<void> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Download failed: ${res.status}`);
  const blob = await res.blob();
  const buffer = await blob.arrayBuffer();
  const data = new Uint8Array(buffer);

  if (IS_TAURI) {
    const { BaseDirectory, writeFile } = await import("@tauri-apps/plugin-fs");
    const { downloadDir } = await import("@tauri-apps/api/path");
    const fileName = url.split("/").pop() ?? "SparkDns-update.exe";
    const dir = await downloadDir();
    const fullPath = `${dir}\\SparkDns\\${fileName}`;
    await writeFile(`SparkDns\\${fileName}`, data, { baseDir: BaseDirectory.Download });
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("run_installer", { path: fullPath });
  } else {
    const blobUrl = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = blobUrl;
    a.download = url.split("/").pop() ?? "SparkDns-update.exe";
    a.click();
    URL.revokeObjectURL(blobUrl);
  }
}
