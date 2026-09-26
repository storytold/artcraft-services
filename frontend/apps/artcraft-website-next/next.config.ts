import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  reactStrictMode: true,
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
  // The hero wall uses WebGL video textures, which need CORS-clean sources.
  // The showcase CDN sends no Access-Control-Allow-Origin header, so these
  // clips use a same-origin proxy. Other marketing media lives in public/.
  async rewrites() {
    return [
      // Showcase clips for the hero wall (see cdnMediaUrl in lib/links.ts).
      {
        source: "/cdn-media/:path*",
        destination: "https://frontend-cdn.fakeyou.com/:path*",
      },
    ];
  },
};

export default nextConfig;
