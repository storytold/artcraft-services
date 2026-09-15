"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import {
  ArrowRightIcon,
  AudioLinesIcon,
  LoaderCircleIcon,
  SparklesIcon,
  XIcon,
} from "lucide-react";
import { Button, Select, Switch } from "@/components/ui";
import { generateVideo, getVideoJob } from "@/lib/api";
import { H3, H3_GATE_PERKS, H3_SAMPLE_PROMPTS } from "@/lib/campaign-data";
import { webappUrl } from "@/lib/links";
import { useAccount } from "@/lib/use-account";
import AuthGateModal from "./auth-gate-modal";

// Interactive hero for the MiniMax H3 page, ported from the Vite site: a
// real promptbox that enqueues an H3 generation through the omni-gen API,
// polls the job, and plays the result inline. Logged-out visitors hit the
// inline auth gate and the prompt they typed runs the moment they're in.
// Keyframe/reference uploads (which need the media upload API) are left to
// the full app.

const ASPECT_RATIOS = [
  { value: "wide_sixteen_by_nine", label: "16:9" },
  { value: "tall_nine_by_sixteen", label: "9:16" },
  { value: "square", label: "1:1" },
];
const RESOLUTIONS = [
  { value: "two_k", label: "2K" },
  { value: "seven_twenty_p", label: "720p" },
];
const DURATIONS = [5, 8, 10, 12, 15].map((s) => ({ value: String(s), label: `${s}s` }));
const MAX_PROMPT_LENGTH = 2000;

const POLL_INTERVAL_MS = 4000;
const POLL_MAX_ATTEMPTS = 180; // 12 min; video takes longer than images

type Generation = {
  id: string;
  prompt: string;
  status: "pending" | "complete" | "failed";
  videoUrl?: string;
  error?: string;
};

export default function H3PromptBox() {
  const { user, loading: accountLoading } = useAccount();
  // The gate may log the visitor in mid-page; track that locally so the
  // shared account snapshot (fetched once) doesn't have to be refreshed.
  const [authed, setAuthed] = useState(false);
  const loggedIn = !!user || authed;

  const [prompt, setPrompt] = useState("");
  const [aspectRatio, setAspectRatio] = useState(ASPECT_RATIOS[0].value);
  const [resolution, setResolution] = useState(RESOLUTIONS[0].value);
  const [duration, setDuration] = useState("10");
  const [sound, setSound] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [gateOpen, setGateOpen] = useState(false);
  const [generations, setGenerations] = useState<Generation[]>([]);
  const [error, setError] = useState<string | null>(null);

  const resultsRef = useRef<HTMLDivElement>(null);
  const stops = useRef(new Map<string, () => void>());

  // Stop in-flight polls when the visitor leaves; completed videos are in
  // their library either way.
  useEffect(() => {
    const map = stops.current;
    return () => {
      map.forEach((stop) => stop());
      map.clear();
    };
  }, []);

  const patch = useCallback((id: string, p: Partial<Generation>) => {
    setGenerations((prev) => prev.map((g) => (g.id === id ? { ...g, ...p } : g)));
  }, []);

  const run = useCallback(async () => {
    const trimmed = prompt.trim();
    if (!trimmed) return;
    setError(null);
    setSubmitting(true);
    const id = crypto.randomUUID();
    setGenerations((prev) => [{ id, prompt: trimmed, status: "pending" }, ...prev]);
    requestAnimationFrame(() =>
      resultsRef.current?.scrollIntoView({ behavior: "smooth", block: "center" }),
    );

    const result = await generateVideo({
      model: H3.modelId,
      prompt: trimmed,
      aspectRatio,
      resolution,
      durationSeconds: Number(duration),
      generateAudio: sound,
    });
    setSubmitting(false);
    if (!result.success) {
      patch(id, { status: "failed", error: result.errorMessage });
      return;
    }

    let attempts = 0;
    let stopped = false;
    const poll = async () => {
      if (stopped) return;
      attempts += 1;
      const job = await getVideoJob(result.data.jobToken);
      if (stopped) return;
      if (job.status === "complete") {
        patch(id, { status: "complete", videoUrl: job.videoUrl });
        stops.current.delete(id);
      } else if (job.status === "failed") {
        patch(id, { status: "failed", error: job.error });
        stops.current.delete(id);
      } else if (attempts >= POLL_MAX_ATTEMPTS) {
        patch(id, { status: "failed", error: "Generation timed out" });
        stops.current.delete(id);
      } else {
        setTimeout(poll, POLL_INTERVAL_MS);
      }
    };
    stops.current.set(id, () => {
      stopped = true;
    });
    setTimeout(poll, 3000);
  }, [prompt, aspectRatio, resolution, duration, sound, patch]);

  const submit = () => {
    if (!prompt.trim() || submitting) return;
    if (prompt.length > MAX_PROMPT_LENGTH) {
      setError(`Prompt exceeds the ${MAX_PROMPT_LENGTH} character limit for this model`);
      return;
    }
    if (!loggedIn) {
      setGateOpen(true);
      return;
    }
    void run();
  };

  const dismiss = (id: string) => {
    stops.current.get(id)?.();
    stops.current.delete(id);
    setGenerations((prev) => prev.filter((g) => g.id !== id));
  };

  return (
    <div className="w-full max-w-3xl text-left">
      <div className="border border-line bg-bg-raised">
        <div className="flex items-center justify-between gap-4 border-b border-line px-4 py-2.5">
          <p className="hud-label flex items-center gap-2 text-muted">
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img src={H3.icon} alt="" className="themed-logo h-3.5 w-3.5" />
            {H3.name}
            <span className="bg-accent px-1.5 py-0.5 font-bold text-white">Free</span>
          </p>
          <p className="hud-label text-faint">
            {prompt.length} / {MAX_PROMPT_LENGTH}
          </p>
        </div>
        <textarea
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) submit();
          }}
          rows={4}
          maxLength={MAX_PROMPT_LENGTH}
          placeholder="Describe the video you want to generate…"
          className="block w-full resize-none bg-transparent px-4 py-4 text-base leading-relaxed text-ink outline-none placeholder:text-faint"
        />
        <div className="flex flex-wrap items-center gap-3 border-t border-line px-4 py-3">
          <Select
            id="h3-aspect"
            value={aspectRatio}
            onChange={(v) => setAspectRatio(String(v))}
            options={ASPECT_RATIOS}
            className="w-24"
          />
          <Select
            id="h3-resolution"
            value={resolution}
            onChange={(v) => setResolution(String(v))}
            options={RESOLUTIONS}
            className="w-24"
          />
          <Select
            id="h3-duration"
            value={duration}
            onChange={(v) => setDuration(String(v))}
            options={DURATIONS}
            className="w-24"
          />
          <label className="hud-label flex items-center gap-2 text-muted">
            <AudioLinesIcon aria-hidden className="h-3.5 w-3.5" />
            Sound
            <Switch enabled={sound} setEnabled={setSound} aria-label="Generate sound" />
          </label>
          <Button
            type="button"
            onClick={submit}
            loading={submitting}
            disabled={!prompt.trim() || accountLoading}
            className="ml-auto"
          >
            <SparklesIcon aria-hidden className="h-3.5 w-3.5" />
            Generate
          </Button>
        </div>
      </div>

      {error && (
        <p role="alert" className="mt-3 font-mono text-[10px] uppercase tracking-[0.1em] text-danger">
          {error}
        </p>
      )}

      <div className="mt-4 flex flex-wrap items-center gap-2">
        <span className="hud-label text-faint">Try:</span>
        {H3_SAMPLE_PROMPTS.map((sample) => (
          <button
            key={sample}
            type="button"
            onClick={() => setPrompt(sample)}
            title={sample}
            className="max-w-full truncate border border-line px-3 py-1.5 text-xs text-muted hover:border-line-strong hover:text-ink sm:max-w-[16rem]"
          >
            {sample}
          </button>
        ))}
      </div>
      <p className="hud-label mt-4 text-center text-faint">
        Free {H3.name} generations require a free ArtCraft account. New here?
        You can sign up when you hit Generate.
      </p>

      <div ref={resultsRef} className="mt-10 flex flex-col gap-px bg-line empty:hidden">
        {generations.map((gen) => (
          <GenerationCard key={gen.id} generation={gen} onDismiss={dismiss} />
        ))}
        {generations.length > 0 && (
          <p className="hud-label bg-bg py-3 text-center text-faint">
            Your videos are also saved to{" "}
            <a href={webappUrl("/library")} className="text-muted underline underline-offset-4 hover:text-ink">
              your ArtCraft library
            </a>
            .
          </p>
        )}
      </div>

      <AuthGateModal
        isOpen={gateOpen}
        onClose={() => setGateOpen(false)}
        onAuthed={() => {
          setGateOpen(false);
          setAuthed(true);
          void run();
        }}
        signupSource={H3.signupSource}
        headline="Create a free account to generate."
        subtitle={`Your prompt is ready to go. ${H3.name} generations are free, and signing up takes less than a minute.`}
        perks={H3_GATE_PERKS}
      />
    </div>
  );
}

function GenerationCard({
  generation,
  onDismiss,
}: {
  generation: Generation;
  onDismiss: (id: string) => void;
}) {
  if (generation.status === "complete" && generation.videoUrl) {
    return (
      <figure className="bg-bg">
        <video
          src={generation.videoUrl}
          controls
          playsInline
          preload="metadata"
          className="aspect-video w-full bg-bg-sunken object-contain"
        />
        <figcaption className="truncate border-t border-line px-5 py-3 text-sm text-muted" title={generation.prompt}>
          {generation.prompt}
        </figcaption>
      </figure>
    );
  }

  if (generation.status === "failed") {
    return (
      <div className="flex items-start justify-between gap-4 border border-danger bg-bg px-5 py-4">
        <div className="min-w-0">
          <p className="hud-label text-danger">Generation failed</p>
          <p className="mt-1 text-sm text-muted">
            {generation.error ?? "Something went wrong. Please try again."}
          </p>
        </div>
        <button
          type="button"
          onClick={() => onDismiss(generation.id)}
          aria-label="Dismiss"
          className="shrink-0 text-faint hover:text-ink"
        >
          <XIcon aria-hidden className="h-4 w-4" />
        </button>
      </div>
    );
  }

  return (
    <div className="flex aspect-video w-full flex-col items-center justify-center gap-3 bg-bg px-6 text-center">
      <LoaderCircleIcon aria-hidden className="h-5 w-5 animate-spin text-accent-ink" />
      <p className="max-w-md truncate text-sm text-ink" title={generation.prompt}>
        {generation.prompt}
      </p>
      <p className="max-w-lg text-xs leading-relaxed text-muted">
        Generating your video. Free {H3.name} generations share a queue, so this
        can take a few minutes (longer at peak). Keep this tab open until it
        finishes; the video also lands in your library.
      </p>
      <Button
        href={webappUrl("/create-video")}
        target="_blank"
        rel="noreferrer"
        variant="secondary"
        size="sm"
        className="mt-1"
      >
        Keep creating in the app while you wait
        <ArrowRightIcon aria-hidden className="h-3 w-3" />
      </Button>
      <p className="hud-label text-faint">Opens in a new tab, so your generation keeps going here.</p>
    </div>
  );
}
