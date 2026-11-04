type ToastKind = "info" | "success" | "error";

interface Toast {
  id: number;
  message: string;
  kind: ToastKind;
  timer: number;
}

let counter = 0;
const toasts = new Map<number, Toast>();
let container: HTMLElement | null = null;

const ICONS: Record<ToastKind, string> = {
  success:
    '<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>',
  error:
    '<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>',
  info:
    '<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>',
};

function ensureContainer(): HTMLElement {
  if (container && document.body.contains(container)) return container;
  container = document.createElement("div");
  container.className = "toast-container";
  document.body.appendChild(container);
  return container;
}

function buildNode(toast: Toast): HTMLElement {
  const node = document.createElement("div");
  node.className = `toast ${toast.kind}`;
  node.dataset.toastId = String(toast.id);
  node.innerHTML = `
    <span class="toast-icon">${ICONS[toast.kind]}</span>
    <span class="toast-message"></span>
    <button class="toast-close" aria-label="Dismiss">
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="6" y1="6" x2="18" y2="18"/><line x1="18" y1="6" x2="6" y2="18"/></svg>
    </button>`;
  const msg = node.querySelector(".toast-message");
  if (msg) msg.textContent = toast.message;
  node.querySelector(".toast-close")?.addEventListener("click", () => dismiss(toast.id));
  return node;
}

function dismiss(id: number): void {
  const toast = toasts.get(id);
  if (!toast) return;
  window.clearTimeout(toast.timer);
  const el = container?.querySelector<HTMLElement>(`[data-toast-id="${id}"]`);
  if (el) {
    el.classList.add("toast-leaving");
    el.addEventListener(
      "animationend",
      () => {
        toasts.delete(id);
        el.remove();
      },
      { once: true },
    );
  } else {
    toasts.delete(id);
  }
}

export function showToast(message: string, kind: ToastKind = "info"): void {
  const el = ensureContainer();
  const id = ++counter;
  const timer = window.setTimeout(() => dismiss(id), 4200);
  const toast: Toast = { id, message, kind, timer };
  toasts.set(id, toast);
  el.appendChild(buildNode(toast));
}

export function clearToasts(): void {
  for (const id of [...toasts.keys()]) dismiss(id);
}