import { twMerge } from "tailwind-merge";
import { Button, CopyButton } from "@/components/ui";
import { trackAttrs } from "@/lib/analytics";

// Share strip: copy-link plus intent links for the networks our audience
// posts to. Hairline cells (gap-px over the line color): one row from sm up,
// a two-column grid below so every row is full and no line color shows
// through. Intent URLs need no SDKs or tracking scripts.
const TARGETS: { name: string; href: (url: string, text: string) => string }[] = [
  {
    name: "X",
    href: (url, text) =>
      `https://x.com/intent/post?text=${encodeURIComponent(text)}&url=${encodeURIComponent(url)}`,
  },
  {
    name: "Reddit",
    href: (url, text) =>
      `https://www.reddit.com/submit?url=${encodeURIComponent(url)}&title=${encodeURIComponent(text)}`,
  },
  {
    name: "Bluesky",
    href: (url, text) =>
      `https://bsky.app/intent/compose?text=${encodeURIComponent(`${text} ${url}`)}`,
  },
  {
    name: "LinkedIn",
    href: (url) =>
      `https://www.linkedin.com/sharing/share-offsite/?url=${encodeURIComponent(url)}`,
  },
];

// Copy and network cells share one ghost-button treatment.
const CELL_CLASSES = "h-9 w-full bg-bg px-3 sm:w-fit";

export default function ShareBar({
  url,
  text,
  className,
}: {
  /** Absolute URL to share. */
  url: string;
  text: string;
  className?: string;
}) {
  const shareParams = { content_type: "page", item_id: new URL(url).pathname };
  return (
    <div
      className={twMerge(
        "grid grid-cols-2 gap-px border border-line bg-line sm:flex sm:w-fit",
        className,
      )}
    >
      <span className="hud-label flex h-9 items-center justify-center bg-bg px-3 text-faint">
        Share
      </span>
      <CopyButton
        value={url}
        label="Copy link"
        copiedLabel="Link copied"
        variant="ghost"
        className={CELL_CLASSES}
        {...trackAttrs("share", { method: "copy_link", ...shareParams })}
      />
      {TARGETS.map((target) => (
        <Button
          key={target.name}
          href={target.href(url, text)}
          target="_blank"
          rel="noopener noreferrer"
          aria-label={`Share on ${target.name}`}
          variant="ghost"
          className={CELL_CLASSES}
          {...trackAttrs("share", { method: target.name.toLowerCase(), ...shareParams })}
        >
          {target.name}
        </Button>
      ))}
    </div>
  );
}
