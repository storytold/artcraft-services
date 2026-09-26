"use client";

import { useEffect, useState } from "react";
import { ArrowUpRightIcon, LoaderCircleIcon } from "lucide-react";
import { SectionShell, SectionEyebrow } from "@/components/landing/section-shell";
import { Accent } from "@/components/page/page-header";
import { Button } from "@/components/ui";
import { getMediaPrompt, getSharedMedia } from "@/lib/media-api";
import { MEDIA_LABELS, mediaKind, mediaTitle, type MediaPrompt, type SharedMedia } from "@/lib/media";
import { webappUrl } from "@/lib/links";
import MediaPreview from "./media-preview";
import MediaDetails from "./media-details";

type MediaState =
  | { status: "loading" }
  | { status: "ready"; media: SharedMedia }
  | { status: "error"; code?: number };

export default function MediaPage({ token }: { token: string }) {
  const [state, setState] = useState<MediaState>({ status: "loading" });
  const [prompt, setPrompt] = useState<MediaPrompt | null>(null);
  const [promptLoading, setPromptLoading] = useState(false);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    const controller = new AbortController();
    setState({ status: "loading" });
    setPrompt(null);
    setPromptLoading(false);
    async function load() {
      const result = await getSharedMedia(token, controller.signal);
      if (controller.signal.aborted) return;
      if (!result.success) {
        setState({ status: "error", code: result.status });
        return;
      }
      setState({ status: "ready", media: result.data });
      if (result.data.maybe_prompt_token) {
        setPromptLoading(true);
        const resultPrompt = await getMediaPrompt(result.data.maybe_prompt_token, controller.signal);
        if (controller.signal.aborted) return;
        setPrompt(resultPrompt.success ? resultPrompt.data : null);
        setPromptLoading(false);
      }
    }
    void load();
    return () => controller.abort();
  }, [token, attempt]);

  const media = state.status === "ready" ? state.media : null;
  const kind = media ? mediaKind(media) : null;
  const title = media ? mediaTitle(media) : "Shared creation";

  return (
    <SectionShell className="min-h-[70vh] overflow-x-clip md:overflow-x-visible">
      <SectionEyebrow index="01" label="Shared creation" annotation={kind ? MEDIA_LABELS[kind] : "Made with ArtCraft"} />
      <header className="flex flex-col items-start gap-6 px-6 py-8 sm:flex-row sm:items-end sm:justify-between md:px-10 md:py-10">
        <div className="w-full min-w-0 flex-1 sm:w-auto">
          <p className="hud-label mb-3 text-faint">The artist’s viewport</p>
          <h1 className="break-words font-display text-3xl font-medium leading-tight tracking-tight text-ink-strong sm:text-4xl">
            {media ? title : <>Shared <Accent>creation</Accent>.</>}
          </h1>
          {media?.maybe_creator_user && (
            <p className="mt-3 text-sm text-muted">
              By <span className="text-ink">{media.maybe_creator_user.display_name || media.maybe_creator_user.username}</span>
              <span className="ml-2 text-faint">@{media.maybe_creator_user.username}</span>
            </p>
          )}
        </div>
        <Button href={webappUrl(`/media/${encodeURIComponent(token)}`)} variant="secondary">
          Open in ArtCraft <ArrowUpRightIcon aria-hidden className="h-4 w-4" />
        </Button>
      </header>

      {state.status === "loading" && (
        <div role="status" className="flex min-h-[50vh] flex-col items-center justify-center gap-4 border-t border-line bg-bg-sunken text-muted">
          <LoaderCircleIcon aria-hidden className="h-6 w-6 animate-spin motion-reduce:animate-none" />
          <span className="hud-label">Loading media</span>
        </div>
      )}
      {state.status === "error" && (
        <div role="alert" className="flex min-h-[45vh] flex-col items-center justify-center gap-5 border-t border-line px-6 py-16 text-center">
          <h2 className="font-display text-2xl text-ink-strong">
            {state.code === 404 ? "Media not found" : state.code === 401 || state.code === 403 ? "This media is private" : "Unable to load this media"}
          </h2>
          <p className="max-w-md text-muted">
            {state.code === 404 ? "This link may have been removed or is no longer available."
              : state.code === 401 || state.code === 403 ? "Open ArtCraft and sign in to an account with access."
              : "The media could not be reached. Please try again."}
          </p>
          <Button variant="secondary" onClick={() => setAttempt((value) => value + 1)}>Try again</Button>
        </div>
      )}
      {media && (
        <div className="grid border-t border-line lg:grid-cols-[minmax(0,1fr)_340px]">
          <MediaPreview key={media.token} media={media} />
          <MediaDetails media={media} prompt={prompt} promptLoading={promptLoading} />
        </div>
      )}
      <noscript>
        <p className="px-6 py-12 text-muted">Enable JavaScript to load this shared media, or open it in ArtCraft using the link above.</p>
      </noscript>
    </SectionShell>
  );
}
