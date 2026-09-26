import { AUTH_LABEL_CLASSES, AUTH_INPUT_CLASSES, AUTH_PASSWORD_INPUT_CLASSES, AUTH_FORM_PADDING } from "../../components/auth/auth-form-styles";
import {
  ArrowLeftIcon,
  CircleCheckIcon,
  EyeIcon,
  EyeOffIcon,
  LoaderCircleIcon,
} from "lucide-react";
import { DynamicIcon } from "@storyteller/icons";
import { Button } from "@storyteller/ui-button";
import { Input } from "@storyteller/ui-input";
import { useState } from "react";
import { Link, useNavigate, useSearchParams } from "react-router-dom";
import { PasswordResetApi, BillingApi } from "@storyteller/api";

import { AuthHeader } from "../../components/auth/auth-layout";
import { AuthPageFrame } from "../../components/auth/auth-page-frame";
import Seo from "../../components/seo";
import { authContinuationUrl, LOGIN_BRIDGE_PATH, safeAuthReturnPath } from "../../lib/login-bridge-context";
import { refreshSession } from "../../lib/session";

const VerifyReset = () => {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();

  // Pre-fill token from URL query param if present
  const tokenFromUrl = searchParams.get("token") || "";

  const [verificationCode, setVerificationCode] = useState(tokenFromUrl);
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [showNewPassword, setShowNewPassword] = useState(false);
  const [showConfirmPassword, setShowConfirmPassword] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [success, setSuccess] = useState(false);
  const [redirectTo, setRedirectTo] = useState("/");
  const [redirectLabel, setRedirectLabel] = useState("Back to Homepage");

  const handleRedeemReset = async () => {
    setError(null);
    setFieldErrors({});

    // Client-side validation
    const errors: Record<string, string> = {};
    if (!verificationCode.trim()) {
      errors.verificationCode = "Verification code is required.";
    }
    if (!newPassword) {
      errors.newPassword = "New password is required.";
    } else if (newPassword.length < 8) {
      errors.newPassword = "Password must be at least 8 characters.";
    }
    if (!confirmPassword) {
      errors.confirmPassword = "Please confirm your new password.";
    } else if (newPassword !== confirmPassword) {
      errors.confirmPassword = "Passwords do not match.";
    }

    if (Object.keys(errors).length > 0) {
      setFieldErrors(errors);
      return;
    }

    setIsLoading(true);

    const api = new PasswordResetApi();
    const response = await api.RedeemPasswordReset({
      resetToken: verificationCode.trim(),
      newPassword: newPassword,
      newPasswordValidation: confirmPassword,
    });

    setIsLoading(false);

    if (response.success) {
      setSuccess(true);
      window.dispatchEvent(new Event("auth-change"));

      if (safeAuthReturnPath(searchParams.get("from")) === LOGIN_BRIDGE_PATH) {
        await refreshSession(true);
        setRedirectTo(LOGIN_BRIDGE_PATH);
        setRedirectLabel("Review desktop login");
        return;
      }

      // Check if user has an active subscription to decide redirect
      try {
        const billingApi = new BillingApi();
        const billingResponse = await billingApi.ListActiveSubscriptions();
        if (
          billingResponse.success &&
          billingResponse.data &&
          billingResponse.data.active_subscriptions.length > 0
        ) {
          setRedirectTo("/");
          setRedirectLabel("Back to Homepage");
        } else {
          setRedirectTo("/pricing");
          setRedirectLabel("Continue");
        }
      } catch {
        // Default to homepage if billing check fails
        setRedirectTo("/");
        setRedirectLabel("Back to Homepage");
      }
    } else {
      setError(
        response.errorMessage ||
          "Failed to reset password. Please check your code and try again.",
      );
    }
  };

  return (
    <AuthPageFrame>
      <Seo
        title="Verify Password Reset - ArtCraft"
        description="Enter your verification code and new password."
      />

      <main className={AUTH_FORM_PADDING}>
        <div className="w-full">
          {!success ? (
            <>
              <AuthHeader title={<>Set a new <span className="font-serif-italic">password.</span></>} subtitle="Enter your reset code and choose a new password." />

              <form
                className="space-y-4"
                onSubmit={(e) => {
                  e.preventDefault();
                  handleRedeemReset();
                }}
              >
                {error && (
                  <div className="bg-red-500/10 border border-red-500/20 text-red-500 px-4 py-3 text-sm text-center">
                    {error}
                  </div>
                )}

                {/* Verification Code */}
                <div className="space-y-2">
                  <label className={AUTH_LABEL_CLASSES}>
                    Verification Code
                  </label>
                  <Input
                    id="verification-code"
                    type="text"
                    value={verificationCode}
                    onChange={(e) => setVerificationCode(e.target.value)}
                    placeholder="Enter verification code"
                    isError={!!fieldErrors.verificationCode}
                    inputClassName={AUTH_INPUT_CLASSES}
                  />
                  {fieldErrors.verificationCode && (
                    <p className="text-red-400 text-xs">
                      {fieldErrors.verificationCode}
                    </p>
                  )}
                </div>

                {/* New Password */}
                <div className="space-y-2">
                  <label className={AUTH_LABEL_CLASSES}>
                    New Password
                  </label>
                  <div className="relative">
                    <Input
                      id="new-password"
                      type={showNewPassword ? "text" : "password"}
                      value={newPassword}
                      onChange={(e) => setNewPassword(e.target.value)}
                      placeholder="Enter new password"
                      isError={!!fieldErrors.newPassword}
                      inputClassName={AUTH_PASSWORD_INPUT_CLASSES}
                    />
                    <button
                      type="button"
                      onClick={() => setShowNewPassword(!showNewPassword)}
                      className="absolute right-4 top-1/2 -translate-y-1/2 text-white/30 hover:text-white/60 transition-colors"
                    >
                      <DynamicIcon
                        icon={showNewPassword ? EyeOffIcon : EyeIcon}
                      />
                    </button>
                  </div>
                  {fieldErrors.newPassword && (
                    <p className="text-red-400 text-xs">
                      {fieldErrors.newPassword}
                    </p>
                  )}
                </div>

                {/* Verify New Password */}
                <div className="space-y-2">
                  <label className={AUTH_LABEL_CLASSES}>
                    Verify New Password
                  </label>
                  <div className="relative">
                    <Input
                      id="confirm-password"
                      type={showConfirmPassword ? "text" : "password"}
                      value={confirmPassword}
                      onChange={(e) => setConfirmPassword(e.target.value)}
                      placeholder="Enter new password again"
                      isError={!!fieldErrors.confirmPassword}
                      inputClassName={AUTH_PASSWORD_INPUT_CLASSES}
                    />
                    <button
                      type="button"
                      onClick={() =>
                        setShowConfirmPassword(!showConfirmPassword)
                      }
                      className="absolute right-4 top-1/2 -translate-y-1/2 text-white/30 hover:text-white/60 transition-colors"
                    >
                      <DynamicIcon
                        icon={showConfirmPassword ? EyeOffIcon : EyeIcon}
                      />
                    </button>
                  </div>
                  {fieldErrors.confirmPassword && (
                    <p className="text-red-400 text-xs">
                      {fieldErrors.confirmPassword}
                    </p>
                  )}
                </div>

                <div className="pt-2">
                  <Button
                    id="change-password-btn"
                    className="w-full justify-center h-10"
                    type="submit"
                    disabled={isLoading}
                  >
                    {isLoading ? (
                      <LoaderCircleIcon className="animate-spin" />
                    ) : (
                      "Change Password"
                    )}
                  </Button>
                </div>
              </form>

              <div className="mt-8 text-center text-sm">
                <Link
                  to={authContinuationUrl("/forgot-password", searchParams.get("from"))}
                  className="text-white/60 hover:text-white transition-colors flex items-center justify-center gap-2"
                >
                  <ArrowLeftIcon /> Request a new code
                </Link>
              </div>
            </>
          ) : (
            <>
              <div className="text-center py-8">
                <div className="w-16 h-16 border border-white/15 bg-green-500/10 flex items-center justify-center mx-auto mb-4 text-green-500">
                  <CircleCheckIcon className="text-2xl" />
                </div>
                <h3 className="text-xl font-medium mb-2">
                  Password Reset Successfully
                </h3>
                <p className="text-white/60 text-sm mb-8">
                  Your password has been changed and you've been logged in
                  successfully.
                </p>
                <Button
                  id="back-to-homepage-btn"
                  className="w-full justify-center h-10"
                  onClick={() => navigate(redirectTo)}
                >
                  {redirectLabel}
                </Button>
              </div>
            </>
          )}
        </div>
      </main>
    </AuthPageFrame>
  );
};

export default VerifyReset;
