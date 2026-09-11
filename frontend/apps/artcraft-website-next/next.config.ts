import type { NextConfig } from "next";

// Marketing routes the nav/footer reference but this app doesn't serve yet.
// They bounce to the currently shipping site (same dependency as MEDIA_BASE —
// this app already assumes the Vite site stays live) so no link dead-ends.
// Delete a route from this list when its page migrates here.
const LEGACY_ROUTES = [
  "download",
  "pricing",
  "tutorials",
  "news",
  "faq",
  "press-kit",
  "support",
];

const nextConfig: NextConfig = {
  reactStrictMode: true,
  // Pin the project root: the repo has lockfiles both here and in the Nx
  // workspace above, and letting Next guess picks the wrong one (breaks
  // output file tracing on serverless deploys).
  turbopack: { root: __dirname },
  outputFileTracingRoot: __dirname,
  async redirects() {
    return LEGACY_ROUTES.map((route) => ({
      source: `/${route}`,
      destination: `https://getartcraft.com/${route}`,
      permanent: false,
    }));
  },
  // Same-origin proxies for large media served from other origins. The hero
  // wall draws its clips as WebGL video textures, which need CORS-clean
  // sources, and neither CDN sends Access-Control-Allow-Origin, so those
  // requests route through our own origin.
  async rewrites() {
    return [
      // Showcase clips for the hero wall (see cdnMediaUrl in lib/links.ts).
      {
        source: "/cdn-media/:path*",
        destination: "https://frontend-cdn.fakeyou.com/:path*",
      },
      // Feature footage and other assets still hosted on the live site.
      {
        source: "/ext-media/:path*",
        destination: "https://getartcraft.com/:path*",
      },
    ];
  },
};

export default nextConfig;
