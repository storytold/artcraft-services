// Landing-context tracking, ported from @storyteller/common. Persists the
// referral username (?u=), referral code (?r=), landing URL, and referrer
// to apex-domain cookies (shared across getartcraft.com subdomains) plus
// localStorage as a fallback, so signup attribution survives the hop from
// the marketing site to app.getartcraft.com. BROWSER ONLY.
//
// First visit wins for landing URL / referrer; latest visit wins for
// referral attribution so a fresh share link gets credit.

const REFERRAL_USERNAME_KEY = "referral_username";
const REFERRAL_CODE_KEY = "referral_code";
const LANDING_URL_KEY = "referral_landing_url";
const REFERRER_KEY = "referral_referrer";

const URL_MAX_LENGTH = 1024;
const REFERRAL_CODE_MAX_LENGTH = 30;
const COOKIE_MAX_AGE_DAYS = 90;
const APEX_DOMAINS = ["getartcraft.com"];

export function captureLandingContext(): void {
  try {
    const params = new URLSearchParams(window.location.search);

    const referralUsername = sanitizeReferralUsername(params.get("u") ?? "");
    if (referralUsername) persist(REFERRAL_USERNAME_KEY, referralUsername);

    const referralCode = sanitizeReferralCode(params.get("r") ?? "");
    if (referralCode) persist(REFERRAL_CODE_KEY, referralCode);

    const landingUrl = sanitizeUrl(window.location.href);
    if (landingUrl && !getLandingUrl()) persist(LANDING_URL_KEY, landingUrl);

    const referrer = sanitizeUrl(document.referrer);
    if (referrer && !getReferrer()) persist(REFERRER_KEY, referrer);
  } catch {
    // Storage or URL APIs unavailable; attribution is best-effort.
  }
}

export function getReferralUsername(): string | undefined {
  const stored = read(REFERRAL_USERNAME_KEY);
  return stored ? sanitizeReferralUsername(stored) || undefined : undefined;
}

export function getReferralCode(): string | undefined {
  const stored = read(REFERRAL_CODE_KEY);
  return stored ? sanitizeReferralCode(stored) || undefined : undefined;
}

export function getLandingUrl(): string | undefined {
  return read(LANDING_URL_KEY);
}

export function getReferrer(): string | undefined {
  return read(REFERRER_KEY);
}

function sanitizeReferralUsername(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_]/g, "")
    .slice(0, 20);
}

function sanitizeReferralCode(value: string): string {
  return value
    .trim()
    .replace(/[^A-Za-z0-9._-]/g, "")
    .slice(0, REFERRAL_CODE_MAX_LENGTH);
}

function sanitizeUrl(value: string | null | undefined): string | undefined {
  const trimmed = value?.trim();
  return trimmed ? trimmed.slice(0, URL_MAX_LENGTH) : undefined;
}

function persist(key: string, value: string): void {
  const maxAge = COOKIE_MAX_AGE_DAYS * 24 * 60 * 60;
  const secure = window.location.protocol === "https:" ? "; Secure" : "";
  const hostname = window.location.hostname;
  const apex = APEX_DOMAINS.find(
    (d) => hostname === d || hostname.endsWith(`.${d}`),
  );
  const domain = apex ? `; Domain=.${apex}` : "";
  document.cookie = `${key}=${encodeURIComponent(value)}; Max-Age=${maxAge}; Path=/; SameSite=Lax${secure}${domain}`;
  try {
    localStorage.setItem(key, value);
  } catch {
    // Private mode; the cookie still covers us.
  }
}

function read(key: string): string | undefined {
  const prefix = `${key}=`;
  const cookie = document.cookie
    .split("; ")
    .find((entry) => entry.startsWith(prefix));
  if (cookie) return decodeURIComponent(cookie.slice(prefix.length)) || undefined;
  try {
    return localStorage.getItem(key) ?? undefined;
  } catch {
    return undefined;
  }
}
