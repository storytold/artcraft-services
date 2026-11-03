// Validates a CSS color before it is written into generated markup (e.g. a
// mention chip's inline `style`). Hosts pass these through props, so a value
// is only accepted when it is plainly a color: a hex color, a bare keyword
// (`red`, `currentcolor`), or an rgb()/rgba()/hsl()/hsla() function whose
// arguments are numbers. Anything else (quotes, angle brackets, `;`, url(),
// var(), ...) is rejected and returns undefined.
const SAFE_CSS_COLOR =
  /^(?:#[0-9a-f]{3,8}|[a-z]{3,30}|(?:rgba?|hsla?)\(\s*[-+0-9.%\s,/]+\))$/i;

export function sanitizeCssColor(value: unknown): string | undefined {
  if (typeof value !== "string") return undefined;
  const trimmed = value.trim();
  return SAFE_CSS_COLOR.test(trimmed) ? trimmed : undefined;
}
