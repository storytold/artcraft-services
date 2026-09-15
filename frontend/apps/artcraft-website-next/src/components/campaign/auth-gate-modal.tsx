"use client";

import { useState, type FormEvent } from "react";
import { CheckIcon, EyeIcon, EyeOffIcon } from "lucide-react";
import { Button, Input, Modal } from "@/components/ui";
import { login, signup } from "@/lib/api";
import {
  getLandingUrl,
  getReferralCode,
  getReferralUsername,
  getReferrer,
} from "@/lib/referral";

// Soft auth gate for campaign pages, ported from the Vite site: keeps the
// visitor on the page, letting them create an account or log in inline and
// then continue whatever action triggered the gate. Email/password only;
// the Google SSO button needs the OAuth provider this app does not mount.
export default function AuthGateModal({
  isOpen,
  onClose,
  onAuthed,
  signupSource,
  headline,
  subtitle,
  perks,
}: {
  isOpen: boolean;
  onClose: () => void;
  /** Called after a successful signup OR login; the session is already set. */
  onAuthed: () => void;
  /** Attribution string passed to the signup endpoint. */
  signupSource: string;
  headline: string;
  subtitle: string;
  perks?: string[];
}) {
  const [mode, setMode] = useState<"signup" | "login">("signup");

  return (
    <Modal isOpen={isOpen} onClose={onClose} childPadding={false} accessibleTitle={headline}>
      <div className="border-b border-line px-6 py-6 md:px-8">
        <p className="hud-label text-faint">
          {mode === "signup" ? "Create a free account" : "Welcome back"}
        </p>
        <h2 className="mt-3 font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
          {headline}
        </h2>
        <p className="mt-2 text-sm leading-relaxed text-muted">{subtitle}</p>
        {perks && perks.length > 0 && (
          <ul className="mt-5 flex flex-col gap-2">
            {perks.map((perk) => (
              <li key={perk} className="flex items-start gap-2.5 text-sm text-ink">
                <CheckIcon aria-hidden className="mt-0.5 h-4 w-4 shrink-0 text-accent-ink" />
                {perk}
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="px-6 py-6 md:px-8">
        {mode === "signup" ? (
          <SignupForm onSuccess={onAuthed} signupSource={signupSource} />
        ) : (
          <LoginForm onSuccess={onAuthed} />
        )}
        <button
          type="button"
          onClick={() => setMode(mode === "signup" ? "login" : "signup")}
          className="mt-5 block w-full text-center text-sm text-muted hover:text-ink"
        >
          {mode === "signup" ? (
            <>
              Already have an account? <span className="text-ink underline underline-offset-4">Log in</span>
            </>
          ) : (
            <>
              New to ArtCraft? <span className="text-ink underline underline-offset-4">Create a free account</span>
            </>
          )}
        </button>
      </div>
    </Modal>
  );
}

function SignupForm({
  onSuccess,
  signupSource,
}: {
  onSuccess: () => void;
  signupSource: string;
}) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    if (!email || !password) return setError("All fields are required");
    if (password.length < 8) return setError("Password must be at least 8 characters");

    setLoading(true);
    // Username generated from the email, same scheme as the Vite form:
    // up to 7 alphanumerics from the local part plus a 3-digit suffix.
    const prefix = email.split("@")[0].replace(/[^a-zA-Z0-9]/g, "").slice(0, 7);
    const username = `${prefix}${Math.floor(Math.random() * 900) + 100}`;
    const result = await signup({
      username,
      email,
      password,
      signupSource,
      maybeReferralUrl: getReferrer(),
      maybeLandingUrl: getLandingUrl(),
      maybeReferralUsername: getReferralUsername(),
      maybeReferralCode: getReferralCode(),
    });
    setLoading(false);
    if (result.success) onSuccess();
    else setError(result.errorMessage || "Failed to create account");
  };

  return (
    <form className="flex flex-col gap-4" onSubmit={submit} noValidate>
      {error && <FormError>{error}</FormError>}
      <Input
        id="gate-email"
        label="Email"
        type="email"
        autoComplete="email"
        autoFocus
        value={email}
        onChange={(e) => setEmail(e.target.value)}
        placeholder="you@example.com"
      />
      <PasswordInput
        id="gate-signup-password"
        autoComplete="new-password"
        value={password}
        onChange={setPassword}
        placeholder="Min. 8 characters"
      />
      <Button type="submit" size="lg" loading={loading} className="mt-1 w-full">
        Create account
      </Button>
    </form>
  );
}

function LoginForm({ onSuccess }: { onSuccess: () => void }) {
  const [usernameOrEmail, setUsernameOrEmail] = useState("");
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    if (!usernameOrEmail || !password) return setError("All fields are required");
    setLoading(true);
    const result = await login({ usernameOrEmail, password });
    setLoading(false);
    if (result.success) onSuccess();
    else setError(result.errorMessage || "Invalid credentials");
  };

  return (
    <form className="flex flex-col gap-4" onSubmit={submit} noValidate>
      {error && <FormError>{error}</FormError>}
      <Input
        id="gate-login-user"
        label="Email or username"
        type="text"
        autoComplete="username"
        autoFocus
        value={usernameOrEmail}
        onChange={(e) => setUsernameOrEmail(e.target.value)}
        placeholder="you@example.com"
      />
      <PasswordInput
        id="gate-login-password"
        autoComplete="current-password"
        value={password}
        onChange={setPassword}
        placeholder="Your password"
      />
      <Button type="submit" size="lg" loading={loading} className="mt-1 w-full">
        Log in
      </Button>
    </form>
  );
}

function PasswordInput({
  id,
  value,
  onChange,
  placeholder,
  autoComplete,
}: {
  id: string;
  value: string;
  onChange: (v: string) => void;
  placeholder: string;
  autoComplete: string;
}) {
  const [show, setShow] = useState(false);
  return (
    <div className="relative">
      <Input
        id={id}
        label="Password"
        type={show ? "text" : "password"}
        autoComplete={autoComplete}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        inputClassName="pr-11"
      />
      <button
        type="button"
        onClick={() => setShow((v) => !v)}
        tabIndex={-1}
        aria-label={show ? "Hide password" : "Show password"}
        className="absolute right-3 bottom-2.5 text-faint hover:text-ink"
      >
        {show ? (
          <EyeOffIcon aria-hidden className="h-4 w-4" />
        ) : (
          <EyeIcon aria-hidden className="h-4 w-4" />
        )}
      </button>
    </div>
  );
}

function FormError({ children }: { children: string }) {
  return (
    <p role="alert" className="border border-danger px-4 py-3 text-sm text-danger">
      {children}
    </p>
  );
}
