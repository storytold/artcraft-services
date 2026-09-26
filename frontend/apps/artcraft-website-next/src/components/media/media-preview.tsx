"use client";

import dynamic from "next/dynamic";
import { useState } from "react";
import { FileIcon, LoaderCircleIcon } from "lucide-react";
import { Button } from "@/components/ui";
import { corsMediaUrl, mediaFormat, mediaKind, mediaPoster, mediaTitle, type SharedMedia } from "@/lib/media";
import AudioPlayer from "./audio-player";

// WebGL and Spark are only loaded when someone opens a 3D creation.
const MediaViewer3D = dynamic(() => import("./media-viewer-3d"), {
  ssr: false,
  loading: () => <PreviewLoading label="Loading 3D viewer" />,
});

export default function MediaPreview({ media }: { media: SharedMedia }) {
  const [loaded, setLoaded] = useState(false);
  const [failed, setFailed] = useState(false);
  const kind = mediaKind(media);
  const url = corsMediaUrl(media.media_links.cdn_url);
  const title = mediaTitle(media);

  return (
    <div className="relative flex min-h-[360px] min-w-0 items-center justify-center bg-bg-sunken lg:min-h-[620px]" data-media-kind={kind}>
      {failed ? (
        <div role="alert" className="flex flex-col items-center gap-4 p-10 text-center">
          <p className="text-muted">This browser could not display the file.</p>
          <Button href={media.media_links.cdn_url} target="_blank" rel="noopener noreferrer" variant="secondary">Open original</Button>
        </div>
      ) : kind === "mesh" || kind === "splat" ? (
        <MediaViewer3D url={url} kind={kind} format={mediaFormat(media)} />
      ) : kind === "image" ? (
        <>
          {/* User media comes from the API; preserve the original resolution. */}
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img src={url} alt={title} onLoad={() => setLoaded(true)} onError={() => setFailed(true)}
            className="max-h-[80svh] w-full object-contain" />
          {!loaded && <PreviewLoading label="Loading image" />}
        </>
      ) : kind === "video" ? (
        <video src={url} aria-label={title} controls playsInline preload="metadata" poster={mediaPoster(media)}
          onError={() => setFailed(true)} className="max-h-[80svh] w-full" />
      ) : kind === "audio" ? (
        <AudioPlayer src={url} title={title} onError={() => setFailed(true)} />
      ) : (
        <div className="flex flex-col items-center gap-4 p-10 text-center text-muted">
          <FileIcon aria-hidden className="h-10 w-10" strokeWidth={1} />
          <p>A browser preview isn’t available for this file. You can download the original or open it in ArtCraft.</p>
        </div>
      )}
    </div>
  );
}

function PreviewLoading({ label }: { label: string }) {
  return (
    <div role="status" className="absolute inset-0 flex items-center justify-center gap-3 bg-bg-sunken/90 text-muted">
      <LoaderCircleIcon aria-hidden className="h-5 w-5 animate-spin motion-reduce:animate-none" />
      <span className="hud-label">{label}</span>
    </div>
  );
}
