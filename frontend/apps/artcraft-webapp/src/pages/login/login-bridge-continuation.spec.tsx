import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";
import Login from "./login";
import { AuthLayout } from "../../components/auth/auth-layout";
import Signup from "../signup/signup";
import ForgotPassword from "../forgot-password/forgot-password";
import VerifyReset from "../forgot-password/verify-reset";

const { transport, refresh, billing, auth } = vi.hoisted(() => ({
  transport: vi.fn(), refresh: vi.fn(), billing: vi.fn(), auth: { loggedIn: false },
}));
vi.mock("@storyteller/tauri-utils", () => ({ FetchProxy: transport }));
vi.mock("@storyteller/api", async () => ({
  ...(await import("../../../../../libs/api/src/lib/UsersApi")),
  ...(await import("../../../../../libs/api/src/lib/PasswordResetApi")),
}));
vi.mock("../../lib/session", () => ({
  refreshSession: refresh,
  useSession: () => ({ loggedIn: auth.loggedIn, authChecked: true }),
}));
vi.mock("../../lib/billing", () => ({ hasActiveSubscription: billing }));
vi.mock("../../components/auth", async () => ({
  ...(await import("../../components/auth/GoogleLoginButton")),
  ...(await import("../../components/auth/signup-form")),
  AuthHeader: () => null,
  AuthFooter: ({ children }: any) => <div>{children}</div>,
}));
vi.mock("@react-oauth/google", () => ({ GoogleLogin: ({ onSuccess }: any) => <button onClick={() => onSuccess({ credential: "google_test_credential" })}>Google sign in</button> }));
vi.mock("@storyteller/common", () => ({ getLandingUrl: () => undefined, getReferralCode: () => undefined, getReferralUsername: () => undefined, getReferrer: () => undefined }));
vi.mock("@storyteller/icons", () => ({ DynamicIcon: () => null }));
vi.mock("@storyteller/ui-button", () => ({ Button: ({ children, ...props }: any) => <button {...props}>{children}</button> }));
vi.mock("@storyteller/ui-input", () => ({ Input: ({ inputClassName, isError, ...props }: any) => <input {...props} /> }));
vi.mock("../../components/seo", () => ({ default: () => null }));
vi.mock("../../components/auth/auth-showcase", () => ({ AuthShowcase: () => null }));
vi.mock("../../components/auth/auth-page-frame", () => ({ AuthPageFrame: ({ children }: any) => <div>{children}</div> }));
vi.mock("../../components/ui/use-media-query", () => ({ useMediaQuery: () => false }));

beforeEach(() => {
  auth.loggedIn = false;
  transport.mockReset();
  refresh.mockReset().mockResolvedValue(undefined);
  billing.mockReset().mockResolvedValue(false);
  transport.mockImplementation(async () => new Response(JSON.stringify({ success: true, signed_session: "signed_web_session" }), { status: 200 }));
  localStorage.clear();
  sessionStorage.clear();
  sessionStorage.setItem("artcraft_pending_desktop_approval", "A".repeat(43));
});
afterEach(cleanup);

describe("webapp authentication bridge continuation", () => {
  it("returns an unsubscribed Google user to explicit desktop consent without passing the bridge token to SSO", async () => {
    renderRoutes(<Login />);
    fireEvent.click(screen.getByRole("button", { name: "Google sign in" }));
    await screen.findByText("/login/desktop");
    expect(refresh).toHaveBeenCalledWith(true);
    expect(billing).not.toHaveBeenCalled();
    expect(JSON.parse(transport.mock.calls[0][1].body)).toEqual({ google_credential: "google_test_credential" });
    expect(sessionStorage.getItem("artcraft_pending_desktop_approval")).toBe("A".repeat(43));
  });

  it("preserves the consent destination when the auth layout sees the refreshed session first", async () => {
    auth.loggedIn = true;
    renderRoutes(<AuthLayout />);
    await screen.findByText("/login/desktop");
    expect(transport).not.toHaveBeenCalled();
  });

  it("preserves the destination through signup and Google signup without checking billing", async () => {
    renderRoutes(<Login />);
    fireEvent.click(screen.getByRole("link", { name: "Sign up" }));
    expect(screen.getByRole("link", { name: "Log in" }).getAttribute("href")).toBe("/login?from=%2Flogin%2Fdesktop");
    fireEvent.click(screen.getByRole("button", { name: "Google sign in" }));
    await screen.findByText("/login/desktop");
    expect(refresh).toHaveBeenCalledWith(true);
    expect(billing).not.toHaveBeenCalled();
    expect(transport).toHaveBeenCalledTimes(1);
  });

  it("returns password login to consent", async () => {
    renderRoutes(<Login />);
    fireEvent.change(screen.getByPlaceholderText("you@example.com or username"), { target: { value: "user@example.com" } });
    fireEvent.change(screen.getByPlaceholderText("Min. 8 characters"), { target: { value: "password123" } });
    fireEvent.click(screen.getByRole("button", { name: "Log in" }));
    await screen.findByText("/login/desktop");
    expect(refresh).toHaveBeenCalledWith(true);
    expect(billing).not.toHaveBeenCalled();
    expect(transport.mock.calls[0][1].body).not.toContain("A".repeat(43));
  });

  it("preserves the destination through the password reset form and redemption", async () => {
    renderRoutes(<Login />);
    fireEvent.click(screen.getByRole("link", { name: "Forgot password?" }));
    expect(screen.getByRole("link", { name: /Back to Log in/ }).getAttribute("href")).toBe("/login?from=%2Flogin%2Fdesktop");
    fireEvent.change(screen.getByPlaceholderText("you@example.com"), { target: { value: "user@example.com" } });
    fireEvent.click(screen.getByRole("button", { name: "Send Reset Code" }));
    fireEvent.click(await screen.findByRole("link", { name: "Enter Verification Code" }));
    fireEvent.change(screen.getByPlaceholderText("Enter verification code"), { target: { value: "test_reset_code" } });
    const passwords = screen.getAllByPlaceholderText(/password|characters/i);
    passwords.forEach((input) => fireEvent.change(input, { target: { value: "password123" } }));
    fireEvent.click(screen.getByRole("button", { name: "Change Password" }));
    fireEvent.click(await screen.findByRole("button", { name: "Review desktop login" }));
    await screen.findByText("/login/desktop");
    expect(refresh).toHaveBeenCalledWith(true);
    expect(transport).toHaveBeenCalledTimes(2);
    expect(billing).not.toHaveBeenCalled();
  });
});

function renderRoutes(login: React.ReactNode) {
  return render(<MemoryRouter initialEntries={["/login?from=%2Flogin%2Fdesktop"]}><Routes>
    <Route path="/login" element={login} />
    <Route path="/signup" element={<Signup />} />
    <Route path="/forgot-password" element={<ForgotPassword />} />
    <Route path="/forgot-password/verify" element={<VerifyReset />} />
    <Route path="*" element={<Location />} />
  </Routes></MemoryRouter>);
}
function Location() {
  return <p>{useLocation().pathname}</p>;
}
