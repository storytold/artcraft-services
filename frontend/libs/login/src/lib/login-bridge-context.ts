export const LOGIN_BRIDGE_PATH = "/login/desktop";
const STORAGE_KEY = "artcraft_pending_desktop_approval";
const TOKEN_PATTERN = /^[A-Za-z0-9_-]{43}$/;
let memoryToken: string | null = null;

/** Capture the fragment before referral tracking or any third-party code can see it. */
export function captureLoginBridgeContext(): string | null {
  if (window.location.pathname !== LOGIN_BRIDGE_PATH) return null;
  const fragment = new URLSearchParams(window.location.hash.slice(1));
  const supplied = fragment.get("approval_token");
  if (window.location.hash) {
    // Remove even malformed input from the URL. It must not become attribution data.
    window.history.replaceState(window.history.state, "", LOGIN_BRIDGE_PATH);
    clearLoginBridgeContext();
    if (supplied && TOKEN_PATTERN.test(supplied)) {
      memoryToken = supplied;
      try { sessionStorage.setItem(STORAGE_KEY, supplied); } catch { /* Storage disabled; keep the current tab working. */ }
    }
  }
  let stored = memoryToken;
  try { stored = sessionStorage.getItem(STORAGE_KEY) ?? memoryToken; } catch { /* Storage disabled. */ }
  return stored && TOKEN_PATTERN.test(stored) ? stored : null;
}

export function clearLoginBridgeContext(): void {
  memoryToken = null;
  try { sessionStorage.removeItem(STORAGE_KEY); } catch { /* Storage disabled. */ }
}

/** Carry only a validated destination, never the approval credential, through auth. */
export function authContinuationUrl(path: string, from: string | null): string {
  const destination = safeAuthReturnPath(from);
  return destination === "/" ? path : `${path}?from=${encodeURIComponent(destination)}`;
}

export function safeAuthReturnPath(value: string | null): string {
  if (!value || !value.startsWith("/") || value.startsWith("//") || /[\\\u0000-\u001f]/.test(value)) return "/";
  return value;
}
