import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router-dom";
import { DesktopLogin } from "./desktop-login";
import { invalidateSession, useSessionStore } from "../../lib/session";
import { clearLoginBridgeContext } from "@frontend/login";

const { transport } = vi.hoisted(() => ({ transport: vi.fn() }));
vi.mock("@storyteller/tauri-utils", () => ({ FetchProxy: transport }));
vi.mock("@storyteller/api", async () => ({
  ...(await import("../../../../../libs/api/src/lib/UsersApi")),
  ...(await import("../../../../../libs/api/src/lib/LoginChallengesApi")),
  HttpApiError: (await import("../../../../../libs/api/src/lib/ApiManager")).HttpApiError,
}));
vi.mock("@react-oauth/google", () => ({ GoogleLogin: ({ onSuccess }: any) => <button onClick={() => onSuccess({ credential: "google_test_credential" })}>Google sign in</button> }));
vi.mock("@storyteller/common", () => ({ getLandingUrl: () => undefined, getReferralCode: () => undefined, getReferralUsername: () => undefined, getReferrer: () => undefined }));
vi.mock("@storyteller/icons", () => ({ DynamicIcon: () => null }));
vi.mock("@storyteller/ui-modal", () => ({ Modal: ({ children, isOpen, onClose }: any) => isOpen ? <div role="dialog">{children}<button onClick={onClose}>Close</button></div> : null }));
vi.mock("@storyteller/ui-button", () => ({ Button: ({ children, ...props }: any) => <button {...props}>{children}</button> }));
vi.mock("@storyteller/ui-input", () => ({ Input: ({ inputClassName, ...props }: any) => <input {...props} /> }));

const APPROVAL_TOKEN = "B".repeat(43);
const USER = { username: "google_user", user_token: "u_test" };
const REVIEW = {
  success: true, status: "pending", maybe_failure_type: null,
  confirmation_code: "WDJBMJHT", requesting_ip: "2001:db8::42",
  username: USER.username, expires_at: new Date(Date.now() + 1_200_000).toISOString(),
};
let signedIn = false;
let authFails = false;

beforeEach(() => {
  signedIn = false;
  authFails = false;
  clearLoginBridgeContext();
  sessionStorage.clear();
  localStorage.clear();
  invalidateSession();
  useSessionStore.setState({ user: undefined, loggedIn: false, authChecked: true });
  window.history.replaceState(null, "", `/login/desktop#approval_token=${APPROVAL_TOKEN}`);
  transport.mockReset().mockImplementation(async (url: string, init: RequestInit) => {
    const path = new URL(url).pathname;
    if (path === "/v1/session") return response({ success: true, logged_in: signedIn, user: signedIn ? USER : undefined });
    if (["/v1/login", "/v1/accounts/google_sso", "/v1/create_account"].includes(path)) {
      signedIn = !authFails;
      return response(authFails ? { success: false, error_message: "Invalid credentials" } : { success: true, signed_session: "browser_session" });
    }
    if (path.endsWith("/review")) return response(REVIEW);
    if (path.endsWith("/decide")) {
      const { approve } = JSON.parse(String(init.body));
      return response({ success: true, status: approve ? "approved" : "failed", maybe_failure_type: approve ? null : "user_declined" });
    }
    throw new Error(`Unexpected test request: ${path}`);
  });
});
afterEach(cleanup);

describe("website desktop login with real auth and approval clients", () => {
  it.each(["google", "password"])("keeps %s sign-in on this origin and waits for explicit consent", async (method) => {
    renderPage();
    expect(window.location.hash).toBe("");
    expect(sessionStorage.getItem("artcraft_pending_desktop_approval")).toBe(APPROVAL_TOKEN);
    if (method === "google") {
      fireEvent.click(screen.getByRole("button", { name: "Google sign in" }));
    } else {
      fireEvent.change(screen.getByPlaceholderText("you@example.com"), { target: { value: "user@example.com" } });
      fireEvent.change(screen.getByPlaceholderText("Your password"), { target: { value: "password123" } });
      fireEvent.click(screen.getByRole("button", { name: "Log in" }));
    }
    await screen.findByText(USER.username);
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByText("2001:db8::42")).toBeTruthy();
    expect(screen.getByText("WDJB-MJHT")).toBeTruthy();
    expect(bridgeCalls("decide")).toHaveLength(0);
    expect(window.location.pathname).toBe("/login/desktop");
    const authCall = transport.mock.calls.find(([url]) => /\/(login|google_sso)$/.test(url));
    expect(authCall?.[1].body).not.toContain(APPROVAL_TOKEN);
    const reviewCall = bridgeCalls("review")[0];
    expect(reviewCall[1].headers.session).toBe("browser_session");
    expect(JSON.parse(reviewCall[1].body)).toEqual({ approval_token: APPROVAL_TOKEN });
    fireEvent.click(screen.getByRole("checkbox"));
    fireEvent.click(screen.getByRole("button", { name: "Approve desktop login" }));
    await screen.findByText(/Desktop login approved/);
    expect(bridgeCalls("decide")).toHaveLength(1);
    expect(localStorage.getItem("artcraft_signed_session")).toBe("browser_session");
    expect(sessionStorage.getItem("artcraft_pending_desktop_approval")).toBeNull();
    expect(bridgeCalls("poll")).toHaveLength(0);
  });

  it("lets an existing session decline without another login or code confirmation", async () => {
    signedIn = true;
    useSessionStore.setState({ user: USER as any, loggedIn: true });
    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: "Decline" }));
    await screen.findByText("Desktop login declined. No session was created.");
    expect(JSON.parse(bridgeCalls("decide")[0][1].body)).toEqual({ approval_token: APPROVAL_TOKEN, approve: false });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(localStorage.getItem("artcraft_signed_session")).toBeNull();
  });

  it("keeps a failed sign-in pending and permits a retry", async () => {
    authFails = true;
    renderPage();
    fireEvent.click(screen.getByRole("button", { name: "Google sign in" }));
    await screen.findByText("Invalid credentials");
    expect(bridgeCalls("review")).toHaveLength(0);
    expect(bridgeCalls("decide")).toHaveLength(0);
    expect(sessionStorage.getItem("artcraft_pending_desktop_approval")).toBe(APPROVAL_TOKEN);
    authFails = false;
    fireEvent.click(screen.getByRole("button", { name: "Google sign in" }));
    await screen.findByText(USER.username);
    expect(bridgeCalls("decide")).toHaveLength(0);
  });

  it("closing sign-in preserves the challenge and does not approve or decline it", async () => {
    renderPage();
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
    expect(screen.getByRole("dialog")).toBeTruthy();
    await act(async () => {});
    expect(bridgeCalls("decide")).toHaveLength(0);
    expect(sessionStorage.getItem("artcraft_pending_desktop_approval")).toBe(APPROVAL_TOKEN);
  });

  it("does not show sign-in or make bridge requests for a malformed link", async () => {
    window.history.replaceState(null, "", "/login/desktop#approval_token=bad");
    renderPage();
    await screen.findByText(/Open a new login link/);
    expect(window.location.hash).toBe("");
    expect(screen.queryByRole("dialog")).toBeNull();
    await waitFor(() => expect(bridgeCalls("review")).toHaveLength(0));
  });
});

function renderPage() {
  return render(<MemoryRouter initialEntries={["/login/desktop"]}><DesktopLogin /></MemoryRouter>);
}
function bridgeCalls(action: string) {
  return transport.mock.calls.filter(([url]) => url.endsWith(`/login_challenges/${action}`));
}
function response(body: unknown) {
  return new Response(JSON.stringify(body), { status: 200, headers: { "Content-Type": "application/json" } });
}
