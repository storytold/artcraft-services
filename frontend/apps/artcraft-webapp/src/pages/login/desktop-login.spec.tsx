import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";
import { DesktopLogin } from "./desktop-login";
import { safeAuthReturnPath } from "../../lib/login-bridge-context";

const { transport, auth } = vi.hoisted(() => ({ transport: vi.fn(), auth: { loggedIn: true } }));
vi.mock("@storyteller/tauri-utils", () => ({ FetchProxy: transport }));
vi.mock("../../lib/session", () => ({ useSession: () => ({ loggedIn: auth.loggedIn, authChecked: true }) }));
vi.mock("@storyteller/api", async () => ({
  ...(await import("../../../../../libs/api/src/lib/LoginChallengesApi")),
  HttpApiError: (await import("../../../../../libs/api/src/lib/ApiManager")).HttpApiError,
}));

const APPROVAL_TOKEN = "A".repeat(43);
const REVIEW = {
  success: true, status: "pending", maybe_failure_type: null,
  confirmation_code: "WDJBMJHT", requesting_ip: "2001:db8::42",
  username: "google_user", expires_at: new Date(Date.now() + 1_200_000).toISOString(),
};

beforeEach(() => {
  auth.loggedIn = true;
  transport.mockReset();
  sessionStorage.clear();
  localStorage.clear();
  window.history.replaceState(null, "", `/login/desktop#approval_token=${APPROVAL_TOKEN}`);
  transport.mockImplementation(async (url: string, init: RequestInit) => {
    const body = JSON.parse(String(init.body));
    if (url.endsWith("/review")) return response(REVIEW);
    return response({ success: true, status: body.approve ? "approved" : "failed", maybe_failure_type: body.approve ? null : "user_declined" });
  });
});
afterEach(cleanup);

describe("website login approval integration", () => {
  it("reviews the real API response and requires consent plus matching code before approval", async () => {
    renderPage();
    expect(await screen.findByText("google_user")).toBeTruthy();
    expect(screen.getByText("2001:db8::42")).toBeTruthy();
    expect(screen.getByText("WDJB-MJHT")).toBeTruthy();
    const approve = screen.getByRole("button", { name: "Approve desktop login" }) as HTMLButtonElement;
    expect(approve.disabled).toBe(true);
    expect(transport).toHaveBeenCalledTimes(1);
    expect(window.location.hash).toBe("");
    fireEvent.click(screen.getByRole("checkbox"));
    fireEvent.click(approve);
    await screen.findByText("Desktop login approved. Return to ArtCraft to finish signing in.");
    expect(JSON.parse(transport.mock.calls[1][1].body)).toEqual({ approval_token: APPROVAL_TOKEN, approve: true });
    expect(localStorage.getItem("artcraft_signed_session")).toBeNull();
    expect(sessionStorage.getItem("artcraft_pending_desktop_approval")).toBeNull();
  });

  it("records decline without requiring the match checkbox or issuing a session", async () => {
    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: "Decline" }));
    await screen.findByText("Desktop login declined. No session was created.");
    expect(JSON.parse(transport.mock.calls[1][1].body).approve).toBe(false);
    expect(localStorage.getItem("artcraft_signed_session")).toBeNull();
  });

  it("preserves approval through login without putting a credential in the login URL", async () => {
    auth.loggedIn = false;
    const first = renderPage();
    await screen.findByText("/login?from=%2Flogin%2Fdesktop");
    expect(transport).not.toHaveBeenCalled();
    expect(sessionStorage.getItem("artcraft_pending_desktop_approval")).toBe(APPROVAL_TOKEN);
    first.unmount();
    auth.loggedIn = true;
    renderPage();
    await screen.findByText("google_user");
    expect(JSON.parse(transport.mock.calls[0][1].body)).toEqual({ approval_token: APPROVAL_TOKEN });
    expect(screen.queryByText("Desktop login approved.")).toBeNull();
  });

  it("prevents approving an expired request", async () => {
    transport.mockResolvedValue(response({ ...REVIEW, expires_at: new Date(Date.now() - 1000).toISOString() }));
    renderPage();
    await screen.findByRole("alert");
    expect(screen.queryByRole("button", { name: "Approve desktop login" })).toBeNull();
    expect(transport).toHaveBeenCalledTimes(1);
  });

  it("shows an invalid challenge without redirecting forever", async () => {
    transport.mockResolvedValue(new Response("{}", { status: 401 }));
    renderPage();
    await screen.findByText(/request or account session is no longer valid/);
    await waitFor(() => expect(transport).toHaveBeenCalledTimes(1));
  });

  it("rejects external or protocol-relative authentication return URLs", () => {
    for (const path of ["https://evil.example", "//evil.example", "/\\evil.example", "/\nevil.example"]) {
      expect(safeAuthReturnPath(path)).toBe("/");
    }
    expect(safeAuthReturnPath("/login/desktop")).toBe("/login/desktop");
  });
});

function renderPage() {
  return render(<MemoryRouter initialEntries={["/login/desktop"]}><Routes>
    <Route path="/login/desktop" element={<DesktopLogin />} />
    <Route path="/login" element={<LoginLocation />} />
  </Routes></MemoryRouter>);
}

function LoginLocation() {
  const location = useLocation();
  return <p>{location.pathname}{location.search}</p>;
}

function response(body: unknown) {
  return new Response(JSON.stringify(body), { status: 200, headers: { "Content-Type": "application/json" } });
}
