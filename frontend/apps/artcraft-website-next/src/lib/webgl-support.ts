// Pre-flight WebGL probe, shared by every r3f canvas gate.
//
// When context creation fails (exhausted drivers, blocklisted GPU,
// software-rendering-only environments), THREE.WebGLRenderer throws inside
// r3f's async init — an unhandled REJECTION that render-phase error
// boundaries (CanvasBoundary) never see. Probing before mounting the
// <Canvas> keeps the failure on our terms: the canvas simply never mounts
// and the page's static fallbacks stand. Result is cached — a probe is
// cheap but not free, and the answer doesn't change within a page's life.
let cached: boolean | null = null;

export function webglAvailable(): boolean {
  if (cached !== null) return cached;
  try {
    const canvas = document.createElement("canvas");
    const gl =
      canvas.getContext("webgl2") ??
      canvas.getContext("webgl") ??
      canvas.getContext("experimental-webgl");
    cached = !!gl;
    // Release the probe context immediately rather than waiting for GC —
    // browsers cap live contexts, and the real canvases want the slots.
    if (gl && "getExtension" in (gl as WebGLRenderingContext)) {
      (gl as WebGLRenderingContext)
        .getExtension("WEBGL_lose_context")
        ?.loseContext();
    }
  } catch {
    cached = false;
  }
  return cached;
}
