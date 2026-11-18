import type { MetadataRoute } from "next";
import { HOME_URL } from "@/content/site";

export const dynamic = "force-static";

export default function robots(): MetadataRoute.Robots {
  return { rules: { userAgent: "*", allow: "/" }, sitemap: new URL("sitemap.xml", HOME_URL).href };
}
