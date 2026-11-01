"use client";

import { useState, type FormEvent } from "react";
import { CheckIcon } from "lucide-react";
import { twMerge } from "tailwind-merge";
import { Button, Input, Label } from "@/components/ui";
import { trackEvent } from "@/lib/analytics";
import { webappUrl } from "@/lib/links";

// Submissions go to a Google Form whose responses land in a Google Sheet.
// The form URL and field IDs are public by nature (they ship in the bundle).
const FORM_RESPONSE_URL =
  "https://docs.google.com/forms/d/e/1FAIpQLSfjRVkOzfBlR0uefebHWSaoEm7R0DRLhUKqwHQlNvBE8bmd-w/formResponse";

const FORM_FIELD_IDS = {
  name: "entry.666666785",
  company: "entry.121325393",
  email: "entry.467034813",
  type: "entry.2139364913",
  typeDetail: "entry.1914395461",
  page: "entry.709868934",
  referrer: "entry.617211358",
} as const;

// Every Google Form field is free text, so these map onto the existing
// "Type" / "Type Detail" columns without touching the form.
const ACCOUNT_TYPES = ["Personal", "Company"] as const;
type AccountType = (typeof ACCOUNT_TYPES)[number];

const COMPANY_SIZES = ["1–10", "11–50", "51–200", "201–1,000", "1,000+"] as const;
type CompanySize = (typeof COMPANY_SIZES)[number];

const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

type FieldErrors = Partial<Record<"name" | "email" | "accountType" | "companySize", string>>;

export default function BetaForm() {
  const [name, setName] = useState("");
  const [company, setCompany] = useState("");
  const [email, setEmail] = useState("");
  const [accountType, setAccountType] = useState<AccountType | null>(null);
  const [companySize, setCompanySize] = useState<CompanySize | null>(null);
  // Honeypot: hidden from humans; bots that fill it are silently dropped.
  const [website, setWebsite] = useState("");

  const [errors, setErrors] = useState<FieldErrors>({});
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [submitted, setSubmitted] = useState(false);

  const validate = (): FieldErrors => {
    const next: FieldErrors = {};
    if (!name.trim()) next.name = "Please enter your name.";
    if (!email.trim()) next.email = "Please enter your email.";
    else if (!EMAIL_PATTERN.test(email.trim())) {
      next.email = "That doesn't look like a valid email address.";
    }
    if (accountType === null) next.accountType = "Please choose personal or company.";
    else if (accountType === "Company" && companySize === null) {
      next.companySize = "Please pick your company size.";
    }
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

    const isCompany = accountType === "Company";
    setLoading(true);
    try {
      const body = new URLSearchParams({
        [FORM_FIELD_IDS.name]: name.trim(),
        [FORM_FIELD_IDS.company]: isCompany ? company.trim() : "",
        [FORM_FIELD_IDS.email]: email.trim(),
        [FORM_FIELD_IDS.type]: accountType ?? "",
        [FORM_FIELD_IDS.typeDetail]: isCompany ? (companySize ?? "") : "",
        [FORM_FIELD_IDS.page]: window.location.href,
        [FORM_FIELD_IDS.referrer]: document.referrer,
      });
      // Google Forms sends no CORS headers, so this is fire-and-forget: the
      // response is opaque and "request completed" counts as success.
      await fetch(FORM_RESPONSE_URL, { method: "POST", mode: "no-cors", body });
      trackEvent("generate_lead", { form_name: "beta", user_type: accountType ?? "" });
      setSubmitted(true);
    } catch {
      setSubmitError(
        "Something went wrong sending your application. Please try again in a moment.",
      );
    } finally {
      setLoading(false);
    }
  };

  const selectAccountType = (type: AccountType) => {
    setAccountType(type);
    setErrors((prev) => ({ ...prev, accountType: undefined, companySize: undefined }));
  };

  const selectCompanySize = (size: CompanySize) => {
    setCompanySize(size);
    setErrors((prev) => ({ ...prev, companySize: undefined }));
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
          Thanks, {name.trim().split(/\s+/)[0]}. We review applications
          regularly and will reach out at{" "}
          <span className="text-ink">{email.trim()}</span> when your spot
          opens.
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
        id="beta-name"
        label="Name"
        type="text"
        autoComplete="name"
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder="Ada Lovelace"
        isError={!!errors.name}
        errorMessage={errors.name}
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

      <ChoiceGroup
        id="beta-account-type"
        label="Signing up as"
        options={ACCOUNT_TYPES}
        value={accountType}
        onChange={selectAccountType}
        error={errors.accountType}
      />

      {accountType === "Company" && (
        <>
          <Input
            id="beta-company"
            label="Company name (optional)"
            type="text"
            autoComplete="organization"
            value={company}
            onChange={(e) => setCompany(e.target.value)}
            placeholder="Studio or company name"
          />
          <ChoiceGroup
            id="beta-company-size"
            label="Company size"
            options={COMPANY_SIZES}
            value={companySize}
            onChange={selectCompanySize}
            error={errors.companySize}
          />
        </>
      )}

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
        We only use this to contact you about the beta. Already have access?{" "}
        <a href={webappUrl("/login")} className="text-muted underline underline-offset-2 hover:text-ink">
          Log in
        </a>
      </p>
    </form>
  );
}

// Single-select row of squared pills, ARIA radio semantics.
function ChoiceGroup<T extends string>({
  id,
  label,
  options,
  value,
  onChange,
  error,
}: {
  id: string;
  label: string;
  options: readonly T[];
  value: T | null;
  onChange: (value: T) => void;
  error?: string;
}) {
  const labelId = `${id}-label`;
  const errorId = `${id}-error`;
  return (
    <div className="flex flex-col">
      <Label id={labelId}>{label}</Label>
      <div
        className="flex flex-wrap gap-px bg-line"
        role="radiogroup"
        aria-labelledby={labelId}
        aria-invalid={!!error}
        aria-describedby={error ? errorId : undefined}
      >
        {options.map((option) => {
          const selected = value === option;
          return (
            <button
              key={option}
              type="button"
              role="radio"
              aria-checked={selected}
              onClick={() => onChange(option)}
              className={twMerge(
                "hud-label h-10 flex-1 whitespace-nowrap px-4 transition-colors",
                selected
                  ? "bg-invert-bg text-invert-fg"
                  : "bg-bg text-muted hover:bg-bg-raised hover:text-ink",
              )}
            >
              {option}
            </button>
          );
        })}
      </div>
      {error && (
        <p id={errorId} className="mt-1 font-mono text-[10px] uppercase tracking-[0.1em] text-danger">
          {error}
        </p>
      )}
    </div>
  );
}
