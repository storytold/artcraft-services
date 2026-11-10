import type { MetadataRoute } from "next";

export const dynamic = "force-static";

/**
 * Generates the static `/robots.txt` metadata route configuration.
 */
export default function robots(): MetadataRoute.Robots {
  return {
    rules: { userAgent: "*", allow: "/" },
    sitemap: "https://getartcraft.com/sitemap.xml",
  };
}
