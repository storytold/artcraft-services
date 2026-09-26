import { useEffect, useRef, useState, type ReactNode } from "react";
import { Link } from "react-router-dom";
import { HttpApiError, LoginChallengesApi, type LoginChallengeResult, type LoginChallengeReview } from "@storyteller/api";
import { captureLoginBridgeContext, clearLoginBridgeContext } from "./login-bridge-context";

const EXPIRED_MESSAGE = "This login request expired. Start again in ArtCraft.";

interface DesktopLoginApprovalProps {
  loggedIn: boolean;
  authChecked: boolean;
  signIn: ReactNode;
}

/** Shared browser consent UI. Signing into the website never approves a desktop. */
export function DesktopLoginApproval({ loggedIn, authChecked, signIn }: DesktopLoginApprovalProps) {
  const [token] = useState(captureLoginBridgeContext);
  const [review, setReview] = useState<LoginChallengeReview | null>(null);
  const [busy, setBusy] = useState(false);
  const deciding = useRef(false);
  const [message, setMessage] = useState("");
  const [finished, setFinished] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const [now, setNow] = useState(Date.now);

  useEffect(() => {
    if (!token || !loggedIn || !authChecked) return;
    let active = true;
    setReview(null);
    setMessage("");
    new LoginChallengesApi().review(token).then((result) => {
      if (!active) return;
      const completion = completionMessage(result);
      if (completion) {
        setFinished(true);
        setMessage(completion);
        clearLoginBridgeContext();
      } else {
        if (!Number.isFinite(Date.parse(result.expires_at)) ||
            !/^[BCDFGHJKLMNPQRSTVWXZ]{8}$/.test(result.confirmation_code) ||
            !result.username || !result.requesting_ip) throw new Error("Invalid login review");
        setNow(Date.now());
        setReview(result);
      }
    }).catch((error) => {
      if (!active) return;
      setMessage(error instanceof HttpApiError && error.status === 401
        ? "This login request or account session is no longer valid. Start a new login in ArtCraft."
        : "Unable to load this login request. Please try again.");
    });
    return () => { active = false; };
  }, [token, loggedIn, authChecked, attempt]);

  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, []);

  const expired = !!review && now >= Date.parse(review.expires_at);
  useEffect(() => {
    if (expired) clearLoginBridgeContext();
  }, [expired]);

  const decide = async (approve: boolean) => {
    if (!token || !review || !loggedIn || deciding.current || finished ||
        Date.now() >= Date.parse(review.expires_at)) return;
    deciding.current = true;
    setBusy(true);
    setMessage("");
    try {
      const result = await new LoginChallengesApi().decide(token, approve);
      const completion = completionMessage(result);
      if (!completion) throw new Error("Unexpected login state");
      setFinished(true);
      setMessage(completion);
      clearLoginBridgeContext();
    } catch {
      setMessage("Unable to confirm your choice. Please try again.");
    } finally {
      deciding.current = false;
      setBusy(false);
    }
  };

  if (!authChecked) return <Status>Checking your account…</Status>;
  if (!token) return <Status>Open a new login link or scan the QR code from ArtCraft.</Status>;
  if (!loggedIn) return <>{signIn}</>;

  return <main className="flex min-h-svh items-center justify-center bg-[#101014] p-6 text-white">
    <section className="w-full max-w-md border border-white/15 bg-white/5 p-6 sm:p-8">
      <Link to="/" className="text-sm text-white/60">ArtCraft</Link>
      <h1 className="mb-4 mt-6 text-3xl font-semibold">Log in to ArtCraft Desktop?</h1>
      {message && <p role="status" className="mb-5">{message}</p>}
      {expired && !finished && <p role="alert">{EXPIRED_MESSAGE}</p>}
      {!finished && !expired && review && <>
        <p>Authorize your desktop as <strong>{review.username}</strong>.</p>
        <p className="mt-3 break-all text-sm text-white/70">Requesting IP: <code>{review.requesting_ip}</code></p>
        <p className="mt-5">Make sure this code matches the code on your desktop:</p>
        <p className="my-5 text-center font-mono text-2xl tracking-widest sm:text-3xl">{review.confirmation_code.slice(0, 4)}-{review.confirmation_code.slice(4)}</p>
        <div className="mt-6 flex flex-wrap gap-3">
          <button className="flex-1 bg-white px-4 py-3 font-medium text-black disabled:opacity-40" disabled={busy} onClick={() => decide(true)}>Approve desktop login</button>
          <button className="border border-white/30 px-4 py-3 disabled:opacity-40" disabled={busy} onClick={() => decide(false)}>Decline</button>
        </div>
      </>}
      {!review && !message && <p role="status">Loading login request…</p>}
      {!review && message && !finished && <button className="mt-4 border border-white/30 px-4 py-3" onClick={() => setAttempt((value) => value + 1)}>Try again</button>}
    </section>
  </main>;
}

function Status({ children }: { children: ReactNode }) {
  return <main className="flex min-h-svh items-center justify-center bg-[#101014] p-8 text-white" role="status">{children}</main>;
}

function completionMessage(result: LoginChallengeResult): string | null {
  if (!result.success) throw new Error("Unsuccessful login response");
  if (result.status === "pending") return null;
  if (result.status === "approved" || result.status === "redeemed") return "Desktop login approved. Return to ArtCraft to finish signing in.";
  if (result.status === "failed" && result.maybe_failure_type === "user_declined") return "Desktop login declined. No session was created.";
  if (result.status === "failed" && result.maybe_failure_type === "expired") return EXPIRED_MESSAGE;
  throw new Error("Unknown login state");
}
