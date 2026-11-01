// Google Analytics 4 (gtag.js). Off unless NEXT_PUBLIC_GA_MEASUREMENT_ID is
// set at build time, so local dev stays out of the reports.
//
// Page views come from GA4 Enhanced Measurement (it records App Router
// navigations via history changes), along with scrolls, outbound clicks and
// common file downloads. On top of that, the site sends the events below.
// Most link clicks need no per-component code: one delegated listener
// (see components/analytics.tsx) classifies them by href, and elements
// tagged with trackAttrs() report their event when clicked. Every delegated
// event also carries `link_location` (nav, footer, modal or the enclosing
// section id).

import { SOCIAL_LINKS, WEBAPP_URL } from "./links";

export const GA_MEASUREMENT_ID =
  process.env.NEXT_PUBLIC_GA_MEASUREMENT_ID?.trim() || undefined;

export type AnalyticsItem = {
  item_id: string;
  item_name: string;
  item_category: "subscription" | "credits";
  item_variant?: string;
  price: number;
  quantity: number;
};

type AnalyticsEvents = {
  // Link clicks, classified by href.
  app_download: {
    app_name: string;
    app_version: string;
    platform: "macos" | "windows" | "linux" | "other";
    file_name: string;
  };
  webapp_click: { destination: string; link_text: string };
  social_click: { network: string };
  contact_click: { method: "mailto" | "copy_email" };
  cta_click: { link_url: string; link_text: string };

  // Clicks on elements tagged with trackAttrs().
  video_start: { video_provider: string; video_title: string; video_url?: string };
  share: { method: string; content_type: string; item_id: string };
  copy_command: { app_name: string };
  app_select: { app_name: string };
  press_kit_download: { asset_name: string };

  // Sent from component code.
  generate_lead: { form_name: string };
  sign_up: { method: "email"; signup_source: string };
  login: { method: "email" };
  prompt_submit: {
    model: string;
    logged_in: boolean;
    aspect_ratio: string;
    resolution: string;
    duration_seconds: number;
    sound: boolean;
  };
  video_generation: { model: string; status: "complete" | "failed" | "timeout" };
  billing_cadence_change: { cadence: string };
  view_credit_packs: Record<string, never>;
  begin_checkout: {
    currency: "USD";
    value: number;
    checkout_type: "signup" | "subscribe" | "switch" | "credits";
    items: AnalyticsItem[];
  };
  manage_subscription: Record<string, never>;
  copy_prompt: { media_type: string };
  media_download: { media_type: string; file_extension: string };
};

export type AnalyticsEvent = keyof AnalyticsEvents;

type ParamValue = string | number | boolean | AnalyticsItem[] | undefined;
type EventParams = Record<string, ParamValue>;

declare global {
  interface Window {
    gtag?: (...args: unknown[]) => void;
  }
}

export function trackEvent<E extends AnalyticsEvent>(
  event: E,
  params: AnalyticsEvents[E],
): void {
  send(event, params);
}

/**
 * Data attributes that make an element report `event` when clicked. Works in
 * server components (no handler needed); spread onto the element or Button.
 */
export function trackAttrs<E extends AnalyticsEvent>(
  event: E,
  params: AnalyticsEvents[E],
): Record<`data-track${string}`, string> {
  const attrs: Record<`data-track${string}`, string> = { "data-track": event };
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined) attrs[`data-track-${key}`] = String(value);
  }
  return attrs;
}

/** Delegated click handler: reports tagged elements and classified links. */
export function trackClick(target: EventTarget | null): void {
  if (!(target instanceof Element)) return;

  const tagged = target.closest<HTMLElement>("[data-track]");
  if (tagged?.dataset.track) {
    send(tagged.dataset.track, {
      ...taggedParams(tagged),
      link_location: linkLocation(tagged),
    });
    return;
  }

  const anchor = target.closest<HTMLAnchorElement>("a[href]");
  const link = anchor && classifyLink(anchor);
  if (link) send(link.event, { ...link.params, link_location: linkLocation(anchor) });
}

function send(event: string, params: EventParams): void {
  if (!GA_MEASUREMENT_ID) return;
  window.gtag?.("event", event, params);
}

const WEBAPP_ORIGIN = new URL(WEBAPP_URL).origin;

// Hostname → network, derived from SOCIAL_LINKS.
const SOCIAL_NETWORKS = new Map(
  Object.entries(SOCIAL_LINKS).map(([name, href]) => [
    bareHostname(new URL(href).hostname),
    name.toLowerCase(),
  ]),
);

// github.com/<org>/<repo>/releases/download/<tag>/<file>: the ArtCraft
// installers and every Crafting App download.
const RELEASE_ASSET_PATH = /^\/[^/]+\/([^/]+)\/releases\/download\/([^/]+)\/([^/]+)$/;

type ClassifiedLink = {
  [E in AnalyticsEvent]: { event: E; params: AnalyticsEvents[E] };
}[AnalyticsEvent];

function classifyLink(anchor: HTMLAnchorElement): ClassifiedLink | null {
  let url: URL;
  try {
    url = new URL(anchor.href);
  } catch {
    return null;
  }

  if (url.protocol === "mailto:") {
    return { event: "contact_click", params: { method: "mailto" } };
  }

  const release = url.hostname === "github.com" && RELEASE_ASSET_PATH.exec(url.pathname);
  if (release) {
    const [, repo, tag, file] = release;
    const fileName = decodeURIComponent(file);
    return {
      event: "app_download",
      params: {
        app_name: repo,
        app_version: tag.replace(/^\D*/, ""),
        platform: downloadPlatform(fileName),
        file_name: fileName,
      },
    };
  }

  if (url.origin === WEBAPP_ORIGIN) {
    return {
      event: "webapp_click",
      params: { destination: url.pathname, link_text: linkText(anchor) },
    };
  }

  const network = SOCIAL_NETWORKS.get(bareHostname(url.hostname));
  if (network) return { event: "social_click", params: { network } };

  // Remaining Button links (internal CTAs, other outbound buttons). Plain
  // text links are left to page views and Enhanced Measurement.
  if (anchor.hasAttribute("data-cta")) {
    return {
      event: "cta_click",
      params: {
        link_url: url.origin === window.location.origin ? url.pathname + url.hash : url.href,
        link_text: linkText(anchor),
      },
    };
  }

  return null;
}

function linkLocation(element: Element): string {
  if (element.closest("#site-nav")) return "nav";
  if (element.closest("footer")) return "footer";
  if (element.closest('[role="dialog"]')) return "modal";
  return element.closest("section[id]")?.id || "page";
}

// data-track-<param> attributes surface in the dataset as track<Param>.
function taggedParams(element: HTMLElement): EventParams {
  const params: EventParams = {};
  for (const [key, value] of Object.entries(element.dataset)) {
    if (key.length > 5 && key.startsWith("track")) {
      params[key.charAt(5).toLowerCase() + key.slice(6)] = value;
    }
  }
  return params;
}

function downloadPlatform(fileName: string): AnalyticsEvents["app_download"]["platform"] {
  if (/\.dmg$|macos/i.test(fileName)) return "macos";
  if (/\.(exe|msi)$|windows/i.test(fileName)) return "windows";
  if (/\.(appimage|deb|rpm)$|linux/i.test(fileName)) return "linux";
  return "other";
}

// GA truncates parameter values at 100 characters.
function linkText(anchor: HTMLAnchorElement): string {
  const text = anchor.getAttribute("aria-label") || anchor.textContent || "";
  return text.replace(/\s+/g, " ").trim().slice(0, 100);
}

function bareHostname(hostname: string): string {
  return hostname.replace(/^www\./, "");
}
