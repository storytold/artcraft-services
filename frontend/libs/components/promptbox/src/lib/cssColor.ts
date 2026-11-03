// Validates a CSS color before it is written into generated markup (e.g. a
// mention chip's inline `style`). Hosts pass these through props, so a value
// is only accepted when it is plainly a color: a hex color, a bare keyword
// (`red`, `currentcolor`), or an rgb()/rgba()/hsl()/hsla() function whose
// arguments are numbers. Anything else (quotes, angle brackets, `;`, url(),
// var(), ...) is rejected and returns undefined.
//
// Kept as three small anchored patterns with no overlapping quantifiers so
// matching stays linear-time.
const HEX_COLOR = /^#(?:[0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i;
const KEYWORD_COLOR = /^[a-z]{3,30}$/i;
const FUNCTION_COLOR = /^(?:rgb|hsl)a?\([-+0-9.%\s,/]{1,64}\)$/i;

export function sanitizeCssColor(value: unknown): string | undefined {
  if (typeof value !== "string") return undefined;
  const trimmed = value.trim();
  const ok =
    HEX_COLOR.test(trimmed) ||
    KEYWORD_COLOR.test(trimmed) ||
    FUNCTION_COLOR.test(trimmed);
  return ok ? trimmed : undefined;
}
