// Minimal Storyteller API client, ported from the @storyteller/api lib's
// ApiManager / UsersApi / BillingApi for the pieces the marketing site
// needs (session detection and Stripe billing flows). BROWSER ONLY: every
// call sends cookies (`credentials: include`) plus the signed-session
// header fallback for browsers that block the third-party cookie.
//
// Endpoints, request bodies, and response envelopes match the lib exactly
// so the same backend handlers serve both sites.

export const API_HOST =
  process.env.NEXT_PUBLIC_API_HOST ?? "https://api.storyteller.ai";

// Same key the shared lib uses, so a session stored by the webapp's login
// on this origin (dev) is honored here too.
const SESSION_STORAGE_KEY = "artcraft_signed_session";

export type ApiResult<T> =
  | { success: true; data: T }
  | { success: false; errorMessage: string };

export type SessionUser = {
  user_token: string;
  username: string;
  display_name: string;
  email_address?: string;
};

export type Session = {
  loggedIn: boolean;
  user?: SessionUser;
};

export type ActiveSubscription = {
  namespace: string;
  product_slug: string;
};

export type BillingCadence = "monthly" | "yearly";

export function getSession(): Promise<ApiResult<Session>> {
  return request<{ logged_in: boolean; user?: SessionUser }>("/v1/session").then(
    (r) =>
      r.success
        ? { success: true, data: { loggedIn: r.data.logged_in, user: r.data.user } }
        : r,
  );
}

export function listActiveSubscriptions(): Promise<
  ApiResult<ActiveSubscription[]>
> {
  return request<{ active_subscriptions?: ActiveSubscription[] }>(
    "/v1/billing/active_subscriptions",
  ).then((r) =>
    r.success ? { success: true, data: r.data.active_subscriptions ?? [] } : r,
  );
}

// Not logged in: Stripe Checkout creates the account and the subscription
// together. Referral attribution rides along.
export function userSignupSubscriptionCheckout(body: {
  plan: string;
  cadence: BillingCadence;
  maybeReferralUrl?: string;
  maybeLandingUrl?: string;
  maybeReferralUsername?: string;
  maybeReferralCode?: string;
}): Promise<ApiResult<{ checkoutUrl: string }>> {
  return checkoutRequest("/v1/stripe_artcraft/user_signup_subscription_checkout", {
    plan: body.plan,
    cadence: body.cadence,
    ...(body.maybeReferralUrl && { maybe_referral_url: body.maybeReferralUrl }),
    ...(body.maybeLandingUrl && { maybe_landing_url: body.maybeLandingUrl }),
    ...(body.maybeReferralUsername && {
      maybe_referral_username: body.maybeReferralUsername,
    }),
    ...(body.maybeReferralCode && { maybe_referral_code: body.maybeReferralCode }),
  });
}

// Logged in, no active plan: attach a subscription to the existing account.
export function subscriptionCheckout(body: {
  plan: string;
  cadence: BillingCadence;
}): Promise<ApiResult<{ checkoutUrl: string }>> {
  return checkoutRequest("/v1/stripe_artcraft/checkout/subscription", body);
}

// Logged in with an active plan: Stripe portal flow to change plans.
export function switchPlan(body: {
  plan: string;
  cadence: BillingCadence;
}): Promise<ApiResult<{ portalUrl: string }>> {
  return portalRequest("/v1/stripe_artcraft/portal/switch_plan", body);
}

export function getPortalUrl(): Promise<ApiResult<{ portalUrl: string }>> {
  return portalRequest("/v1/stripe_artcraft/portal/manage_plan", {});
}

export function creditsPackCheckout(
  creditsPack: string,
): Promise<ApiResult<{ checkoutUrl: string }>> {
  return checkoutRequest("/v1/stripe_artcraft/checkout/credits_pack", {
    credits_pack: creditsPack,
  });
}

// Fire-and-forget referral log, once per browser session (mirrors the Vite
// site's main.tsx bootstrap).
export function logWebReferral(maybeReferralUrl?: string): Promise<void> {
  return request("/v1/web_referrals/record", {
    method: "POST",
    body: { maybe_referral_url: maybeReferralUrl ?? null },
  }).then(() => undefined);
}

function checkoutRequest(
  path: string,
  body: unknown,
): Promise<ApiResult<{ checkoutUrl: string }>> {
  return request<{ stripe_checkout_redirect_url?: string }>(path, {
    method: "POST",
    body,
  }).then((r) => {
    if (!r.success) return r;
    return r.data.stripe_checkout_redirect_url
      ? { success: true, data: { checkoutUrl: r.data.stripe_checkout_redirect_url } }
      : { success: false, errorMessage: "Failed to initiate checkout" };
  });
}

function portalRequest(
  path: string,
  body: unknown,
): Promise<ApiResult<{ portalUrl: string }>> {
  return request<{ stripe_portal_url?: string }>(path, {
    method: "POST",
    body,
  }).then((r) => {
    if (!r.success) return r;
    return r.data.stripe_portal_url
      ? { success: true, data: { portalUrl: r.data.stripe_portal_url } }
      : { success: false, errorMessage: "Failed to open the billing portal" };
  });
}

type Envelope = { success: boolean; error_message?: string; message?: string };

async function request<T extends object>(
  path: string,
  init: { method?: "GET" | "POST"; body?: unknown } = {},
): Promise<ApiResult<T>> {
  try {
    const headers: Record<string, string> = {
      Accept: "application/json",
      "Content-Type": "application/json",
    };
    const session = readSignedSession();
    if (session) headers.session = session;

    const response = await fetch(`${API_HOST}${path}`, {
      method: init.method ?? "GET",
      headers,
      credentials: "include",
      body: init.body === undefined ? undefined : JSON.stringify(init.body),
    });

    const payload = (await response.json().catch(() => null)) as
      | (Envelope & T)
      | null;
    if (!response.ok) {
      return {
        success: false,
        errorMessage:
          payload?.message ?? payload?.error_message ?? `Request failed (${response.status})`,
      };
    }
    if (!payload?.success) {
      return {
        success: false,
        errorMessage: payload?.error_message ?? "Request failed",
      };
    }
    return { success: true, data: payload };
  } catch (err) {
    return {
      success: false,
      errorMessage: err instanceof Error ? err.message : "Network error",
    };
  }
}

function readSignedSession(): string | null {
  try {
    return localStorage.getItem(SESSION_STORAGE_KEY);
  } catch {
    return null;
  }
}
