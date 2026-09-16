import type { ReactNode } from "react";

// Structural frame shared by every landing section: a full-bleed top rule,
// content constrained between two continuous side rails, and crosshair ticks
// straddling the rail intersections.
//
// The top rule and ticks are scroll-choreographed (see reveal-manager.tsx):
// as a section enters the viewport its rule DRAWS across and the ticks pop
// in — scrub-linked, so scrolling back un-draws them. They render complete
// in the server HTML; only motion-capable visitors get the choreography.
export function SectionShell({
  id,
  children,
  className = "",
  ticks = true,
}: {
  id?: string;
  children: ReactNode;
  className?: string;
  ticks?: boolean;
}) {
  return (
    <section id={id} data-choreo className={`relative ${className}`}>
      <span
        aria-hidden
        data-draw-rule
        className="absolute inset-x-0 top-0 h-px origin-left bg-line"
      />
      <div className="relative mx-auto max-w-[1280px] border-x border-line">
        {ticks && (
          <>
            {/* -top-[5px] centers the crossbar on the section's top
                hairline; z-[31] keeps the downward stems above the frosted
                eyebrow band (z-30), which otherwise swallows them. */}
            <span
              aria-hidden
              data-draw-tick
              className="tick tick-half z-[31] -top-[5px] -left-[6px]"
            />
            <span
              aria-hidden
              data-draw-tick
              className="tick tick-half z-[31] -top-[5px] -right-[5px]"
            />
          </>
        )}
        {children}
      </div>
    </section>
  );
}

// Mono eyebrow row at the top of a section: index + label left, optional
// annotation right. Sticky while its section scrolls (pushed away by the
// next section — never stacking), frosted like the ruler rail so content
// ghosts through beneath the label.
export function SectionEyebrow({
  index,
  label,
  annotation,
}: {
  index: string;
  label: string;
  annotation?: string;
}) {
  return (
    // The band owns BOTH hairlines: the top one keeps the eyebrow edged
    // while it floats stuck mid-section (and coincides with the section's
    // drawn rule at rest, so they read as one line). The whole band fades
    // with the draw choreography — frost, rules, and contents together.
    <div
      data-draw-eyebrow
      className="sticky top-12 z-30 border-y border-line bg-bg/60 backdrop-blur-md"
    >
      <div className="flex items-center justify-between gap-4 px-6 py-3 md:px-10">
        <p className="hud-label text-muted">
          <span className="text-faint">{index}</span>
          <span aria-hidden className="mx-2 text-faint">
            /
          </span>
          {label}
        </p>
        {annotation && (
          <p className="hud-label hidden text-faint sm:block">{annotation}</p>
        )}
      </div>
    </div>
  );
}
