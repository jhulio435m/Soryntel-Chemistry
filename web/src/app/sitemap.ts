import type { MetadataRoute } from "next";
import { HOME_URL } from "@/content/site";

export const dynamic = "force-static";

export default function sitemap(): MetadataRoute.Sitemap {
  return [{ url: HOME_URL }];
}
