import { Link, useNavigate, useSearchParams } from "react-router-dom";
import { AuthHeader, AuthFooter, SignupForm } from "../../components/auth";
import Seo from "../../components/seo";
import { authContinuationUrl, LOGIN_BRIDGE_PATH, safeAuthReturnPath } from "../../lib/login-bridge-context";
import { refreshSession } from "../../lib/session";

const Signup = () => {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const from = searchParams.get("from");
  const isDesktopLogin = safeAuthReturnPath(from) === LOGIN_BRIDGE_PATH;

  return (
    <>
      <Seo
        title="Sign Up - ArtCraft"
        description="Create your ArtCraft account."
      />
      <AuthHeader title={<>Make it <span className="font-serif-italic">yours.</span></>} subtitle="Create your account and start crafting." />
      <SignupForm
        onSuccess={async () => {
          if (isDesktopLogin) await refreshSession(true);
          navigate(isDesktopLogin ? LOGIN_BRIDGE_PATH : "/welcome");
        }}
        continueAfterGoogleLogin={isDesktopLogin}
        signupSource="artcraft"
      />

      <div>
        <AuthFooter>
          Already have an account?{" "}
          <Link
            to={authContinuationUrl("/login", from)}
            className="font-semibold text-primary transition-colors hover:text-primary-400"
          >
            Log in
          </Link>
        </AuthFooter>
      </div>
    </>
  );
};

export default Signup;
