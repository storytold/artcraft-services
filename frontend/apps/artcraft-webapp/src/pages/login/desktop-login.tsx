import { useEffect, useState } from "react";
import { Link, Navigate } from "react-router-dom";
import { HttpApiError, LoginChallengesApi, type LoginChallengeReview } from "@storyteller/api";
import { useSession } from "../../lib/session";
import { captureLoginBridgeContext, clearLoginBridgeContext, LOGIN_BRIDGE_PATH } from "../../lib/login-bridge-context";

export function DesktopLogin() {
  const { loggedIn, authChecked } = useSession();
  const [token] = useState(captureLoginBridgeContext);
  const [review, setReview] = useState<LoginChallengeReview | null>(null);
  const [matched, setMatched] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [finished, setFinished] = useState(false);
  const [now, setNow] = useState(Date.now);

  useEffect(() => {
    if (!token || !loggedIn) return;
    let active = true;
    new LoginChallengesApi().review(token).then((result) => {
      if (active) {
        setReview(result);
        if (result.status !== "pending") {
          setFinished(true);
          setMessage(result.maybe_failure_type === "expired" ? "This login request expired. Start again in ArtCraft." : "This login request has already been completed.");
          clearLoginBridgeContext();
        }
      }
    }).catch((error) => {
      if (!active) return;
      setMessage(error instanceof HttpApiError && error.status === 401
        ? "This login request or account session is no longer valid. Start a new login in ArtCraft."
        : "Unable to load this login request. Return to ArtCraft and try again.");
    });
    return () => { active = false; };
  }, [token, loggedIn]);

  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, []);

  const expired = review ? now >= Date.parse(review.expires_at) : false;
  const decide = async (approve: boolean) => {
    if (!token || busy || expired || (approve && !matched)) return;
    setBusy(true);
    setMessage("");
    try {
      const result = await new LoginChallengesApi().decide(token, approve);
      const approved = result.status === "approved" || result.status === "redeemed";
      const declined = result.status === "failed" && result.maybe_failure_type === "user_declined";
      const timedOut = result.status === "failed" && result.maybe_failure_type === "expired";
      if (!result.success || (!approved && !declined && !timedOut)) throw new Error("Unexpected login state");
      setFinished(true);
      setMessage(approved ? "Desktop login approved. Return to ArtCraft to finish signing in." : declined ? "Desktop login declined. No session was created." : "This login request expired. Start again in ArtCraft.");
      clearLoginBridgeContext();
    } catch {
      setMessage("Unable to confirm your choice. Please try again.");
    } finally {
      setBusy(false);
    }
  };

  if (!authChecked) return <main className="p-8" role="status">Checking your account…</main>;
  if (!token) return <main className="p-8">Open a new login link or scan the QR code from ArtCraft.</main>;
  if (!loggedIn) return <Navigate to={`/login?from=${encodeURIComponent(LOGIN_BRIDGE_PATH)}`} replace />;

  return <main className="flex min-h-svh items-center justify-center bg-ui-background p-6 text-white">
    <section className="w-full max-w-md rounded-2xl border border-white/15 bg-white/5 p-8">
      <Link to="/" className="text-sm text-white/60">ArtCraft</Link>
      <h1 className="mb-4 mt-6 text-3xl font-semibold">Log in to ArtCraft Desktop?</h1>
      {message && <p role="status" className="mb-5">{message}</p>}
      {expired && !finished && <p role="alert">This request expired. Start a new login in ArtCraft.</p>}
      {!finished && !expired && review && <>
        <p>Authorize your desktop as <strong>{review.username}</strong>.</p>
        <p className="mt-3 text-sm text-white/70">Requesting IP: <code>{review.requesting_ip}</code></p>
        <p className="mt-5">Make sure this code matches the code on your desktop:</p>
        <p className="my-5 text-center font-mono text-3xl tracking-widest">{review.confirmation_code.slice(0, 4)}-{review.confirmation_code.slice(4)}</p>
        <label className="flex items-start gap-3 text-sm">
          <input type="checkbox" checked={matched} onChange={(event) => setMatched(event.target.checked)} />
          <span>I started this request on my desktop and the codes match.</span>
        </label>
        <div className="mt-6 flex gap-3">
          <button className="flex-1 rounded-lg bg-white px-4 py-3 font-medium text-black disabled:opacity-40" disabled={!matched || busy} onClick={() => decide(true)}>Approve desktop login</button>
          <button className="rounded-lg border border-white/30 px-4 py-3" disabled={busy} onClick={() => decide(false)}>Decline</button>
        </div>
      </>}
      {!review && !message && <p role="status">Loading login request…</p>}
    </section>
  </main>;
}
