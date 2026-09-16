import { AUTH_LABEL_CLASSES, AUTH_INPUT_CLASSES, AUTH_PASSWORD_INPUT_CLASSES } from "./auth-form-styles";
import { EyeIcon, EyeOffIcon, LoaderCircleIcon } from "lucide-react";
import { DynamicIcon } from "@storyteller/icons";
import { Button } from "@storyteller/ui-button";
import { Input } from "@storyteller/ui-input";
import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { UsersApi } from "@storyteller/api";
import {
  getLandingUrl,
  getReferralCode,
  getReferralUsername,
  getReferrer,
} from "@storyteller/common";
import { refreshSession } from "../../lib/session";
import { hasActiveSubscription } from "../../lib/billing";
import { GoogleLoginButton } from "./GoogleLoginButton";

interface SignupFormProps {
  onSuccess: (isNewUser?: boolean) => void;
  signupSource: string;
  className?: string;
  autoFocus?: boolean;
}

export const SignupForm = ({
  onSuccess,
  signupSource,
  className = "",
  autoFocus = false,
}: SignupFormProps) => {
  const navigate = useNavigate();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);

  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSignup = async () => {
    setError(null);

    if (!email || !password) {
      setError("All fields are required");
      return;
    }

    if (password.length < 8) {
      setError("Password must be at least 8 characters");
      return;
    }

    setIsLoading(true);

    const api = new UsersApi();

    // Generate a username from the email
    const emailPrefix = email.split("@")[0].replace(/[^a-zA-Z0-9]/g, "");
    // Ensure we don't exceed max length (7 chars from email + 3 digits = 10 total)
    const truncatedPrefix = emailPrefix.substring(0, 7);
    const randomSuffix = Math.floor(Math.random() * 900) + 100; // 100-999
    const generatedUsername = `${truncatedPrefix}${randomSuffix}`;

    const response = await api.Signup({
      username: generatedUsername,
      email,
      password,
      passwordConfirmation: password,
      signupSource,
      maybeReferralUrl: getReferrer(),
      maybeLandingUrl: getLandingUrl(),
      maybeReferralUsername: getReferralUsername(),
      maybeReferralCode: getReferralCode(),
    });

    setIsLoading(false);

    if (response.success) {
      window.dispatchEvent(new Event("auth-change"));
      onSuccess();
    } else {
      setError(response.errorMessage || "Failed to create account");
    }
  };

  const handleGoogleSuccess = async () => {
    // Refresh the session and check the subscription in parallel; users without
    // an active subscription are pushed to pricing, subscribers go home.
    const [, subscribed] = await Promise.all([
      refreshSession(true),
      hasActiveSubscription(),
    ]);
    navigate(subscribed ? "/" : "/pricing");
  };

  const handleGoogleError = (message: string) => {
    setError(message);
  };

  return (
    <div className={className}>
      <form
        className="space-y-4"
        onSubmit={(e) => {
          e.preventDefault();
          handleSignup();
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
              Email
            </label>
            <Input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="you@example.com"
              autoFocus={autoFocus}
              inputClassName={AUTH_INPUT_CLASSES}
            />
          </div>

          <div className="space-y-2">
            <div className="flex justify-between items-center">
              <label className={AUTH_LABEL_CLASSES}>
                Password
              </label>
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
                "Create account"
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
            mode="signup"
            onSuccess={handleGoogleSuccess}
            onError={handleGoogleError}
          />
        </div>
      </div>
    </div>
  );
};
