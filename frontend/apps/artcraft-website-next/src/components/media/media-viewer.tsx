"use client";

import { useCallback, useEffect, useState, type ReactNode } from "react";
import Link from "next/link";
import {
  ArrowDownToLineIcon,
  BoxIcon,
  CheckIcon,
  CopyIcon,
  ExternalLinkIcon,
  ImageIcon,
  InfoIcon,
  LinkIcon,
  LoaderCircleIcon,
  PencilIcon,
  UserIcon,
} from "lucide-react";
import { twMerge } from "tailwind-merge";
import { SectionShell, SectionEyebrow } from "@/components/landing/section-shell";
import { Button } from "@/components/ui";
import {
  getMediaFile,
  getPrompt,
  type MediaFile,
  type Prompt,
} from "@/lib/api";
import { webappUrl } from "@/lib/links";
import {
  addCorsParam,
  downloadMediaFile,
  formatAspectRatio,
  formatCreatedAt,
  formatDuration,
  formatResolution,
  mediaKindForUrl,
  SHARE_URL_BASE,
  thumbnailUrl,
} from "@/lib/media";

const COPY_FEEDBACK_MS = 1500;
const REFERENCE_THUMB_WIDTH = 128;

// Ported from the Vite site's pages/media + LightboxDetails: the media
// preview beside a details rail (author, prompt, reference images, info)
// with share / download / open-in-app actions. Server HTML renders the
// loading frame; the media record and its prompt load on the client.
type LoadState =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "ready"; media: MediaFile; prompt: Prompt | null };

type Dimensions = { width: number; height: number };

export default function MediaViewer({ token }: { token: string }) {
  const [state, setState] = useState<LoadState>({ status: "loading" });
  const [dimensions, setDimensions] = useState<Dimensions | null>(null);

  useEffect(() => {
    let live = true;
    setState({ status: "loading" });
    setDimensions(null);

    (async () => {
      const mediaResult = await getMediaFile(token);
      if (!mediaResult.success) {
        return { status: "error", message: mediaResult.errorMessage } as const;
      }
      const media = mediaResult.data;
      const promptResult = media.maybe_prompt_token
        ? await getPrompt(media.maybe_prompt_token)
        : null;
      return {
        status: "ready",
        media,
        prompt: promptResult?.success ? promptResult.data : null,
      } as const;
    })().then((next) => {
      if (live) setState(next);
    });

    return () => {
      live = false;
    };
  }, [token]);

  const media = state.status === "ready" ? state.media : null;
  const prompt = state.status === "ready" ? state.prompt : null;
  const mediaUrl = media?.media_links.cdn_url ?? null;

  return (
    <SectionShell id="media">
      <SectionEyebrow
        index="01"
        label="Shared media"
        annotation={media ? `Token · ${media.token}` : undefined}
      />
      <div className="flex flex-col lg:flex-row lg:min-h-[calc(100vh-6rem)]">
        <div className="relative flex min-h-[50vh] flex-1 items-center justify-center bg-bg-sunken lg:min-h-0">
          {state.status === "loading" && <Spinner />}
          {state.status === "error" && (
            <EmptyState title="Media not available" message={state.message} />
          )}
          {mediaUrl && (
            <MediaPreview
              url={mediaUrl}
              token={token}
              onDimensions={setDimensions}
            />
          )}
        </div>

        <aside className="flex w-full shrink-0 flex-col border-t border-line lg:w-[340px] lg:border-t-0 lg:border-l">
          {state.status === "loading" ? (
            <DetailsSkeleton />
          ) : (
            <DetailsBody
              media={media}
              prompt={prompt}
              dimensions={dimensions}
            />
          )}
          <Actions token={token} media={media} />
        </aside>
      </div>
    </SectionShell>
  );
}

// ── Preview ──────────────────────────────────────────────────────────────

function MediaPreview({
  url,
  token,
  onDimensions,
}: {
  url: string;
  token: string;
  onDimensions: (dimensions: Dimensions) => void;
}) {
  const [loaded, setLoaded] = useState(false);
  const [failed, setFailed] = useState(false);
  const kind = mediaKindForUrl(url);
  const src = addCorsParam(url);

  if (kind === "3d") {
    return (
      <EmptyState
        icon={<BoxIcon aria-hidden className="h-6 w-6" />}
        title="3D model"
        message="Open this model in ArtCraft to view it in 3D, or download the file below."
      >
        <Button href={webappUrl(`media/${token}`)} size="sm" className="mt-6">
          Open in ArtCraft
          <ExternalLinkIcon aria-hidden className="h-3.5 w-3.5" />
        </Button>
      </EmptyState>
    );
  }

  if (failed) {
    return (
      <EmptyState
        title="Preview unavailable"
        message="The file could not be loaded. You can still download it below."
      />
    );
  }

  const onReady = (dimensions: Dimensions) => {
    setLoaded(true);
    onDimensions(dimensions);
  };

  return (
    <div className="relative flex h-full w-full items-center justify-center">
      {!loaded && <Spinner />}
      {kind === "video" ? (
        <video
          src={src}
          className="h-full max-h-[calc(100vh-6rem)] w-full object-contain"
          controls
          autoPlay
          loop
          muted
          playsInline
          onError={() => setFailed(true)}
          onLoadedData={(e) =>
            onReady({
              width: e.currentTarget.videoWidth,
              height: e.currentTarget.videoHeight,
            })
          }
        />
      ) : (
        // Plain <img>: user-generated CDN content with unknown dimensions,
        // outside next/image's remote-pattern allowlist.
        // eslint-disable-next-line @next/next/no-img-element
        <img
          src={src}
          alt="Shared ArtCraft generation"
          className={twMerge(
            "h-full max-h-[calc(100vh-6rem)] w-full object-contain transition-opacity duration-300",
            loaded ? "opacity-100" : "opacity-0",
          )}
          onError={() => setFailed(true)}
          onLoad={(e) =>
            onReady({
              width: e.currentTarget.naturalWidth,
              height: e.currentTarget.naturalHeight,
            })
          }
        />
      )}
    </div>
  );
}

// ── Details rail ─────────────────────────────────────────────────────────

function DetailsBody({
  media,
  prompt,
  dimensions,
}: {
  media: MediaFile | null;
  prompt: Prompt | null;
  dimensions: Dimensions | null;
}) {
  if (!media) return <div className="flex-1" />;

  const creator = media.maybe_creator_user;
  const references = prompt?.maybe_context_images ?? [];
  const infoRows: [string, ReactNode][] = [];
  if (prompt?.maybe_model_type) infoRows.push(["Model", prompt.maybe_model_type]);
  if (prompt?.maybe_generation_provider) {
    infoRows.push(["Provider", prompt.maybe_generation_provider]);
  }
  if (prompt?.maybe_aspect_ratio) {
    infoRows.push(["Aspect ratio", formatAspectRatio(prompt.maybe_aspect_ratio)]);
  }
  if (prompt?.maybe_resolution) {
    infoRows.push(["Resolution", formatResolution(prompt.maybe_resolution)]);
  }
  if (prompt?.maybe_duration_seconds != null) {
    infoRows.push(["Duration", formatDuration(prompt.maybe_duration_seconds)]);
  }
  if (prompt?.maybe_generate_audio != null) {
    infoRows.push(["Audio", prompt.maybe_generate_audio ? "On" : "Off"]);
  }
  if (dimensions) {
    infoRows.push(["Size", `${dimensions.width} × ${dimensions.height}`]);
  }
  infoRows.push(["Created", formatCreatedAt(media.created_at)]);

  return (
    <div className="flex flex-1 flex-col gap-6 p-6">
      {creator && (
        <div className="flex items-center gap-3">
          <span className="flex h-9 w-9 shrink-0 items-center justify-center border border-line bg-bg-raised text-muted">
            <UserIcon aria-hidden className="h-4 w-4" />
          </span>
          <div className="min-w-0">
            <p className="truncate text-sm font-semibold text-ink-strong">
              {creator.display_name}
            </p>
            <p className="hud-label text-faint">Author</p>
          </div>
        </div>
      )}

      {media.maybe_prompt_token && (
        <Section icon={<PencilIcon aria-hidden className="h-3.5 w-3.5" />} label="Prompt">
          <PromptText text={prompt?.maybe_positive_prompt ?? null} />
        </Section>
      )}

      {references.length > 0 && (
        <Section icon={<ImageIcon aria-hidden className="h-3.5 w-3.5" />} label="Reference images">
          <div className="grid grid-cols-5 gap-2">
            {references.map((reference, index) => (
              <Link
                key={reference.media_token}
                href={`/media/${reference.media_token}`}
                className="relative block aspect-square overflow-hidden border border-line bg-bg-sunken hover:border-line-strong"
              >
                {/* eslint-disable-next-line @next/next/no-img-element */}
                <img
                  src={
                    thumbnailUrl(
                      reference.media_links.maybe_thumbnail_template,
                      REFERENCE_THUMB_WIDTH,
                    ) ?? reference.media_links.cdn_url
                  }
                  alt={`Reference ${index + 1}`}
                  className="h-full w-full object-cover"
                />
              </Link>
            ))}
          </div>
        </Section>
      )}

      <Section icon={<InfoIcon aria-hidden className="h-3.5 w-3.5" />} label="Information">
        <dl className="border border-line bg-bg-raised">
          {infoRows.map(([label, value]) => (
            <div
              key={label}
              className="flex items-center justify-between gap-4 border-b border-line px-4 py-3 last:border-b-0"
            >
              <dt className="text-sm text-muted">{label}</dt>
              <dd className="truncate text-right text-sm font-medium text-ink">
                {value}
              </dd>
            </div>
          ))}
        </dl>
      </Section>
    </div>
  );
}

function PromptText({ text }: { text: string | null }) {
  const [expanded, setExpanded] = useState(false);
  const copy = useCopyFeedback(text);

  return (
    <div className="space-y-2">
      <div
        className={twMerge(
          "border border-line bg-bg-raised px-4 py-3 text-sm leading-relaxed text-ink break-words",
          !expanded && "line-clamp-4",
        )}
      >
        {text ?? <span className="text-muted">No prompt</span>}
      </div>
      {text && (
        <div className="flex items-center justify-between">
          <button
            type="button"
            onClick={() => setExpanded((prev) => !prev)}
            className="hud-label text-muted hover:text-ink"
          >
            {expanded ? "Show less" : "Show more"}
          </button>
          <button
            type="button"
            onClick={copy.trigger}
            className="hud-label inline-flex items-center gap-1.5 text-muted hover:text-ink"
          >
            {copy.copied ? (
              <CheckIcon aria-hidden className="h-3 w-3" />
            ) : (
              <CopyIcon aria-hidden className="h-3 w-3" />
            )}
            {copy.copied ? "Copied" : "Copy"}
          </button>
        </div>
      )}
    </div>
  );
}

function Actions({ token, media }: { token: string; media: MediaFile | null }) {
  const share = useCopyFeedback(`${SHARE_URL_BASE}${token}`);
  const [downloading, setDownloading] = useState(false);
  const [downloadError, setDownloadError] = useState(false);
  const mediaUrl = media?.media_links.cdn_url;

  const handleDownload = useCallback(async () => {
    if (!mediaUrl || downloading) return;
    setDownloading(true);
    setDownloadError(false);
    try {
      await downloadMediaFile({
        url: mediaUrl,
        filename: `artcraft-${token}`,
        mediaClass: media?.media_class,
      });
    } catch {
      setDownloadError(true);
    } finally {
      setDownloading(false);
    }
  }, [mediaUrl, downloading, token, media?.media_class]);

  return (
    <div className="space-y-2 border-t border-line p-6">
      <div className="grid grid-cols-2 gap-2">
        <Button variant="secondary" className="w-full" onClick={share.trigger}>
          {share.copied ? (
            <CheckIcon aria-hidden className="h-3.5 w-3.5" />
          ) : (
            <LinkIcon aria-hidden className="h-3.5 w-3.5" />
          )}
          {share.copied ? "Copied" : "Share"}
        </Button>
        <Button
          variant="secondary"
          className="w-full"
          disabled={!mediaUrl}
          loading={downloading}
          onClick={handleDownload}
        >
          {!downloading && (
            <ArrowDownToLineIcon aria-hidden className="h-3.5 w-3.5" />
          )}
          Download
        </Button>
      </div>
      {downloadError && (
        <p className="hud-label text-danger">Could not download file.</p>
      )}
      {media && (
        <Button
          variant="action"
          className="w-full"
          href={webappUrl(`media/${token}`)}
        >
          Open in ArtCraft
          <ExternalLinkIcon aria-hidden className="h-3.5 w-3.5" />
        </Button>
      )}
      <Button className="w-full" href="/download">
        <ArrowDownToLineIcon aria-hidden className="h-3.5 w-3.5" />
        Download ArtCraft
      </Button>
    </div>
  );
}

// ── Small pieces ─────────────────────────────────────────────────────────

function Section({
  icon,
  label,
  children,
}: {
  icon: ReactNode;
  label: string;
  children: ReactNode;
}) {
  return (
    <div className="space-y-2">
      <p className="hud-label inline-flex items-center gap-2 text-muted">
        {icon}
        {label}
      </p>
      {children}
    </div>
  );
}

function EmptyState({
  icon,
  title,
  message,
  children,
}: {
  icon?: ReactNode;
  title: string;
  message: string;
  children?: ReactNode;
}) {
  return (
    <div className="flex flex-col items-center px-6 py-16 text-center text-muted">
      {icon}
      <p className={twMerge("hud-label text-ink", icon && "mt-4")}>{title}</p>
      <p className="mt-2 max-w-sm text-sm leading-relaxed">{message}</p>
      {children}
    </div>
  );
}

function Spinner() {
  return (
    <div className="absolute inset-0 flex items-center justify-center">
      <LoaderCircleIcon
        aria-label="Loading"
        className="h-8 w-8 animate-spin text-muted"
      />
    </div>
  );
}

function DetailsSkeleton() {
  return (
    <div className="flex-1 animate-pulse space-y-6 p-6" aria-hidden>
      <div className="h-9 w-40 bg-bg-sunken" />
      <div className="space-y-2">
        <div className="h-3 w-16 bg-bg-sunken" />
        <div className="h-24 w-full bg-bg-sunken" />
      </div>
      <div className="space-y-2">
        <div className="h-3 w-24 bg-bg-sunken" />
        <div className="h-32 w-full bg-bg-sunken" />
      </div>
    </div>
  );
}

// Copies `text` to the clipboard and reports "copied" briefly afterwards.
function useCopyFeedback(text: string | null) {
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!copied) return;
    const timeout = window.setTimeout(() => setCopied(false), COPY_FEEDBACK_MS);
    return () => window.clearTimeout(timeout);
  }, [copied]);

  const trigger = useCallback(() => {
    if (!text) return;
    navigator.clipboard
      .writeText(text)
      .then(() => setCopied(true))
      .catch(() => setCopied(false));
  }, [text]);

  return { copied, trigger };
}
