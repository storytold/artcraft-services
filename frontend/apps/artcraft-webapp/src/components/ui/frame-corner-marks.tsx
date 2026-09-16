const CORNERS = ["top-0 left-0", "top-0 right-0", "bottom-0 left-0", "bottom-0 right-0"];

/** Decorative registration marks for a positioned panel. */
export function FrameCornerMarks() {
  return CORNERS.map((corner) => (
    <span key={corner} aria-hidden="true" className={`frame-corner-mark ${corner}`} />
  ));
}
