// Themed in-app dialogs that replace native prompt/confirm/alert, which the
// Tauri webview blocks. Each returns a Promise resolved by user interaction.

type DialogField = { id: string; label: string; placeholder?: string; value?: string };

let dialogHost: HTMLElement | null = null;

export function ensureDialogHost(): HTMLElement {
  if (dialogHost && document.body.contains(dialogHost)) return dialogHost;
  dialogHost = document.createElement("div");
  dialogHost.id = "app-dialog-host";
  document.body.appendChild(dialogHost);
  return dialogHost;
}

function escapeAttr(value: string): string {
  return value.replace(/[&<>"']/g, (ch) => `&#${ch.charCodeAt(0)};`);
}

/** A themed confirmation dialog. Resolves true when confirmed. */
export function confirmDialog(opts: {
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
}): Promise<boolean> {
  return new Promise((resolve) => {
    const host = ensureDialogHost();
    const confirmLabel = opts.confirmLabel ?? "Confirm";
    const cancelLabel = opts.cancelLabel ?? "Cancel";
    host.innerHTML = `
      <div class="modal-overlay active" id="__confirm_overlay">
        <div class="modal-content" role="dialog" aria-modal="true">
          <h3>${escapeAttr(opts.title)}</h3>
          <p class="text-secondary" style="font-size:13px; line-height:1.5; margin-bottom:8px;">${escapeAttr(opts.message)}</p>
          <div class="modal-actions">
            <button class="btn" id="__confirm_cancel">${escapeAttr(cancelLabel)}</button>
            <button class="btn ${opts.danger ? "btn-danger" : "btn-primary"}" id="__confirm_ok">${escapeAttr(confirmLabel)}</button>
          </div>
        </div>
      </div>`;
    const overlay = host.querySelector<HTMLElement>("#__confirm_overlay");
    const cleanup = (result: boolean) => {
      host.innerHTML = "";
      resolve(result);
    };
    host.querySelector("#__confirm_ok")?.addEventListener("click", () => cleanup(true));
    host.querySelector("#__confirm_cancel")?.addEventListener("click", () => cleanup(false));
    overlay?.addEventListener("click", (e) => {
      if (e.target === overlay) cleanup(false);
    });
  });
}

/** A themed prompt dialog with one or more text fields. Resolves the field
 * values keyed by id, or null when cancelled. */
export function promptDialog(opts: {
  title: string;
  fields: DialogField[];
  confirmLabel?: string;
}): Promise<Record<string, string> | null> {
  return new Promise((resolve) => {
    const host = ensureDialogHost();
    const fieldsHtml = opts.fields
      .map(
        (f) =>
          `<label>${escapeAttr(f.label)}<input class="input" type="text" id="__pf_${f.id}" placeholder="${escapeAttr(f.placeholder ?? "")}" value="${escapeAttr(f.value ?? "")}"></label>`,
      )
      .join("");
    host.innerHTML = `
      <div class="modal-overlay active" id="__prompt_overlay">
        <div class="modal-content" role="dialog" aria-modal="true">
          <h3>${escapeAttr(opts.title)}</h3>
          ${fieldsHtml}
          <div class="modal-actions">
            <button class="btn" id="__prompt_cancel">Cancel</button>
            <button class="btn btn-primary" id="__prompt_ok">${escapeAttr(opts.confirmLabel ?? "Save")}</button>
          </div>
        </div>
      </div>`;
    const overlay = host.querySelector<HTMLElement>("#__prompt_overlay");
    const cleanup = (result: Record<string, string> | null) => {
      host.innerHTML = "";
      resolve(result);
    };
    host.querySelector("#__prompt_ok")?.addEventListener("click", () => {
      const values: Record<string, string> = {};
      for (const f of opts.fields) {
        values[f.id] = (host.querySelector<HTMLInputElement>(`#__pf_${f.id}`)?.value ?? "").trim();
      }
      cleanup(values);
    });
    host.querySelector("#__prompt_cancel")?.addEventListener("click", () => cleanup(null));
    overlay?.addEventListener("click", (e) => {
      if (e.target === overlay) cleanup(null);
    });
    host.querySelector<HTMLInputElement>(`#__pf_${opts.fields[0]?.id}`)?.focus();
  });
}
