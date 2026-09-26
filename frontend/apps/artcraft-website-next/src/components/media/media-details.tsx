"use client";

import { useEffect, useRef, useState } from "react";
import Link from "next/link";
import { ArrowDownToLineIcon, CheckIcon, CopyIcon, ExternalLinkIcon } from "lucide-react";
import { Button } from "@/components/ui";
import { MEDIA_LABELS, corsMediaUrl, mediaDetailLabel, mediaFormat, mediaKind, safeMediaUrl, type MediaPrompt, type SharedMedia } from "@/lib/media";

export default function MediaDetails({ media, prompt, promptLoading }: {
  media: SharedMedia; prompt: MediaPrompt | null; promptLoading: boolean;
}) {
  const [copied, setCopied] = useState<string | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [message, setMessage] = useState("");
  const downloadController = useRef<AbortController | null>(null);
  useEffect(() => () => downloadController.current?.abort(), []);

  async function copy(value: string, label: string) {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(label);
      setMessage("");
    } catch {
      setMessage("Copy is unavailable in this browser.");
    }
  }

  async function download() {
    setDownloading(true);
    setMessage("");
    const controller = new AbortController();
    downloadController.current = controller;
    try {
      const url = new URL(corsMediaUrl(media.media_links.cdn_url));
      url.searchParams.set("dl", "1");
      const response = await fetch(url, { credentials: "omit", signal: controller.signal });
      if (!response.ok) throw new Error("Download failed");
      const blob = await response.blob();
      if (controller.signal.aborted) return;
      const objectUrl = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = objectUrl;
      anchor.download = `${media.token}.${mediaFormat(media) || "bin"}`;
      document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      // Give the browser time to start consuming the download before revoking.
      setTimeout(() => URL.revokeObjectURL(objectUrl), 1000);
    } catch {
      if (!controller.signal.aborted) setMessage("Download failed. Use Open original to save the file directly.");
    } finally {
      if (!controller.signal.aborted) setDownloading(false);
    }
  }

  const kind = mediaKind(media);
  const created = media.created_at ? new Date(media.created_at) : null;
  const details = [
    ["Type", MEDIA_LABELS[kind]],
    ["Format", mediaFormat(media).toUpperCase()],
    ["Model", media.maybe_model_weight_info?.title || mediaDetailLabel(prompt?.maybe_model_type)],
    ["Provider", mediaDetailLabel(prompt?.maybe_generation_provider)],
    ["Aspect ratio", mediaDetailLabel(prompt?.maybe_aspect_ratio)],
    ["Resolution", mediaDetailLabel(prompt?.maybe_resolution)],
    ["Duration", media.maybe_duration_millis != null ? `${(media.maybe_duration_millis / 1000).toFixed(1)} sec` : prompt?.maybe_duration_seconds != null ? `${prompt.maybe_duration_seconds} sec` : null],
    ["Audio", prompt?.maybe_generate_audio == null ? null : prompt.maybe_generate_audio ? "On" : "Off"],
    ["Created", created && !Number.isNaN(created.valueOf()) ? created.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" }) : null],
  ].filter((entry) => entry[1]);

  return (
    <aside aria-label="Media details" className="min-w-0 border-t border-line bg-bg lg:border-l lg:border-t-0">
      <div className="border-b border-line p-6">
        <p className="hud-label mb-5 text-faint">Creation details</p>
        <dl className="space-y-3 text-sm">
          {details.map(([label, value]) => (
            <div key={label} className="flex justify-between gap-4">
              <dt className="shrink-0 text-muted">{label}</dt>
              <dd className="break-words text-right text-ink">{value}</dd>
            </div>
          ))}
        </dl>
      </div>

      {media.maybe_prompt_token && (
        <div className="border-b border-line p-6">
          <div className="mb-4 flex items-center justify-between gap-2">
            <h2 className="hud-label text-faint">Prompt</h2>
            {prompt?.maybe_positive_prompt && (
              <button type="button" className="text-muted hover:text-ink" aria-label={copied === "prompt" ? "Prompt copied" : "Copy prompt"}
                onClick={() => copy(prompt.maybe_positive_prompt!, "prompt")}>
                {copied === "prompt" ? <CheckIcon className="h-4 w-4" /> : <CopyIcon className="h-4 w-4" />}
              </button>
            )}
          </div>
          <p className="max-h-72 overflow-y-auto whitespace-pre-wrap break-words text-sm leading-relaxed text-muted">
            {promptLoading ? "Loading prompt…" : prompt?.maybe_positive_prompt || "Prompt details aren’t available for this creation."}
          </p>
          {prompt?.maybe_negative_prompt && (
            <details className="mt-4 text-sm text-muted">
              <summary className="cursor-pointer">Negative prompt</summary>
              <p className="mt-2 whitespace-pre-wrap break-words">{prompt.maybe_negative_prompt}</p>
            </details>
          )}
          {!!prompt?.maybe_context_images?.length && (
            <div className="mt-5 grid grid-cols-3 gap-2" aria-label="Reference images">
              {prompt.maybe_context_images.map((image, index) => {
                const src = safeMediaUrl(image.media_links.maybe_thumbnail_template?.replace("{WIDTH}", "256")) ?? safeMediaUrl(image.media_links.cdn_url);
                return src ? (
                  <Link key={`${image.media_token}-${index}`} href={`/media/${encodeURIComponent(image.media_token)}`} className="border border-line">
                    {/* eslint-disable-next-line @next/next/no-img-element */}
                    <img src={corsMediaUrl(src)} alt={image.semantic || `Reference ${index + 1}`} loading="lazy" className="aspect-square w-full object-cover" />
                  </Link>
                ) : null;
              })}
            </div>
          )}
        </div>
      )}

      {media.maybe_text_transcript && (
        <div className="border-b border-line p-6">
          <h2 className="hud-label mb-4 text-faint">Transcript / lyrics</h2>
          <p className="max-h-72 overflow-y-auto whitespace-pre-wrap break-words text-sm leading-relaxed text-muted">{media.maybe_text_transcript}</p>
        </div>
      )}

      <div className="flex flex-col gap-3 p-6">
        <Button onClick={download} loading={downloading} className="w-full">
          <ArrowDownToLineIcon aria-hidden className="h-4 w-4" />{downloading ? "Downloading" : "Download file"}
        </Button>
        <Button variant="secondary" className="w-full" onClick={() => copy(`${window.location.origin}/media/${encodeURIComponent(media.token)}`, "link")}>
          {copied === "link" ? <CheckIcon aria-hidden className="h-4 w-4" /> : <CopyIcon aria-hidden className="h-4 w-4" />}
          {copied === "link" ? "Link copied" : "Copy share link"}
        </Button>
        <a href={media.media_links.cdn_url} target="_blank" rel="noopener noreferrer" className="hud-label flex items-center justify-center gap-2 py-2 text-muted hover:text-ink">
          Open original <ExternalLinkIcon aria-hidden className="h-3 w-3" />
        </a>
        <p role="status" className="text-sm text-muted">{message}</p>
        <div className="mt-3 border-t border-line pt-5">
          <p className="mb-4 text-sm leading-relaxed text-muted">Make something of your own. Images, films, and worlds — all in ArtCraft.</p>
          <Button href="/download" variant="secondary" className="w-full">Get ArtCraft</Button>
        </div>
      </div>
    </aside>
  );
}
