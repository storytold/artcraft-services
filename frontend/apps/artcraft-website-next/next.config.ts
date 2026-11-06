import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  reactStrictMode: true,
  // Share previews must work from the raw <head>, even for an unknown crawler
  // or curl. Wait for generateMetadata instead of streaming it into the body.
  // Media lookups have a five-second timeout; static marketing pages do not
  // perform that lookup. https://nextjs.org/docs/app/api-reference/config/next-config-js/htmlLimitedBots
  htmlLimitedBots: /.*/,
  // Pin the project root: the repo has lockfiles both here and in the Nx
  // workspace above, and letting Next guess picks the wrong one (breaks
  // output file tracing on serverless deploys).
  turbopack: { root: __dirname },
  outputFileTracingRoot: __dirname,
  async redirects() {
    return [
      // Legacy Stripe return paths from the Vite site; checkout now lives in
      // the webapp, which serves the canonical slash forms.
      ...["checkout_success", "checkout_cancel", "portal_closed"].map(
        (route) => ({
          source: `/${route}`,
          destination: `https://app.getartcraft.com/checkout/${
            route === "checkout_success" ? "success" : "cancel"
          }`,
          permanent: false,
        }),
      ),
      // LoopCraft is no longer offered: its old page and share images go to
      // the Crafting Apps index.
      {
        source: "/apps/loopcraft",
        destination: "/apps",
        permanent: true,
      },
      {
        source: "/images/apps/loopcraft/:file",
        destination: "/apps",
        permanent: true,
      },
      // DrawCraft was renamed VectorCraft; the Crafting Apps READMEs still
      // link the old page.
      {
        source: "/apps/drawcraft",
        destination: "/apps/vectorcraft",
        permanent: true,
      },
      // PrintCraft was renamed PdfCraft. Old page links and share images
      // (cached by social sites) follow it.
      {
        source: "/apps/printcraft",
        destination: "/apps/pdfcraft",
        permanent: true,
      },
      {
        source: "/images/apps/printcraft/:file",
        destination: "/images/apps/pdfcraft/:file",
        permanent: true,
      },
      // Legacy share-link form (`/media?media=<token>`) from the Vite site;
      // the canonical route is /media/<token>. Mirrored in netlify.toml so
      // the edge answers it before the Next runtime.
      {
        source: "/media",
        has: [{ type: "query", key: "media", value: "(?<token>.+)" }],
        destination: "/media/:token",
        permanent: true,
      },
    ];
  },
};

export default nextConfig;
