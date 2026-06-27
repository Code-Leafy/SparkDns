const IS_TAURI =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function sendNotification(
  title: string,
  body: string,
): Promise<void> {
  if (!IS_TAURI) return;
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("send_notification", { title, body });
  } catch {
    // Silent fail — OS notification is best-effort
  }
}
