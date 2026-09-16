import { AUTH_FORM_PADDING } from "./auth-form-styles";
import { ReactNode } from "react";
import { Outlet, Navigate, Link } from "react-router-dom";
import { LoaderCircleIcon } from "lucide-react";
import { AuthShowcase } from "./auth-showcase";
import { AuthPageFrame } from "./auth-page-frame";
import { useMediaQuery } from "../ui/use-media-query";
import { useSession } from "../../lib/session";

/**
 * Persistent shell for the auth pages. Rendered as a layout route so the
 * showcase pane (and its video) stays mounted while the form pane swaps
 * between login and signup via <Outlet> — toggling between the two no longer
 * reloads the showcase video from the start. Each page renders its own header
 * (AuthHeader) and footer (AuthFooter) into the outlet.
 */
export const AuthLayout = () => {
  // Only mount the showcase on wide screens (matches the `lg` breakpoint) so
  // mobile never downloads the demo videos.
  const showShowcase = useMediaQuery("(min-width: 1024px)");
  const { loggedIn, authChecked } = useSession();

  // Already signed in? Never show the auth form — go straight home.
  if (loggedIn) {
    return <Navigate to="/" replace />;
  }

  return (
    <AuthPageFrame wide>
        {/* Form pane — only the inner content swaps per route (via Outlet), so
            this pane and the showcase beside it stay mounted across the
            login/signup toggle. */}
        <div className="relative flex w-full flex-col lg:w-1/2">
          <div className={`flex flex-1 flex-col justify-center ${AUTH_FORM_PADDING}`}>
            <div className="w-full">
              {authChecked ? (
                <Outlet />
              ) : (
                <div className="flex justify-center py-12">
                  <LoaderCircleIcon
                    
                    className="animate-spin text-2xl text-white/40" />
                </div>
              )}
            </div>
          </div>

        </div>

        {/* Showcase pane (desktop only) */}
        {showShowcase && (
          <div className="relative border-l border-white/15 lg:w-1/2">
            <AuthShowcase />
          </div>
        )}
    </AuthPageFrame>
  );
};

interface AuthHeaderProps {
  title: ReactNode;
  subtitle: string;
}

export const AuthHeader = ({ title, subtitle }: AuthHeaderProps) => (
  <div className="mb-8 text-left">
    <Link
      to="/"
      aria-label="ArtCraft home"
      className="mb-8 inline-flex rounded-sm transition-opacity hover:opacity-80 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-white/60"
    >
      <img src="/artcraft-icon.svg" alt="ArtCraft" className="h-8 w-8" />
    </Link>
    <h1 className="mb-3 text-balance font-display text-3xl leading-[1.05] tracking-tight sm:text-4xl">
      {title}
    </h1>
    <p className="text-sm leading-relaxed text-white/60">
      {subtitle}
    </p>
  </div>
);

export const AuthFooter = ({ children }: { children: ReactNode }) => (
  <div className="mt-8 text-center text-sm text-white/60">{children}</div>
);
