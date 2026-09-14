"use client";

import { useEffect, useRef, useState, type CSSProperties } from "react";
import gsap from "gsap";
import { AppleIcon, MonitorIcon } from "lucide-react";
import { WEBAPP_URL } from "@/lib/links";
import { Button } from "@/components/ui";
import { useTunerStore } from "@/lib/tuner";
import { introClock, introTuner } from "@/lib/intro";
import { slateTuner } from "./hero-slate";

// The hero's sales copy as a living type specimen. Fluid poster sizing —
// the headline scales with the stage so its ratio to the wordmark is a
// designed constant, not a fixed desktop size marooned on ultrawides —
// plus two restrained variable-axis behaviors on the display letters:
// an intro settle (letters arrive at the wordmark's poster width, wdth
// 125, and cascade down to the display setting on the copy beat) and a
// cursor field afterwards (letters bulge back toward poster width under
// the pointer with a gaussian falloff — the same "mouse has weight" idea
// as the galaxy's warp, spoken in typography).

const REST_WDTH = 118;
const REST_WGHT = 620;
const WIDE_WDTH = 125; // Archivo's width-axis ceiling — the wordmark's poster extreme.
const WIDE_WGHT = 700;
const HEADLINE = "Controllable AI";

function easeOutCubic(p: number): number {
  return 1 - Math.pow(1 - p, 3);
}

export default function HeroCopy() {
  const lettersRef = useRef<(HTMLSpanElement | null)[]>([]);
  const h1Ref = useRef<HTMLHeadingElement>(null);
  // SSR-safe tuner subscription: server renders defaults, the client
  // re-renders (debounced) when the fluid-size knobs move.
  const [, setTv] = useState(-1);
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | null = null;
    const unsub = useTunerStore.subscribe((s) => {
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => setTv(s.version), 150);
    });
    return () => {
      unsub();
      if (timer) clearTimeout(timer);
    };
  }, []);

  useEffect(() => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const letters = lettersRef.current.filter(Boolean) as HTMLSpanElement[];
    const h1 = h1Ref.current;
    if (!letters.length || !h1) return;

    const cur = letters.map(() => ({ wdth: WIDE_WDTH, wght: WIDE_WGHT }));
    const pointer = { x: 0, y: 0, inside: false };
    const onMove = (e: PointerEvent) => {
      pointer.x = e.clientX;
      pointer.y = e.clientY;
      pointer.inside = true;
    };
    const onLeave = () => {
      pointer.inside = false;
    };
    h1.addEventListener("pointermove", onMove);
    h1.addEventListener("pointerleave", onLeave);

    let idle = false;
    const tick = (_t: number, deltaMs: number) => {
      const sv = slateTuner.read();
      const copyAt = introTuner.read().copyAt;
      const dt = Math.min(deltaMs / 1000, 0.1);
      const k = 1 - Math.exp(-dt / 0.09);

      // Read phase: letter centers, before any style writes (one layout).
      const rects = pointer.inside ? letters.map((l) => l.getBoundingClientRect()) : null;

      let settled = true;
      for (let i = 0; i < letters.length; i++) {
        // Intro: each letter settles from the poster extreme on its own
        // lagged window. Clock-driven, so the tuner replay rewinds it.
        const p = Math.min(
          1,
          Math.max(0, (introClock.t - copyAt - i * sv.settleLag) / sv.settleDur),
        );
        const e = easeOutCubic(p);
        let wdthT = WIDE_WDTH + (REST_WDTH - WIDE_WDTH) * e;
        const wghtT = WIDE_WGHT + (REST_WGHT - WIDE_WGHT) * e;

        // Cursor field, only once the letter has fully settled.
        if (p >= 1 && pointer.inside && rects) {
          const r = rects[i];
          const dx = pointer.x - (r.left + r.width / 2);
          const dy = pointer.y - (r.top + r.height / 2);
          const g = Math.exp(-(dx * dx + dy * dy) / (2 * sv.hoverSigma * sv.hoverSigma));
          wdthT = Math.min(WIDE_WDTH, wdthT + sv.hoverAmp * g);
        }

        const c = cur[i];
        c.wdth += (wdthT - c.wdth) * k;
        c.wght += (wghtT - c.wght) * k;
        if (Math.abs(c.wdth - wdthT) > 0.05 || Math.abs(c.wght - wghtT) > 0.25)
          settled = false;
        if (p < 1 || !settled || pointer.inside || !idle) {
          letters[i].style.fontVariationSettings = `"wght" ${c.wght.toFixed(1)}, "wdth" ${c.wdth.toFixed(2)}`;
        }
      }
      // Sleep flag: when everything rests and the pointer is gone, stop
      // touching styles (the ticker itself stays cheap).
      idle = settled && !pointer.inside && introClock.t > copyAt;
    };
    gsap.ticker.add(tick);
    return () => {
      gsap.ticker.remove(tick);
      h1.removeEventListener("pointermove", onMove);
      h1.removeEventListener("pointerleave", onLeave);
    };
  }, []);

  const sv = slateTuner.read();
  // The fluid sizes ride CSS variables on a non-animated `contents` wrapper:
  // RevealManager finishes its reveals with clearProps:"all", which would
  // wipe inline font-size set directly on the [data-reveal] elements.
  const sizes = {
    "--head-size": `clamp(2.125rem, calc(1.1rem + ${sv.headSlope}vw), ${sv.headMax}rem)`,
    "--sub-size": `clamp(1.0625rem, calc(0.92rem + ${(sv.headSlope * 0.24).toFixed(2)}vw), ${(sv.headMax * 0.31).toFixed(2)}rem)`,
  } as CSSProperties;

  let li = 0;
  return (
    <div className="contents" style={sizes}>
      <h1
        ref={h1Ref}
        data-reveal
        className="pointer-events-auto relative mt-8 cursor-default font-display leading-[1.05] tracking-[-0.03em] text-ink-strong [font-size:var(--head-size)]"
      >
        {HEADLINE.split(" ").map((word, wi) => (
          <span key={wi} className="inline-block whitespace-nowrap">
            {wi > 0 && <span className="inline-block">&nbsp;</span>}
            {[...word].map((ch) => {
              const i = li++;
              return (
                <span
                  key={i}
                  ref={(el) => {
                    lettersRef.current[i] = el;
                  }}
                  className="inline-block"
                  style={{
                    fontVariationSettings: `"wght" ${REST_WGHT}, "wdth" ${REST_WDTH}`,
                  }}
                >
                  {ch}
                </span>
              );
            })}
          </span>
        ))}{" "}
        <span className="whitespace-nowrap font-serif italic font-normal text-muted">
          for artists.
        </span>
      </h1>

      <p
        data-reveal
        className="relative mt-[1.1em] max-w-[44ch] leading-relaxed text-muted [font-size:var(--sub-size)]"
      >
        Artists need and deserve unparalleled control and precision.
        ArtCraft&rsquo;s got you covered — compose in real 3D, then render
        with AI.
      </p>

      <div
        data-reveal
        className="pointer-events-auto relative mt-7 flex flex-wrap items-center justify-center gap-3"
      >
        <Button href="/download" size="lg">
          <AppleIcon aria-hidden className="h-4 w-4" />
          <MonitorIcon aria-hidden className="h-4 w-4" />
          Download free
        </Button>
        <Button href={WEBAPP_URL} variant="secondary" size="lg">
          Use on web
        </Button>
      </div>
    </div>
  );
}
