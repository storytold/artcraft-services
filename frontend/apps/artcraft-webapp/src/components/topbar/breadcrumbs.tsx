import { useLocation, Link } from "react-router-dom";
import { Fragment } from "react";
import { ChevronRightIcon } from "lucide-react";

type Crumb = { label: string; href?: string };

const ROUTE_CRUMBS: Record<string, Crumb[]> = {
  "/": [{ label: "Home" }],
  "/create-image": [{ label: "Create", href: "/" }, { label: "Image" }],
  "/create-video": [{ label: "Create", href: "/" }, { label: "Video" }],
  "/create-audio": [{ label: "Create", href: "/" }, { label: "Audio" }],
  "/create-object": [{ label: "Create", href: "/" }, { label: "3D Object" }],
  "/create-world": [{ label: "Create", href: "/" }, { label: "3D World" }],
  "/background-change": [
    { label: "Studio", href: "/" },
    { label: "Background Change" },
  ],
  "/edit-image": [{ label: "Studio", href: "/" }, { label: "Edit Image" }],
  "/edit-3d": [{ label: "Studio", href: "/" }, { label: "Edit 3D" }],
  "/video-editor": [{ label: "Studio", href: "/" }, { label: "Edit Video" }],
  "/moodboard": [{ label: "Studio", href: "/" }, { label: "Moodboard" }],
  "/frame-extractor": [
    { label: "Studio", href: "/" },
    { label: "Frame Extractor" },
  ],
  "/referrals": [{ label: "Referrals" }],
  "/media": [{ label: "Library", href: "/library" }, { label: "Media" }],
  "/pricing": [{ label: "Pricing" }],
  "/support": [{ label: "Support" }],
  "/login": [{ label: "Login" }],
  "/signup": [{ label: "Sign up" }],
  "/forgot-password": [{ label: "Forgot password" }],
  "/welcome": [{ label: "Welcome" }],
  "/onboarding": [{ label: "Onboarding" }],
  "/checkout/success": [
    { label: "Checkout", href: "/pricing" },
    { label: "Success" },
  ],
  "/checkout/cancel": [
    { label: "Checkout", href: "/pricing" },
    { label: "Cancelled" },
  ],
};

function resolveCrumbs(pathname: string): Crumb[] {
  // Library has sub-tabs (Unsorted / Folders / Tags) that live under /library/*
  // rather than being separate top-level pages.
  if (pathname === "/library" || pathname.startsWith("/library/")) {
    const onFolders =
      pathname === "/library/folders" ||
      pathname.startsWith("/library/folder_");
    const onFolderless = pathname === "/library/folderless";
    const onTags =
      pathname === "/library/tags" || pathname.startsWith("/library/tag_");
    return [
      { label: "Library", href: "/library" },
      {
        label: onFolders
          ? "Folders"
          : onFolderless
            ? "Unfoldered"
            : onTags
              ? "Tags"
              : "All Assets",
      },
    ];
  }
  if (ROUTE_CRUMBS[pathname]) return ROUTE_CRUMBS[pathname];
  const matchedKey = Object.keys(ROUTE_CRUMBS).find(
    (k) => k !== "/" && pathname.startsWith(k + "/"),
  );
  return matchedKey ? ROUTE_CRUMBS[matchedKey] : [];
}

export function Breadcrumbs() {
  const { pathname } = useLocation();
  const crumbs = resolveCrumbs(pathname);
  if (crumbs.length === 0) return null;

  return (
    <nav aria-label="Breadcrumb" className="hidden md:flex items-center gap-1.5 hud-label text-base-fg/70 min-w-0">
      {crumbs.map((crumb, i) => {
        const isLast = i === crumbs.length - 1;
        return (
          <Fragment key={`${crumb.label}-${i}`}>
            {i > 0 && (
              <ChevronRightIcon
                aria-hidden="true"
                className="h-3 w-3 shrink-0 text-base-fg/50"
              />
            )}
            {crumb.href && !isLast ? (
              <Link
                to={crumb.href}
                className="truncate px-1.5 py-1 hover:bg-ui-ink hover:text-ui-panel transition-colors"
              >
                {crumb.label}
              </Link>
            ) : (
              <span
                className={isLast ? "bg-ui-ink px-1.5 py-1 text-ui-panel truncate" : "truncate"}
                aria-current={isLast ? "page" : undefined}
              >
                {crumb.label}
              </span>
            )}
          </Fragment>
        );
      })}
    </nav>
  );
}
