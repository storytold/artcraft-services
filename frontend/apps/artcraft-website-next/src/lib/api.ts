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

// Email/password login. Stores the signed session so auth survives where
// the cross-site cookie is blocked (Safari ITP), mirroring the shared lib.
export function login(body: {
  usernameOrEmail: string;
  password: string;
}): Promise<ApiResult<{ signedSession?: string }>> {
  return request<{ signed_session?: string }>("/v1/login", {
    method: "POST",
    body: { username_or_email: body.usernameOrEmail, password: body.password },
  }).then((r) => {
    if (!r.success) return r;
    storeSignedSession(r.data.signed_session);
    return { success: true, data: { signedSession: r.data.signed_session } };
  });
}

export function signup(body: {
  username: string;
  email: string;
  password: string;
  signupSource: string;
  maybeReferralUrl?: string;
  maybeLandingUrl?: string;
  maybeReferralUsername?: string;
  maybeReferralCode?: string;
}): Promise<ApiResult<{ signedSession?: string }>> {
  return request<{
    signed_session?: string;
    error_fields?: Record<string, string>;
  }>("/v1/create_account", {
    method: "POST",
    body: {
      username: body.username,
      email_address: body.email,
      password: body.password,
      password_confirmation: body.password,
      signup_source: body.signupSource,
      ...(body.maybeReferralUrl && { maybe_referral_url: body.maybeReferralUrl }),
      ...(body.maybeLandingUrl && { maybe_landing_url: body.maybeLandingUrl }),
      ...(body.maybeReferralUsername && {
        maybe_referral_username: body.maybeReferralUsername,
      }),
      ...(body.maybeReferralCode && { maybe_referral_code: body.maybeReferralCode }),
    },
  }).then((r) => {
    if (!r.success) return r;
    storeSignedSession(r.data.signed_session);
    return { success: true, data: { signedSession: r.data.signed_session } };
  });
}

// Omni-gen video generation. Only the fields the campaign promptbox sets;
// nulls let the server apply model defaults, same as the Vite page.
export function generateVideo(body: {
  model: string;
  prompt: string;
  aspectRatio?: string;
  resolution?: string;
  durationSeconds?: number;
  generateAudio?: boolean;
}): Promise<ApiResult<{ jobToken: string }>> {
  return request<{ inference_job_token?: string }>("/v1/omni_gen/generate/video", {
    method: "POST",
    body: {
      model: body.model,
      prompt: body.prompt,
      idempotency_token: crypto.randomUUID(),
      aspect_ratio: body.aspectRatio ?? null,
      resolution: body.resolution ?? null,
      duration_seconds: body.durationSeconds ?? null,
      generate_audio: body.generateAudio ?? null,
      video_batch_count: 1,
      start_frame_image_media_token: null,
      end_frame_image_media_token: null,
      reference_image_media_tokens: null,
      reference_video_media_tokens: null,
      reference_audio_media_tokens: null,
      reference_character_tokens: null,
    },
  }).then((r) => {
    if (!r.success) return r;
    return r.data.inference_job_token
      ? { success: true, data: { jobToken: r.data.inference_job_token } }
      : { success: false, errorMessage: "Generation failed" };
  });
}

export type JobPoll =
  | { status: "pending" }
  | { status: "complete"; videoUrl: string }
  | { status: "failed"; error: string };

export function getVideoJob(jobToken: string): Promise<JobPoll> {
  return request<{
    state?: {
      status?: {
        status?: string;
        maybe_failure_message?: string;
        maybe_extra_status_description?: string;
      };
      maybe_result?: { media_links?: { cdn_url?: string } };
    };
  }>(`/v1/jobs/job/${jobToken}`).then((r) => {
    if (!r.success) return { status: "pending" };
    const state = r.data.state;
    const status = state?.status?.status?.toLowerCase() ?? "";
    if (status === "complete_success" || status === "complete") {
      const url = state?.maybe_result?.media_links?.cdn_url;
      return url ? { status: "complete", videoUrl: url } : { status: "pending" };
    }
    if (status.includes("fail") || status.includes("error") || status === "dead") {
      return {
        status: "failed",
        error:
          state?.status?.maybe_failure_message ??
          state?.status?.maybe_extra_status_description ??
          "Generation failed",
      };
    }
    return { status: "pending" };
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

// ── Shared media (the /media/[token] share page) ─────────────────────────
// Mirrors MediaFilesApi.GetMediaFileByToken / PromptsApi.GetPromptsByToken
// from the shared lib, trimmed to the fields the share page renders.

export type MediaUser = {
  user_token: string;
  username: string;
  display_name: string;
  email_gravatar_hash: string;
};

export type MediaFile = {
  token: string;
  media_class: string | null;
  maybe_creator_user: MediaUser | null;
  maybe_prompt_token: string | null;
  media_links: {
    cdn_url: string;
    /** Contains a `{WIDTH}` placeholder. */
    thumbnail_template: string | null;
  };
  created_at: string;
};

export type PromptContextImage = {
  media_token: string;
  semantic: string;
  media_links: {
    cdn_url: string;
    maybe_thumbnail_template: string | null;
    /** Set for video references: a still frame and its thumbnail template. */
    maybe_video_previews: {
      still: string;
      still_thumbnail_template: string | null;
    } | null;
  };
};

export type Prompt = {
  token: string;
  maybe_positive_prompt: string | null;
  maybe_generation_provider: string | null;
  maybe_model_type: string | null;
  maybe_context_images: PromptContextImage[] | null;
  maybe_aspect_ratio: string | null;
  maybe_resolution: string | null;
  maybe_duration_seconds: number | null;
  maybe_generate_audio: boolean | null;
};

export function mediaFilePath(token: string): string {
  return `/v1/media_files/file/${encodeURIComponent(token)}`;
}

export function getMediaFile(token: string): Promise<ApiResult<MediaFile>> {
  return request<{ media_file?: MediaFile }>(mediaFilePath(token)).then((r) => {
    if (!r.success) return r;
    return r.data.media_file
      ? { success: true, data: r.data.media_file }
      : { success: false, errorMessage: "Media not found" };
  });
}

export function getPrompt(token: string): Promise<ApiResult<Prompt>> {
  return request<{ prompt?: Prompt }>(
    `/v1/prompts/${encodeURIComponent(token)}`,
  ).then((r) => {
    if (!r.success) return r;
    return r.data.prompt
      ? { success: true, data: r.data.prompt }
      : { success: false, errorMessage: "Prompt not found" };
  });
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

function storeSignedSession(signedSession: string | undefined): void {
  if (!signedSession) return;
  try {
    localStorage.setItem(SESSION_STORAGE_KEY, signedSession);
  } catch {
    // Storage unavailable; the cookie still carries the session.
  }
}

function readSignedSession(): string | null {
  try {
    return localStorage.getItem(SESSION_STORAGE_KEY);
  } catch {
    return null;
  }
}
