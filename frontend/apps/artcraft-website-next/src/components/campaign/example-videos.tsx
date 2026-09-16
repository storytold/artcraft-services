"use client";

import { useState } from "react";
import { PlayIcon } from "lucide-react";
import { twMerge } from "tailwind-merge";
import type { ExampleVideo } from "@/lib/campaign-data";

// Click-to-play facades: the mp4s (several MB each) are only requested
// once the visitor presses play.
export default function ExampleVideos({
  examples,
  creditHref,
  creditLabel,
}: {
  examples: ExampleVideo[];
  creditHref: string;
  creditLabel: string;
}) {
  return (
    <ul className="grid gap-px border-t border-line bg-line sm:grid-cols-2 lg:grid-cols-6">
      {examples.map((example, i) => (
        <li key={example.src} className={twMerge("bg-bg", example.spanClass)}>
          <ExampleCard
            example={example}
            index={String(i + 1).padStart(2, "0")}
            creditHref={creditHref}
            creditLabel={creditLabel}
          />
        </li>
      ))}
    </ul>
  );
}

function ExampleCard({
  example,
  index,
  creditHref,
  creditLabel,
}: {
  example: ExampleVideo;
  index: string;
  creditHref: string;
  creditLabel: string;
}) {
  const [active, setActive] = useState(false);
  return (
    <figure className="flex h-full flex-col">
      <div className="flex items-center justify-between gap-4 border-b border-line px-6 py-2.5">
        <figcaption className="hud-label truncate text-muted">{example.label}</figcaption>
        <p className="hud-label shrink-0 text-faint">{index}</p>
      </div>
      <div
        className="relative w-full overflow-hidden bg-bg-sunken"
        style={{ aspectRatio: example.wide ? "92 / 39" : "16 / 9" }}
      >
        {active ? (
          <video
            src={example.src}
            poster={example.poster}
            controls
            autoPlay
            playsInline
            className="absolute inset-0 h-full w-full"
          />
        ) : (
          <button
            type="button"
            onClick={() => setActive(true)}
            aria-label={`Play example: ${example.label}`}
            className="group absolute inset-0 h-full w-full"
          >
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img
              src={example.poster}
              alt=""
              loading="lazy"
              className="absolute inset-0 h-full w-full object-cover"
            />
            <span className="absolute inset-0 bg-black/20 transition-colors group-hover:bg-black/10" />
            <span className="hud-label absolute bottom-3 left-3 flex items-center gap-1.5 bg-invert-bg px-3 py-1.5 font-bold text-invert-fg">
              <PlayIcon aria-hidden className="h-3.5 w-3.5" />
              Play with sound
            </span>
          </button>
        )}
      </div>
      <a
        href={creditHref}
        target="_blank"
        rel="noreferrer"
        className="hud-label mt-auto border-t border-line px-6 py-2.5 text-faint hover:text-ink"
      >
        {creditLabel}
      </a>
    </figure>
  );
}
