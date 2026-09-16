import { Link } from "react-router-dom";
import {
  ArrowRightIcon,
  BoxIcon,
  FilmIcon,
  GlobeIcon,
  GroupIcon,
  ImageIcon,
  ImagesIcon,
  MusicIcon,
  VideoIcon,
  WandSparklesIcon,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { DynamicIcon } from "@storyteller/icons";
import Seo from "../../components/seo";
import { Reveal, RevealGroup } from "../../components/motion/reveal";

type AppCard = {
  label: string;
  description: string;
  href: string;
  icon: LucideIcon;
  iconTile: string;
  hoverStyle: string;
  badge?: string;
};

const APPS: AppCard[] = [
  {
    label: "Image",
    description: "Generate images from prompts and references.",
    href: "/create-image",
    icon: ImageIcon,
    iconTile: "border-blue-400/40 bg-blue-500/20 text-blue-300",
    hoverStyle: "hover:border-blue-400/60 hover:bg-blue-500/10",
  },
  {
    label: "Video",
    description: "Generate cinematic clips from text or images.",
    href: "/create-video",
    icon: VideoIcon,
    iconTile: "border-purple-400/40 bg-purple-500/20 text-purple-300",
    hoverStyle: "hover:border-purple-400/60 hover:bg-purple-500/10",
  },
  {
    label: "Audio",
    description: "Generate songs and sound effects from prompts.",
    href: "/create-audio",
    icon: MusicIcon,
    iconTile: "border-pink-400/40 bg-pink-500/20 text-pink-300",
    hoverStyle: "hover:border-pink-400/60 hover:bg-pink-500/10",
  },
  {
    label: "3D Object",
    description: "Turn a prompt or image into a 3D model.",
    href: "/create-object",
    icon: BoxIcon,
    iconTile: "border-cyan-400/40 bg-cyan-500/20 text-cyan-300",
    hoverStyle: "hover:border-cyan-400/60 hover:bg-cyan-500/10",
  },
  {
    label: "3D World",
    description: "Turn a prompt or image into an explorable world.",
    href: "/create-world",
    icon: GlobeIcon,
    iconTile: "border-teal-400/40 bg-teal-500/20 text-teal-300",
    hoverStyle: "hover:border-teal-400/60 hover:bg-teal-500/10",
  },
  {
    label: "Edit 3D",
    description: "Compose 3D scenes and render with AI cameras.",
    href: "/edit-3d",
    icon: BoxIcon,
    iconTile: "border-amber-400/40 bg-amber-500/20 text-amber-300",
    hoverStyle: "hover:border-amber-400/60 hover:bg-amber-500/10",
  },
  {
    label: "Background Change",
    description: "Swap or remove backgrounds with AI VFX.",
    href: "/background-change",
    icon: WandSparklesIcon,
    iconTile: "border-emerald-400/40 bg-emerald-500/20 text-emerald-300",
    hoverStyle: "hover:border-emerald-400/60 hover:bg-emerald-500/10",
  },
  {
    label: "Edit Video",
    description: "Trim, arrange, and edit clips on a timeline.",
    href: "/video-editor",
    icon: FilmIcon,
    iconTile: "border-rose-400/40 bg-rose-500/20 text-rose-300",
    hoverStyle: "hover:border-rose-400/60 hover:bg-rose-500/10",
    badge: "BETA",
  },
  {
    label: "Moodboard",
    description: "Collect references and ideas to steer a generation.",
    href: "/moodboard",
    icon: GroupIcon,
    iconTile: "border-indigo-400/40 bg-indigo-500/20 text-indigo-300",
    hoverStyle: "hover:border-indigo-400/60 hover:bg-indigo-500/10",
    badge: "BETA",
  },
  {
    label: "Frame Extractor",
    description: "Grab a still frame from any video.",
    href: "/frame-extractor",
    icon: ImagesIcon,
    iconTile: "border-orange-400/40 bg-orange-500/20 text-orange-300",
    hoverStyle: "hover:border-orange-400/60 hover:bg-orange-500/10",
    badge: "NEW",
  },
];

export function Home() {
  return (
    <div className="min-h-full px-5 py-8 sm:px-8 sm:py-12 max-w-6xl mx-auto w-full">
      <Seo
        title="ArtCraft - Create AI Images and Video"
        description="Generate AI images and video with ArtCraft."
      />
      <p className="hud-label mb-4 text-ui-accent-ink">Your creative workspace</p>
      <Reveal
        as="h1"
        inView={false}
        y={20}
        className="max-w-3xl font-display text-3xl leading-tight sm:text-5xl tracking-tight"
      >
        What will you <span className="text-ui-accent-ink">craft</span> today?
      </Reveal>

      <p className="mt-4 max-w-xl text-sm leading-relaxed text-base-fg/70 sm:text-base">
        Start with an idea. Choose a tool to bring it to life.
      </p>

      <section className="mt-8 border-t border-ui-border pt-6 sm:mt-10">
        <Reveal
          as="h2"
          inView={false}
          delay={0.08}
          className="hud-label text-base-fg/70 mb-4"
        >
          Create
        </Reveal>
        <RevealGroup
          inView={false}
          delayChildren={0.12}
          stagger={0.06}
          className="grid gap-3 auto-rows-fr md:grid-cols-2 xl:grid-cols-3"
        >
          {APPS.map((app) => (
            <Reveal key={app.href} y={20}>
              <Link
                to={app.href}
                className={`group relative flex h-full rounded-[3px] border border-ui-border bg-white/5 p-5 transition-colors duration-150 focus-visible:border-primary ${app.hoverStyle}`}
              >
                <div className="relative flex w-full items-start gap-4">
                  <div
                    className={`flex h-10 w-10 shrink-0 items-center justify-center border ${app.iconTile}`}
                  >
                    <DynamicIcon icon={app.icon} className="text-base" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center justify-between gap-2">
                      <div className="flex flex-wrap items-center gap-2 min-w-0">
                        <h3 className="text-base font-bold uppercase tracking-tight text-ui-ink">
                          {app.label}
                        </h3>
                        {app.badge && (
                          <span
                            className="shrink-0 border border-primary/30 px-1.5 py-1 font-mono text-[9px] font-medium tracking-wider leading-none text-ui-accent-ink"
                          >
                            {app.badge}
                          </span>
                        )}
                      </div>
                      <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-[3px] border border-white/15 text-white/40 transition-colors duration-150 group-hover:border-white/40 group-hover:text-white">
                        <ArrowRightIcon className="text-xs" />
                      </span>
                    </div>
                    <p className="mt-2 text-sm text-base-fg/70 leading-relaxed">
                      {app.description}
                    </p>
                  </div>
                </div>
              </Link>
            </Reveal>
          ))}
        </RevealGroup>
      </section>
    </div>
  );
}

export default Home;
