"use client";

import { useState, type FormEvent } from "react";
import { CheckIcon } from "lucide-react";
import { Button, Input, Textarea } from "@/components/ui";
import { trackEvent } from "@/lib/analytics";

// Submissions go to a Google Form whose responses land in a Google Sheet.
// The form URL and field IDs are public by nature (they ship in the bundle).
const FORM_RESPONSE_URL =
  "https://docs.google.com/forms/d/e/1FAIpQLSfjRVkOzfBlR0uefebHWSaoEm7R0DRLhUKqwHQlNvBE8bmd-w/formResponse";

// Reuses the form's original short-answer questions (renaming a question in
// Google Forms keeps its ID): appName was "Name", repo was "Company" and
// description was "Type Detail". "Type" is left blank, which marks new rows.
const FORM_FIELD_IDS = {
  appName: "entry.666666785",
  repo: "entry.121325393",
  email: "entry.467034813",
  description: "entry.1914395461",
  page: "entry.709868934",
  referrer: "entry.617211358",
} as const;

const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

type FieldErrors = Partial<Record<"appName" | "email" | "repo" | "description", string>>;

export default function BetaForm() {
  const [appName, setAppName] = useState("");
  const [email, setEmail] = useState("");
  const [repo, setRepo] = useState("");
  const [description, setDescription] = useState("");
  // Honeypot: hidden from humans; bots that fill it are silently dropped.
  const [website, setWebsite] = useState("");

  const [errors, setErrors] = useState<FieldErrors>({});
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [submitted, setSubmitted] = useState(false);

  const validate = (): FieldErrors => {
    const next: FieldErrors = {};
    if (!appName.trim()) next.appName = "Please enter your app's name.";
    if (!email.trim()) next.email = "Please enter your email.";
    else if (!EMAIL_PATTERN.test(email.trim())) {
      next.email = "That doesn't look like a valid email address.";
    }
    if (!repo.trim()) next.repo = "Please link your app's repo.";
    if (!description.trim()) next.description = "Tell us a little about your app.";
    return next;
  };

  const onSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setSubmitError(null);
    const next = validate();
    setErrors(next);
    if (Object.keys(next).length) return;

    if (website.trim()) {
      // Bot filled the honeypot. Pretend everything worked.
      setSubmitted(true);
      return;
    }

    setLoading(true);
    try {
      const body = new URLSearchParams({
        [FORM_FIELD_IDS.appName]: appName.trim(),
        [FORM_FIELD_IDS.repo]: repo.trim(),
        [FORM_FIELD_IDS.email]: email.trim(),
        [FORM_FIELD_IDS.description]: description.trim(),
        [FORM_FIELD_IDS.page]: window.location.href,
        [FORM_FIELD_IDS.referrer]: document.referrer,
      });
      // Google Forms sends no CORS headers, so this is fire-and-forget: the
      // response is opaque and "request completed" counts as success.
      await fetch(FORM_RESPONSE_URL, { method: "POST", mode: "no-cors", body });
      trackEvent("generate_lead", { form_name: "beta" });
      setSubmitted(true);
    } catch {
      setSubmitError(
        "Something went wrong sending your application. Please try again in a moment.",
      );
    } finally {
      setLoading(false);
    }
  };

  if (submitted) {
    return (
      <div className="flex flex-col items-center py-10 text-center" role="status" aria-live="polite">
        <span className="flex h-12 w-12 items-center justify-center bg-invert-bg text-invert-fg">
          <CheckIcon aria-hidden className="h-5 w-5" />
        </span>
        <h2 className="mt-6 font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
          You&rsquo;re on the list
        </h2>
        <p className="mt-3 max-w-xs leading-relaxed text-muted">
          Thanks for applying with {appName.trim()}. We review applications
          regularly and will reach out at{" "}
          <span className="text-ink">{email.trim()}</span>{" "}
          if it&rsquo;s selected.
        </p>
        <Button href="/" variant="secondary" className="mt-8">
          Back to the homepage
        </Button>
      </div>
    );
  }

  return (
    <form className="flex flex-col gap-5" onSubmit={onSubmit} noValidate>
      {submitError && (
        <p className="border border-danger px-4 py-3 text-sm text-danger">
          {submitError}
        </p>
      )}

      <Input
        id="beta-app-name"
        label="App name"
        type="text"
        autoComplete="off"
        value={appName}
        onChange={(e) => setAppName(e.target.value)}
        placeholder="My App"
        isError={!!errors.appName}
        errorMessage={errors.appName}
      />

      <Input
        id="beta-email"
        label="Email"
        type="email"
        autoComplete="email"
        value={email}
        onChange={(e) => setEmail(e.target.value)}
        placeholder="you@example.com"
        isError={!!errors.email}
        errorMessage={errors.email}
      />

      <Input
        id="beta-repo"
        label="Repo"
        type="url"
        autoComplete="url"
        value={repo}
        onChange={(e) => setRepo(e.target.value)}
        placeholder="https://github.com/you/my-app"
        isError={!!errors.repo}
        errorMessage={errors.repo}
      />

      <Textarea
        id="beta-description"
        label="Description"
        value={description}
        onChange={(e) => setDescription(e.target.value)}
        placeholder="What does your app do, and who is it for?"
        isError={!!errors.description}
        errorMessage={errors.description}
      />

      <div aria-hidden="true" className="absolute -left-[9999px] top-auto h-px w-px overflow-hidden">
        <label htmlFor="beta-website">Website</label>
        <input
          id="beta-website"
          type="text"
          tabIndex={-1}
          autoComplete="off"
          value={website}
          onChange={(e) => setWebsite(e.target.value)}
        />
      </div>

      <Button type="submit" size="lg" loading={loading} className="mt-1 w-full">
        Request access
      </Button>

      <p className="text-center text-xs leading-relaxed text-faint">
        We only use this to contact you about the beta.
      </p>
    </form>
  );
}
