import { useState } from "react";
import { DesktopLoginApproval } from "@frontend/login";
import { AuthGateModal } from "../../components/auth/auth-gate-modal";
import { refreshSession, useSession } from "../../lib/session";

export function DesktopLogin() {
  const { loggedIn, authChecked, user } = useSession();
  return <DesktopLoginApproval
    key={user?.user_token ?? "signed-out"}
    loggedIn={loggedIn}
    authChecked={authChecked}
    signIn={<DesktopSignIn />}
  />;
}

// Keep sign-in on this origin: sessionStorage and mobile session headers are
// origin-scoped. Redirecting to the webapp would lose the pending approval.
function DesktopSignIn() {
  const [open, setOpen] = useState(true);
  return <main className="flex min-h-svh items-center justify-center bg-[#101014] p-6 text-white">
    <section className="w-full max-w-md border border-white/15 p-8">
      <h1 className="mb-4 text-3xl font-semibold">Sign in to continue</h1>
      <p>Sign in to your ArtCraft account, then review the desktop login request. Signing in does not approve the request.</p>
      <button className="mt-6 bg-white px-4 py-3 font-medium text-black" onClick={() => setOpen(true)}>Sign in</button>
    </section>
    <AuthGateModal
      isOpen={open}
      onClose={() => setOpen(false)}
      initialMode="login"
      onAuthed={() => { void refreshSession(true); }}
      signupSource="artcraft_desktop_login"
      headline="Sign in to ArtCraft"
      subtitle="Next, you’ll review and confirm the desktop login request."
    />
  </main>;
}
