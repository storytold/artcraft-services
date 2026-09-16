"use client";

import { useState } from "react";
import {
  ArrowDownToLineIcon,
  FileArchiveIcon,
  ImageIcon,
  PlayIcon,
  VideoIcon,
} from "lucide-react";
import { twMerge } from "tailwind-merge";
import { SectionShell, SectionEyebrow } from "@/components/landing/section-shell";
import { Button, Modal } from "@/components/ui";
import { mediaUrl } from "@/lib/links";
import {
  PRESS_KIT_CATEGORIES,
  type PressKitAsset,
  type PressKitAssetType,
} from "@/lib/press-kit-data";

const TYPE_ICONS: Record<PressKitAssetType, typeof ImageIcon> = {
  video: VideoIcon,
  embed: VideoIcon,
  image: ImageIcon,
  link: FileArchiveIcon,
};

const isPlayable = (asset: PressKitAsset) =>
  asset.type === "video" || (asset.type === "embed" && !!asset.embedUrl);

// Site-relative asset paths resolve against the media host; absolute URLs
// (YouTube thumbnails, R2) pass through.
const assetUrl = (path: string) => (/^https?:\/\//.test(path) ? path : mediaUrl(path));

export default function PressKitGallery() {
  const [active, setActive] = useState<PressKitAsset | null>(null);
  const categories = PRESS_KIT_CATEGORIES.filter((c) => c.assets.length > 0);

  return (
    <>
      {categories.map((category, ci) => (
        <SectionShell key={category.name} id={ci === 0 ? "articles" : undefined}>
          <SectionEyebrow
            index={String(ci + 2).padStart(2, "0")}
            label={category.name}
            annotation={category.description}
          />
          <ul className="grid gap-px bg-line md:grid-cols-2 lg:grid-cols-3">
            {category.assets.map((asset, i) => (
              <li key={asset.title} className="bg-bg">
                <AssetCard
                  asset={asset}
                  index={String(i + 1).padStart(2, "0")}
                  onPlay={() => setActive(asset)}
                />
              </li>
            ))}
          </ul>
        </SectionShell>
      ))}

      <Modal
        isOpen={active !== null}
        onClose={() => setActive(null)}
        childPadding={false}
        accessibleTitle={active?.title}
        className="max-w-5xl overflow-hidden bg-black"
      >
        {active && (
          <div className="relative aspect-video w-full">
            {active.type === "embed" && active.embedUrl ? (
              <iframe
                src={`${active.embedUrl}?autoplay=1`}
                title={active.title}
                allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                allowFullScreen
                className="absolute inset-0 h-full w-full"
              />
            ) : (
              <video
                src={active.videoUrl ?? active.downloadUrl}
                className="absolute inset-0 h-full w-full"
                controls
                autoPlay
                playsInline
              />
            )}
          </div>
        )}
      </Modal>
    </>
  );
}

function AssetCard({
  asset,
  index,
  onPlay,
}: {
  asset: PressKitAsset;
  index: string;
  onPlay: () => void;
}) {
  const TypeIcon = TYPE_ICONS[asset.type];
  const playable = isPlayable(asset);
  const typeLabel = asset.type === "embed" ? "Video" : asset.type;

  const thumbnail = (
    <>
      {asset.thumbnail ? (
        // eslint-disable-next-line @next/next/no-img-element
        <img
          src={assetUrl(asset.thumbnail)}
          alt=""
          loading="lazy"
          className={twMerge(
            "absolute inset-0 h-full w-full transition-transform duration-500 group-hover:scale-[1.02]",
            asset.containThumbnail ? "object-contain p-8" : "object-cover",
          )}
        />
      ) : (
        <span className="absolute inset-0 flex items-center justify-center text-faint">
          <TypeIcon aria-hidden className="h-8 w-8" />
        </span>
      )}
      {playable && (
        <span className="hud-label absolute bottom-3 left-3 flex items-center gap-1.5 bg-invert-bg px-3 py-1.5 font-bold text-invert-fg">
          <PlayIcon aria-hidden className="h-3.5 w-3.5" />
          Play
        </span>
      )}
    </>
  );

  return (
    <article className="flex h-full flex-col">
      <div className="flex items-center justify-between gap-4 border-b border-line px-6 py-2.5">
        <p className="hud-label flex items-center gap-1.5 capitalize text-muted">
          <TypeIcon aria-hidden className="h-3 w-3" />
          {typeLabel}
        </p>
        <p className="hud-label text-faint">{index}</p>
      </div>

      {playable ? (
        <button
          type="button"
          onClick={onPlay}
          aria-label={`Play ${asset.title}`}
          className="group relative aspect-video w-full overflow-hidden bg-bg-sunken"
        >
          {thumbnail}
        </button>
      ) : (
        <div className="group relative aspect-video w-full overflow-hidden bg-bg-sunken">
          {thumbnail}
        </div>
      )}

      <div className="flex flex-1 flex-col px-6 py-5">
        <h3 className="font-display text-xl font-medium tracking-[-0.02em] text-ink-strong">
          {asset.title}
        </h3>
        {asset.description && (
          <p className="mt-1.5 text-sm leading-relaxed text-muted">
            {asset.description}
          </p>
        )}
        <div className="mt-auto pt-5">
          {asset.downloadUrl ? (
            <Button
              href={assetUrl(asset.downloadUrl)}
              external
              target="_blank"
              rel="noopener noreferrer"
              variant="secondary"
              className="w-full"
            >
              <ArrowDownToLineIcon aria-hidden className="h-3.5 w-3.5" />
              {asset.downloadLabel ?? "Download"}
              {asset.fileSize && (
                <span className="font-normal opacity-60">{asset.fileSize}</span>
              )}
            </Button>
          ) : (
            <Button disabled variant="secondary" className="w-full">
              Download coming soon
            </Button>
          )}
        </div>
      </div>
    </article>
  );
}
