import { AUTH_LABEL_CLASSES, AUTH_INPUT_CLASSES, AUTH_PASSWORD_INPUT_CLASSES } from "../../components/auth/auth-form-styles";
import { EyeIcon, EyeOffIcon, LoaderCircleIcon } from "lucide-react";
import { DynamicIcon } from "@storyteller/icons";
import { Button } from "@storyteller/ui-button";
import { Input } from "@storyteller/ui-input";
import { useState } from "react";
import { Link, useNavigate, useSearchParams } from "react-router-dom";
import { UsersApi } from "@storyteller/api";
import {
  AuthHeader,
  AuthFooter,
  GoogleLoginButton,
} from "../../components/auth";
import Seo from "../../components/seo";
import { refreshSession } from "../../lib/session";
import { hasActiveSubscription } from "../../lib/billing";
import { LOGIN_BRIDGE_PATH, safeAuthReturnPath } from "../../lib/login-bridge-context";

const Login = () => {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const fromParam = searchParams.get("from");
  const redirectTo = safeAuthReturnPath(fromParam);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);

  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleLogin = async () => {
    setError(null);
    setIsLoading(true);

    const api = new UsersApi();
    const response = await api.Login({
      usernameOrEmail: email,
      password: password,
    });

    setIsLoading(false);

    if (response.success) {
      // Wait for the session store to actually reflect the new cookie before
      // navigating — otherwise RequireAuth on the destination sees loggedIn=false
      // and bounces straight back to /login?from=…
      await refreshSession(true);
      navigate(redirectTo);
    } else {
      setError(response.errorMessage || "Invalid credentials");
    }
  };

  const handleGoogleSuccess = async () => {
    if (redirectTo === LOGIN_BRIDGE_PATH) {
      await refreshSession(true);
      navigate(redirectTo);
      return;
    }
    // Refresh the session (so the app sees the new cookie) and check the
    // subscription in parallel; users without one are pushed to pricing.
    const [, subscribed] = await Promise.all([
      refreshSession(true),
      hasActiveSubscription(),
    ]);
    navigate(subscribed ? redirectTo : "/pricing");
  };

  const handleGoogleError = (message: string) => {
    setError(message);
  };

  return (
    <>
      <Seo
        title="Login - ArtCraft"
        description="Login to your ArtCraft account."
      />
      <AuthHeader title={<>Welcome <span className="font-serif-italic">back.</span></>} subtitle="Log in to your creative workspace." />

      <form
        className="space-y-4"
        onSubmit={(e) => {
          e.preventDefault();
          handleLogin();
        }}
      >
        {error && (
          <div className="bg-red-500/10 border border-red-500/20 text-red-500 px-4 py-3 text-sm text-center">
            {error}
          </div>
        )}
        <div
          className="space-y-4"
        >
          <div className="space-y-2">
            <label className={AUTH_LABEL_CLASSES}>
              Email or Username
            </label>
            <Input
              type="text"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="you@example.com or username"
              inputClassName={AUTH_INPUT_CLASSES}
            />
          </div>
          <div className="space-y-2">
            <div className="flex justify-between items-center">
              <label className={AUTH_LABEL_CLASSES}>
                Password
              </label>
              <Link
                to="/forgot-password"
                className="text-xs text-primary hover:text-primary-400 transition-colors"
              >
                Forgot password?
              </Link>
            </div>
            <div className="relative">
              <Input
                type={showPassword ? "text" : "password"}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="Min. 8 characters"
                inputClassName={AUTH_PASSWORD_INPUT_CLASSES}
              />
              <button
                type="button"
                onClick={() => setShowPassword(!showPassword)}
                className="absolute right-4 top-1/2 -translate-y-1/2 text-white/30 hover:text-white/60 transition-colors"
                tabIndex={-1}
              >
                <DynamicIcon icon={showPassword ? EyeOffIcon : EyeIcon} />
              </button>
            </div>
          </div>

          <div className="pt-2">
            <Button
              className="w-full justify-center h-10"
              type="submit"
              disabled={isLoading}
            >
              {isLoading ? (
                <LoaderCircleIcon className="animate-spin" />
              ) : (
                "Log in"
              )}
            </Button>
          </div>
        </div>
      </form>
      <div>
        <div className="my-6 flex items-center gap-4 before:h-px before:flex-1 before:bg-white/15 after:h-px after:flex-1 after:bg-white/15">
          <span className="font-mono text-[11px] font-semibold uppercase tracking-[0.12em] text-white/40">
            or
          </span>
        </div>

        <div>
          <GoogleLoginButton
            mode="login"
            onSuccess={handleGoogleSuccess}
            onError={handleGoogleError}
          />
        </div>

        <div>
          <AuthFooter>
            Don't have an account?{" "}
            <Link
              to="/signup"
              className="font-semibold text-primary transition-colors hover:text-primary-400"
            >
              Sign up
            </Link>
          </AuthFooter>
        </div>
      </div>
    </>
  );
};

export default Login;
