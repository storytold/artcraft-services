"use client";

import { useEffect, useRef } from "react";

export default function AudioWaveform({ analyser, playing }: { analyser: AnalyserNode | null; playing: boolean }) {
  const line = useRef<SVGPathElement>(null);
  useEffect(() => {
    if (!analyser || !playing) return;
    const samples = new Float32Array(analyser.fftSize);
    let frame = 0;
    // A live oscilloscope: actual audio samples, never decorative random bars.
    // Motion-sensitive visitors retain the static baseline and all controls.
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    function draw() {
      analyser!.getFloatTimeDomainData(samples);
      const points = Array.from({ length: 160 }, (_, index) => {
        const amplitude = samples[Math.floor(index / 160 * samples.length)];
        return `${index ? "L" : "M"}${index / 159 * 640},${36 - Math.max(-1, Math.min(1, amplitude)) * 30}`;
      });
      line.current?.setAttribute("d", points.join(" "));
      frame = requestAnimationFrame(draw);
    }
    draw();
    return () => cancelAnimationFrame(frame);
  }, [analyser, playing]);

  return (
    <div className="relative w-full border-y border-line bg-bg-sunken px-5 py-3" aria-label="Live audio waveform">
      <svg viewBox="0 0 640 72" className="h-16 w-full text-accent-ink" preserveAspectRatio="none" aria-hidden>
        <path d="M0,36 L640,36" stroke="var(--line)" strokeWidth="1" />
        <path ref={line} d="M0,36 L640,36" fill="none" stroke="currentColor" strokeWidth="1.5" vectorEffect="non-scaling-stroke" />
      </svg>
    </div>
  );
}
