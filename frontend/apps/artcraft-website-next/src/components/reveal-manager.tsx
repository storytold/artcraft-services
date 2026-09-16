"use client";

import { useEffect } from "react";
import gsap from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import { introClock, introTuner, onIntroReplay } from "@/lib/intro";

gsap.registerPlugin(ScrollTrigger);

const HIDDEN = { autoAlpha: 0, y: 28, filter: "blur(8px)" } as const;
// clearProps on settle: a lingering `filter: blur(0px)` keeps the element on a
// composited layer, which disables subpixel text antialiasing on Windows.
const SHOWN = {
  autoAlpha: 1,
  y: 0,
  filter: "blur(0px)",
  duration: 0.9,
  ease: "power3.out",
  overwrite: true,
  clearProps: "all",
} as const;

// Progressive-enhancement scroll reveals for `[data-reveal]` elements.
//
// Elements render fully visible in the server HTML (crawlers and no-JS
// visitors see everything). Only once JS runs — and only for visitors without
// a reduced-motion preference — do we hide them and reveal on scroll.
// `data-reveal-group` on a container staggers its `[data-reveal]` children.
export default function RevealManager() {
  useEffect(() => {
    const mm = gsap.matchMedia();

    mm.add("(prefers-reduced-motion: no-preference)", () => {
      // Structural draw-up, one-shot like everything else: as a section
      // first enters, its top rule draws across, the eyebrow settles in,
      // and the corner ticks pop — then the frame is built and stays
      // built. (An earlier scrub-linked version un-drew on scroll-back;
      // it read as inconsistent against the one-shot content.)
      gsap.utils.toArray<HTMLElement>("[data-choreo]").forEach((sec) => {
        const rule = sec.querySelector<HTMLElement>("[data-draw-rule]");
        const ticks = sec.querySelectorAll<HTMLElement>("[data-draw-tick]");
        const eyebrow = sec.querySelector<HTMLElement>("[data-draw-eyebrow]");
        if (rule) gsap.set(rule, { scaleX: 0 });
        if (ticks.length) gsap.set(ticks, { scale: 0, opacity: 0 });
        if (eyebrow) gsap.set(eyebrow, HIDDEN);
        ScrollTrigger.create({
          trigger: sec,
          start: "top 88%",
          once: true,
          onEnter: () => {
            const tl = gsap.timeline();
            if (rule) {
              tl.to(
                rule,
                { scaleX: 1, duration: 0.7, ease: "power2.out", clearProps: "all" },
                0,
              );
            }
            if (eyebrow) tl.to(eyebrow, { ...SHOWN }, 0.15);
            if (ticks.length) {
              tl.to(
                ticks,
                {
                  scale: 1,
                  opacity: 1,
                  duration: 0.35,
                  ease: "power3.out",
                  stagger: 0.06,
                  clearProps: "all",
                },
                0.35,
              );
            }
          },
        });
      });

      gsap.utils
        .toArray<HTMLElement>(
          "[data-reveal]:not([data-reveal-group] [data-reveal])",
        )
        .forEach((el) => {
          gsap.set(el, HIDDEN);
          ScrollTrigger.create({
            trigger: el,
            start: "top 85%",
            once: true,
            onEnter: () => gsap.to(el, SHOWN),
          });
        });

      const introCleanups: (() => void)[] = [];
      gsap.utils.toArray<HTMLElement>("[data-reveal-group]").forEach((group) => {
        const children = Array.from(
          group.querySelectorAll<HTMLElement>("[data-reveal]"),
        );
        if (!children.length) return;
        gsap.set(children, HIDDEN);
        // Hero groups play on the master intro's copy beat instead of a
        // scroll trigger — they're above the fold on load, and firing
        // immediately would land the pitch before the brand has formed.
        // The tuner's replay re-hides and re-arms them.
        if (group.closest("#hero")) {
          let wait: gsap.TickerCallback | null = null;
          const arm = () => {
            if (wait) gsap.ticker.remove(wait);
            gsap.set(children, HIDDEN);
            const cb: gsap.TickerCallback = () => {
              if (introClock.t < introTuner.read().copyAt) return;
              gsap.to(children, { ...SHOWN, stagger: 0.08 });
              gsap.ticker.remove(cb);
              if (wait === cb) wait = null;
            };
            wait = cb;
            gsap.ticker.add(cb);
          };
          arm();
          introCleanups.push(onIntroReplay(arm));
          introCleanups.push(() => {
            if (wait) gsap.ticker.remove(wait);
          });
          return;
        }
        // Per-CHILD triggers, not one for the whole group: a tall grid's
        // single trigger revealed every cell the moment the grid's top
        // entered, so everything below the fold animated unseen and the
        // section read as static against the drawn frames. Each cell now
        // assembles as it arrives; cells sharing a row cascade left to
        // right (delay from their horizontal position), which keeps the
        // old stagger feel without a group-wide clock.
        const groupRect = group.getBoundingClientRect();
        children.forEach((child) => {
          const delay =
            groupRect.width > 0
              ? ((child.getBoundingClientRect().left - groupRect.left) /
                  groupRect.width) *
                0.18
              : 0;
          ScrollTrigger.create({
            trigger: child,
            start: "top 88%",
            once: true,
            onEnter: () => gsap.to(child, { ...SHOWN, delay }),
          });
        });
      });

      return () => {
        introCleanups.forEach((fn) => fn());
      };
    });

    return () => mm.revert();
  }, []);

  return null;
}
