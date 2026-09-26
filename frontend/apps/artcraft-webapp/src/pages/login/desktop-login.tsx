import { Navigate } from "react-router-dom";
import { DesktopLoginApproval, LOGIN_BRIDGE_PATH } from "@frontend/login";
import { useSession } from "../../lib/session";

export function DesktopLogin() {
  const { loggedIn, authChecked, user } = useSession();
  return <DesktopLoginApproval
    key={user?.user_token ?? "signed-out"}
    loggedIn={loggedIn}
    authChecked={authChecked}
    signIn={<Navigate to={`/login?from=${encodeURIComponent(LOGIN_BRIDGE_PATH)}`} replace />}
  />;
}
