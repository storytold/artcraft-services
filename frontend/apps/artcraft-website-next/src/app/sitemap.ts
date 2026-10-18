import type { MetadataRoute } from "next";
import { CRAFTING_APPS, craftAppPath } from "@/lib/crafting-apps";
import { getFaqItems, getNewsPosts, getTutorialItems } from "@/lib/content";
import { SITE_URL } from "@/lib/links";

type Entry = MetadataRoute.Sitemap[number];

const STATIC_ROUTES: { path: string; priority: number; changeFrequency: Entry["changeFrequency"] }[] = [
  { path: "/", priority: 1, changeFrequency: "weekly" },
  { path: "/pricing", priority: 0.9, changeFrequency: "weekly" },
  { path: "/download", priority: 0.9, changeFrequency: "weekly" },
  { path: "/tutorials", priority: 0.7, changeFrequency: "weekly" },
  { path: "/news", priority: 0.7, changeFrequency: "weekly" },
  { path: "/faq", priority: 0.7, changeFrequency: "monthly" },
  { path: "/support", priority: 0.5, changeFrequency: "monthly" },
  { path: "/press-kit", priority: 0.5, changeFrequency: "monthly" },
  { path: "/beta", priority: 0.5, changeFrequency: "monthly" },
  { path: "/mission", priority: 0.5, changeFrequency: "monthly" },
  { path: "/seedance-2", priority: 0.8, changeFrequency: "weekly" },
  { path: "/seedance2-5", priority: 0.8, changeFrequency: "weekly" },
  { path: "/minimax-h3", priority: 0.8, changeFrequency: "weekly" },
  { path: "/creators/jboogxcreative", priority: 0.6, changeFrequency: "monthly" },
  { path: "/apps", priority: 0.8, changeFrequency: "weekly" },
  ...CRAFTING_APPS.map((app) => ({
    path: craftAppPath(app),
    priority: 0.8,
    changeFrequency: "weekly" as const,
  })),
];

export default function sitemap(): MetadataRoute.Sitemap {
  const staticEntries: Entry[] = STATIC_ROUTES.map((route) => ({
    url: `${SITE_URL}${route.path}`,
    changeFrequency: route.changeFrequency,
    priority: route.priority,
  }));

  const articleEntries: Entry[] = [
    ...getFaqItems().map((item) => `/faq/${item.slug}`),
    ...getTutorialItems().map((item) => `/tutorials/${item.slug}`),
    ...getNewsPosts().map((post) => `/news/${post.slug}`),
  ].map((path) => ({
    url: `${SITE_URL}${path}`,
    changeFrequency: "monthly" as const,
    priority: 0.6,
  }));

  return [...staticEntries, ...articleEntries];
}
