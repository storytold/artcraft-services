"use client";

import { useMemo, useState } from "react";
import Link from "next/link";
import { PlayIcon } from "lucide-react";
import { Badge, TabSelector } from "@/components/ui";
import { mediaUrl } from "@/lib/links";

// Serializable subset of TutorialItem the server page hands down.
export type TutorialCard = {
  slug: string;
  title: string;
  category?: string;
  thumbnail?: string;
};

const ALL = "All";
const FALLBACK_THUMBNAIL = "/images/tutorial-thumbnails/2D_Editor_Basics.jpg";

// Category filter + card grid. Filtering is the only client state on the
// tutorials page, so it is isolated here and the index stays a server page.
export default function TutorialGrid({ items }: { items: TutorialCard[] }) {
  const [category, setCategory] = useState(ALL);

  const tabs = useMemo(() => {
    const categories = new Set<string>();
    for (const item of items) if (item.category) categories.add(item.category);
    return [ALL, ...categories].map((c) => ({ id: c, label: c }));
  }, [items]);

  const visible =
    category === ALL ? items : items.filter((t) => t.category === category);

  return (
    <>
      <div className="flex flex-wrap items-center justify-between gap-4 border-b border-line px-6 py-4 md:px-10">
        <TabSelector tabs={tabs} activeTab={category} onTabChange={setCategory} />
        <p className="hud-label hidden text-faint sm:block">
          {visible.length} {visible.length === 1 ? "video" : "videos"}
        </p>
      </div>

      <ul className="grid gap-px bg-line md:grid-cols-2 lg:grid-cols-3">
        {visible.map((item, i) => (
          <li key={item.slug} className="bg-bg">
            <Link
              href={`/tutorials/${item.slug}`}
              className="group block h-full"
            >
              <div className="flex items-center justify-between gap-4 border-b border-line px-6 py-2.5">
                <p className="hud-label text-muted">
                  {item.category ?? "Tutorial"}
                </p>
                <p className="hud-label text-faint">
                  {String(i + 1).padStart(2, "0")}
                </p>
              </div>
              <div className="relative aspect-video w-full overflow-hidden bg-bg-sunken">
                {/* eslint-disable-next-line @next/next/no-img-element */}
                <img
                  src={mediaUrl(item.thumbnail ?? FALLBACK_THUMBNAIL)}
                  alt=""
                  loading="lazy"
                  className="absolute inset-0 h-full w-full object-cover transition-transform duration-500 group-hover:scale-[1.02]"
                />
                <span className="hud-label absolute bottom-3 left-3 flex items-center gap-1.5 bg-invert-bg px-3 py-1.5 font-bold text-invert-fg">
                  <PlayIcon aria-hidden className="h-3.5 w-3.5" />
                  Watch
                </span>
              </div>
              <div className="flex items-start justify-between gap-3 px-6 py-5">
                <h2 className="font-display text-xl font-medium tracking-[-0.02em] text-ink-strong">
                  {item.title}
                </h2>
                {item.category && <Badge label={item.category} />}
              </div>
            </Link>
          </li>
        ))}
      </ul>
    </>
  );
}
