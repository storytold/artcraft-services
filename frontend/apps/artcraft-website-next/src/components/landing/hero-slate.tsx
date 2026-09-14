"use client";

import { useEffect, useRef, type CSSProperties } from "react";
import gsap from "gsap";
import { AppleIcon, MonitorIcon } from "lucide-react";
import { SOCIAL_LINKS } from "@/lib/links";
import { GitHubIcon } from "@/components/icons";
import { defineTunables } from "@/lib/tuner";
import { introClock, introTuner } from "@/lib/intro";
import { heroTelemetry } from "@/lib/hero-telemetry";

// The hero's viewfinder slate: the sales layer annotated in the product's
// own metaphor — a camera viewport where shots are composed. Corner
// brackets frame the stage like framing guides, a timecode readout runs on
// the galaxy's scene clock, and the flanks carry live telemetry from the
// running instrument. All of it is width-adaptive chrome: on wide stages
// the brackets and flanks move OUT with the viewport, so excess negative
// space becomes measured, annotated space instead of margin.

export const slateTuner = defineTunables("heroSlate", "Hero slate", {
  bracketInset: {
    label: "Bracket inset px",
    min: 8,
    max: 96,
    step: 1,
    default: 22,
    info: "Distance of the corner framing brackets from the poster's edges.",
  },
  bracketArm: {
    label: "Bracket arm px",
    min: 10,
    max: 80,
    step: 1,
    default: 28,
    info: "Length of each bracket's two arms.",
  },
  bracketBottom: {
    label: "Bracket bottom px",
    min: 20,
    max: 160,
    step: 2,
    default: 76,
    info: "Extra bottom inset so the lower brackets clear the proof strip.",
  },
  flankInset: {
    label: "Flank inset px",
    min: 16,
    max: 220,
    step: 2,
    default: 56,
    info: "Distance of the telemetry stacks from the stage's left/right edges.",
  },
  flankHz: {
    label: "Flank refresh Hz",
    min: 1,
    max: 30,
    step: 1,
    default: 6,
    info: "How often the flank readouts sample the galaxy's telemetry.",
  },
  tcFps: {
    label: "Timecode fps",
    min: 12,
    max: 60,
    step: 1,
    default: 24,
    info: "Frame rate of the slate timecode readout (frames digit).",
  },
  hoverSigma: {
    label: "Hover falloff px",
    min: 20,
    max: 300,
    step: 5,
    default: 90,
    info: "Gaussian falloff radius of the headline's cursor width-bulge — the letter field's reach.",
  },
  hoverAmp: {
    label: "Hover width +",
    min: 0,
    max: 7,
    step: 0.5,
    default: 6,
    info: "How many width-axis units a letter gains directly under the cursor (Archivo caps at wdth 125).",
  },
  settleLag: {
    label: "Settle lag s",
    min: 0,
    max: 0.15,
    step: 0.005,
    default: 0.035,
    info: "Per-letter delay of the intro width-settle cascade across the headline.",
  },
  settleDur: {
    label: "Settle dur s",
    min: 0.1,
    max: 2,
    step: 0.05,
    default: 0.6,
    info: "How long each headline letter takes to settle from the wordmark's poster width down to display width.",
  },
  headMax: {
    label: "Headline max rem",
    min: 2.5,
    max: 7,
    step: 0.125,
    default: 4.5,
    info: "Ceiling of the fluid headline size — reached on wide stages.",
  },
  headSlope: {
    label: "Headline vw slope",
    min: 0.5,
    max: 4,
    step: 0.1,
    default: 2.1,
    info: "How fast the headline grows with viewport width (the vw term of its clamp).",
  },
});

// Eases slate chrome in on an intro beat, clock-driven so the tuner's
// replay button rewinds it for free. Returns the current 0..1 opacity.
function introFade(at: number, dur = 0.8): number {
  return Math.min(1, Math.max(0, (introClock.t - at) / dur));
}

function pad2(n: number): string {
  return String(Math.floor(n)).padStart(2, "0");
}

function formatTimecode(t: number, fps: number): string {
  const f = Math.floor((t % 1) * fps);
  const s = Math.floor(t) % 60;
  const m = Math.floor(t / 60) % 60;
  const h = Math.floor(t / 3600);
  return `${pad2(h)}:${pad2(m)}:${pad2(s)}:${pad2(f)}`;
}

// Corner framing brackets + the REC/timecode readout. Chrome only — no
// pointer events, hidden below lg where the stage is too small to frame.
export function SlateFrame() {
  const rootRef = useRef<HTMLDivElement>(null);
  const tcRef = useRef<HTMLSpanElement>(null);

  useEffect(() => {
    const root = rootRef.current;
    if (!root) return;
    let lastTc = "";
    let acc = 0;
    const tick = (_t: number, deltaMs: number) => {
      const sv = slateTuner.read();
      // A dead instrument gets no slate: hold the chrome invisible until
      // the galaxy's clock actually ticks (reduced motion / WebGL failure).
      const live = heroTelemetry.time > 0 ? 1 : 0;
      root.style.opacity = String(introFade(introTuner.read().instrAt) * live);
      // The right edge clears the scroll ruler's rail when it's active.
      const railClear =
        document.documentElement.getAttribute("data-ruler") === "on" ? 48 : 0;
      root.style.inset = `${sv.bracketInset}px`;
      root.style.right = `${sv.bracketInset + railClear}px`;
      root.style.bottom = `${sv.bracketBottom}px`;
      root.style.setProperty("--slate-arm", `${sv.bracketArm}px`);
      acc += deltaMs / 1000;
      if (acc < 1 / sv.tcFps) return;
      acc = 0;
      const tc = formatTimecode(heroTelemetry.time, sv.tcFps);
      if (tc !== lastTc && tcRef.current) {
        tcRef.current.textContent = tc;
        lastTc = tc;
      }
    };
    gsap.ticker.add(tick);
    return () => gsap.ticker.remove(tick);
  }, []);

  const arm = "var(--slate-arm)";
  const corner = "absolute border-line-strong opacity-60";
  return (
    <div
      ref={rootRef}
      aria-hidden
      className="pointer-events-none absolute z-30 hidden lg:block"
      style={{ opacity: 0, "--slate-arm": "28px" } as CSSProperties}
    >
      <span
        className={`${corner} left-0 top-0 border-l border-t`}
        style={{ width: arm, height: arm }}
      />
      <span
        className={`${corner} right-0 top-0 border-r border-t`}
        style={{ width: arm, height: arm }}
      />
      <span
        className={`${corner} bottom-0 left-0 border-b border-l`}
        style={{ width: arm, height: arm }}
      />
      <span
        className={`${corner} bottom-0 right-0 border-b border-r`}
        style={{ width: arm, height: arm }}
      />

      {/* Slate line: REC dot + scene + timecode, riding the top-left bracket. */}
      <p className="hud-label absolute left-6 top-0 flex -translate-y-1/2 items-center gap-2 text-faint">
        <span className="relative flex h-1.5 w-1.5">
          <span className="absolute inline-flex h-full w-full animate-ping bg-accent opacity-60" />
          <span className="relative inline-flex h-1.5 w-1.5 bg-accent" />
        </span>
        SC 01
        <span aria-hidden>·</span>
        TC <span ref={tcRef} className="tabular-nums">00:00:00:00</span>
      </p>
    </div>
  );
}

// One flank row: mono label left, live value right (written imperatively).
function Row({
  label,
  refFn,
  initial,
}: {
  label: string;
  refFn: (el: HTMLSpanElement | null) => void;
  initial: string;
}) {
  return (
    <p className="flex items-baseline justify-between gap-6">
      <span className="text-faint">{label}</span>
      <span ref={refFn} className="tabular-nums text-muted">
        {initial}
      </span>
    </p>
  );
}

// Live telemetry stacks at the stage's far flanks — honest readouts from
// the running galaxy, in the slate voice. The wider the stage, the further
// out they sit: annotated space instead of empty margin. Hidden below xl.
export function FlankTelemetry() {
  const leftRef = useRef<HTMLDivElement>(null);
  const rightRef = useRef<HTMLDivElement>(null);
  const vals = useRef<Record<string, HTMLSpanElement | null>>({});

  useEffect(() => {
    let acc = 1;
    const set = (key: string, text: string) => {
      const el = vals.current[key];
      if (el && el.textContent !== text) el.textContent = text;
    };
    const tick = (_t: number, deltaMs: number) => {
      const sv = slateTuner.read();
      const live = heroTelemetry.time > 0 ? 1 : 0;
      const fade = introFade(introTuner.read().instrAt + 0.2) * live;
      for (const el of [leftRef.current, rightRef.current]) {
        if (!el) continue;
        el.style.opacity = String(fade);
        el.style.setProperty("--flank-inset", `${sv.flankInset}px`);
      }
      acc += deltaMs / 1000;
      if (acc < 1 / sv.flankHz) return;
      acc = 0;
      const t = heroTelemetry;
      set("cards", String(t.cards).padStart(3, "0"));
      set("clips", `${t.clipsLive}/${t.clipPool}`);
      set("fps", String(Math.round(t.fps)).padStart(3, "0"));
      set("spin", `${t.spinDeg < 0 ? "−" : "+"}${pad2(Math.abs(t.spinDeg) % 360)}°`);
      set("lens", t.boost ? "3.0×" : "1.0×");
    };
    gsap.ticker.add(tick);
    return () => gsap.ticker.remove(tick);
  }, []);

  // Frosted chip under the readouts — bare mono over bright/dark footage
  // was illegible; a viewfinder HUD sits on its own translucent plate.
  const stack =
    "hud-label pointer-events-none absolute top-1/2 z-30 hidden w-40 -translate-y-1/2 flex-col gap-2 border-y border-line bg-bg/60 px-3 py-3 backdrop-blur-sm xl:flex";
  const r = (k: string) => (el: HTMLSpanElement | null) => {
    vals.current[k] = el;
  };
  return (
    <>
      <div
        ref={leftRef}
        aria-hidden
        className={stack}
        style={{ opacity: 0, left: "var(--flank-inset, 56px)" }}
      >
        <Row label="Cards" refFn={r("cards")} initial="000" />
        <Row label="Playing" refFn={r("clips")} initial="0/0" />
        <Row label="FPS" refFn={r("fps")} initial="000" />
      </div>
      <div
        ref={rightRef}
        aria-hidden
        className={stack}
        style={{ opacity: 0, right: "var(--flank-inset, 56px)" }}
      >
        <Row label="Spin θ" refFn={r("spin")} initial="+00°" />
        <Row label="Lens" refFn={r("lens")} initial="1.0×" />
        <Row label="Scene" refFn={() => {}} initial="01" />
      </div>
    </>
  );
}

// The proof line as a full-width ruled strip at the poster's bottom edge —
// the composition's horizontal anchor. Spreads to the stage's width, so
// wide screens get a real baseline instead of a floating centered cluster.
export function ProofStrip() {
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const root = rootRef.current;
    if (!root) return;
    const tick = () => {
      root.style.opacity = String(introFade(introTuner.read().copyAt + 0.25));
    };
    gsap.ticker.add(tick);
    return () => gsap.ticker.remove(tick);
  }, []);

  return (
    // The right padding steps up under an active scroll ruler so the strip's
    // last item never sits beneath the rail's section queue (bottom-right).
    <div
      ref={rootRef}
      className="pointer-events-auto relative z-30 flex flex-wrap items-center justify-center gap-x-8 gap-y-2 border-t border-line px-6 py-3.5 md:justify-between md:px-10 [html[data-ruler=on]_&]:md:pr-64"
      style={{ opacity: 0 }}
    >
      <a
        href={SOCIAL_LINKS.GITHUB}
        target="_blank"
        rel="noopener noreferrer"
        className="hud-label flex items-center gap-1.5 text-faint hover:text-ink"
      >
        <GitHubIcon className="h-3.5 w-3.5" />
        Open source on GitHub
      </a>
      <p className="hud-label flex items-center gap-2 text-faint">
        <AppleIcon aria-hidden className="h-3.5 w-3.5" />
        <MonitorIcon aria-hidden className="h-3.5 w-3.5" />
        macOS · Windows · Web
      </p>
      <p className="hud-label text-faint">No subscription required</p>
    </div>
  );
}
