import type { ViewContext } from "../main.js";

export function renderAbout(ctx: ViewContext): string {
  const { state } = ctx;
  const version = "0.1.0";
  const autoUpdate = state.config.settings.auto_update;

  return `
    <div class="page-header">
      <h2>About SparkDns</h2>
    </div>

    <div class="grid-container" style="grid-template-rows: 1fr;">
      <div class="card col-span-4 row-span-3" style="padding: 0;">
        <div class="card-content">
          <div class="list-row" style="padding: 28px 24px;">
            <div class="flex items-center" style="gap: 16px;">
              <img src="icon.png" alt="SparkDns" style="width: 48px; height: 48px; border-radius: 14px; filter: drop-shadow(0 2px 8px rgba(0,229,255,0.3));">
              <div>
                <div style="font-weight: 600; font-size: 18px; color: var(--text-primary); margin-bottom: 2px;">SparkDns</div>
                <div class="text-secondary" style="font-size: 13px;">v${version} &middot; Cross-platform DNS Manager</div>
              </div>
            </div>
          </div>

          <div class="list-row" style="padding: 20px 24px;">
            <div>
              <div style="font-weight: 600; font-size: 15px; margin-bottom: 4px; color: var(--text-primary);">Check for Updates</div>
              <div class="text-secondary" style="font-size: 13px;">Check if a newer version of SparkDns is available.</div>
            </div>
            <button class="btn" data-action="check-update" style="white-space: nowrap;">Check Now</button>
          </div>

          <div class="list-row" style="padding: 20px 24px;">
            <div>
              <div style="font-weight: 600; font-size: 15px; margin-bottom: 4px; color: var(--text-primary);">Auto-Update</div>
              <div class="text-secondary" style="font-size: 13px;">Automatically download and install updates in the background.</div>
            </div>
            <label class="toggle">
              <input type="checkbox" data-setting="auto_update" ${autoUpdate ? "checked" : ""} />
              <span class="toggle-slider"></span>
            </label>
          </div>

          <div class="list-row" style="padding: 20px 24px;">
            <div>
              <div style="font-weight: 600; font-size: 15px; margin-bottom: 4px; color: var(--text-primary);">Source Code</div>
              <div class="text-secondary" style="font-size: 13px;">View the project on GitHub. Contributions welcome.</div>
            </div>
            <button class="btn" data-action="open-github" style="white-space: nowrap;">GitHub</button>
          </div>

          <div class="list-row" style="padding: 20px 24px; border-bottom: none;">
            <div>
              <div style="font-weight: 600; font-size: 15px; margin-bottom: 4px; color: var(--text-primary);">License</div>
              <div class="text-secondary" style="font-size: 13px;">MIT License &middot; Free and open source software.</div>
            </div>
            <button class="btn" data-action="open-license" style="white-space: nowrap;">View License</button>
          </div>

          <div style="text-align: center; padding: 20px 24px; border-top: 1px solid var(--border-light);">
            <div class="text-muted" style="font-size: 12px;">Built with Tauri &middot; Rust &middot; TypeScript</div>
            <div class="text-muted" style="font-size: 11px; margin-top: 4px;">&copy; 2026 SparkDns. All rights reserved.</div>
          </div>
        </div>
      </div>
    </div>
  `;
}
