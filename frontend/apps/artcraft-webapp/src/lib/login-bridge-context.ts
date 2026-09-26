export const LOGIN_BRIDGE_PATH = "/login/desktop";
const STORAGE_KEY = "artcraft_pending_desktop_approval";
const TOKEN_PATTERN = /^[A-Za-z0-9_-]{43}$/;

/** Capture the fragment before referral tracking or any third-party code can see it. */
export function captureLoginBridgeContext(): string | null {
  if (window.location.pathname !== LOGIN_BRIDGE_PATH) return null;
  const fragment = new URLSearchParams(window.location.hash.slice(1));
  const supplied = fragment.get("approval_token");
  if (supplied !== null) {
    // Remove even malformed input from the URL. It must not become attribution data.
    window.history.replaceState(window.history.state, "", LOGIN_BRIDGE_PATH);
    sessionStorage.removeItem(STORAGE_KEY);
    if (TOKEN_PATTERN.test(supplied)) sessionStorage.setItem(STORAGE_KEY, supplied);
  }
  const stored = sessionStorage.getItem(STORAGE_KEY);
  return stored && TOKEN_PATTERN.test(stored) ? stored : null;
}

export function clearLoginBridgeContext(): void {
  sessionStorage.removeItem(STORAGE_KEY);
}

export function safeAuthReturnPath(value: string | null): string {
  if (!value || !value.startsWith("/") || value.startsWith("//") || /[\\\u0000-\u001f]/.test(value)) return "/";
  return value;
}
