"use client";

import { useEffect, useRef } from "react";
import gsap from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import { MANIFESTO_EMPHASIS, MANIFESTO_WORDS } from "@/lib/campaign-data";

gsap.registerPlugin(ScrollTrigger);

// Pinned manifesto: the sentence reveals word by word as the visitor
// scrolls, then the accent block sweeps in under the emphasized word and
// the section holds a beat before releasing. Scrub-linked and reversible.
// Reduced motion / small screens get the static sentence. (The Vite page
// also ran a Three.js character and a scroll-scrubbed video here; those
// were not carried over.)
export default function Manifesto() {
  const root = useRef<HTMLElement>(null);

  useEffect(() => {
    const mm = gsap.matchMedia();
    mm.add(
      "(prefers-reduced-motion: no-preference) and (min-width: 768px)",
      () => {
        const section = root.current;
        if (!section) return;
        const words = section.querySelectorAll<HTMLElement>("[data-word]");
        const sweep = section.querySelector<HTMLElement>("[data-sweep]");
        gsap.set(words, { opacity: 0.15, y: 6 });
        if (sweep) gsap.set(sweep, { scaleX: 0 });

        const tl = gsap.timeline({
          scrollTrigger: {
            trigger: section,
            start: "top top",
            end: "bottom bottom",
            scrub: 0.2,
          },
        });
        words.forEach((word, i) => {
          tl.to(word, { opacity: 1, y: 0, duration: 0.7, ease: "power2.out" }, i * 0.5);
        });
        if (sweep) {
          tl.to(sweep, { scaleX: 1, duration: 4, ease: "power2.out" }, ">");
        }
        tl.to({}, { duration: 4 }); // hold
      },
    );
    return () => mm.revert();
  }, []);

  return (
    <section
      ref={root}
      id="manifesto"
      className="relative border-t border-line md:h-[300vh]"
    >
      <div className="flex items-center justify-center md:sticky md:top-12 md:h-[calc(100vh-3rem)]">
        <div className="relative mx-auto w-full max-w-[1280px] border-x border-line px-6 py-24 md:px-10">
          <span aria-hidden className="tick -top-[6px] -left-[6px]" />
          <span aria-hidden className="tick -top-[6px] -right-[5px]" />
          <p className="hud-label text-faint">Manifesto</p>
          <h2 className="mt-8 max-w-4xl font-display text-3xl font-medium leading-[1.2] tracking-[-0.035em] text-ink-strong sm:text-4xl md:text-5xl lg:text-6xl">
            {MANIFESTO_WORDS.map((word, i) => (
              <span key={i} data-word className="mr-[0.25em] inline-block">
                {word === MANIFESTO_EMPHASIS ? (
                  <span className="relative inline-block">
                    <span
                      data-sweep
                      aria-hidden
                      className="absolute inset-x-[-0.1em] bottom-[0.05em] h-[0.35em] origin-left bg-accent/80"
                    />
                    <span className="relative">{word}</span>
                  </span>
                ) : (
                  word
                )}
              </span>
            ))}
          </h2>
        </div>
      </div>
    </section>
  );
}
