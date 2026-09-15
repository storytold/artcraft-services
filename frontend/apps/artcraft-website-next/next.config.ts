import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  reactStrictMode: true,
  // Pin the project root: the repo has lockfiles both here and in the Nx
  // workspace above, and letting Next guess picks the wrong one (breaks
  // output file tracing on serverless deploys).
  turbopack: { root: __dirname },
  outputFileTracingRoot: __dirname,
  async redirects() {
    // Legacy Stripe return paths from the Vite site; checkout now lives in
    // the webapp, which serves the canonical slash forms.
    return ["checkout_success", "checkout_cancel", "portal_closed"].map(
      (route) => ({
        source: `/${route}`,
        destination: `https://app.getartcraft.com/checkout/${
          route === "checkout_success" ? "success" : "cancel"
        }`,
        permanent: false,
      }),
    );
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
