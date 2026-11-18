import type { Metadata, Viewport } from "next";
import localFont from "next/font/local";
import { HOME_URL, VERSION } from "@/content/site";
import "./globals.css";

// Self-hosted Geist (SIL OFL), variable weight; covers Spanish via Latin‑1.
const sans = localFont({ src: "./fonts/geist-latin.woff2", variable: "--font-sans", weight: "100 900", display: "swap" });
const mono = localFont({ src: "./fonts/geist-mono-latin.woff2", variable: "--font-geist-mono", weight: "100 900", display: "swap" });

const title = "Soryntel Chemistry | Software de termodinámica";
const description =
  "Software gratuito para Windows y Linux. Calcula equilibrio químico, reacciones, balances y propiedades del agua y vapor IF97 con SoryntelDB.";

export const metadata: Metadata = {
  metadataBase: new URL(HOME_URL),
  title,
  description,
  alternates: { canonical: HOME_URL },
  openGraph: {
    type: "website",
    locale: "es_PE",
    url: HOME_URL,
    siteName: "Soryntel Chemistry",
    title,
    description,
    images: [{ url: "/media/water.jpg", width: 1440, height: 872, alt: "Calculadora de agua y vapor IF97 de Soryntel Chemistry" }],
  },
  twitter: { card: "summary_large_image", title, description, images: ["/media/water.jpg"] },
};

const structuredData = {
  "@context": "https://schema.org",
  "@graph": [
    {
      "@type": "WebSite",
      name: "Soryntel Chemistry",
      url: HOME_URL,
      inLanguage: "es",
    },
    {
      "@type": "SoftwareApplication",
      name: "Soryntel Chemistry",
      url: HOME_URL,
      description,
      operatingSystem: ["Windows", "Linux"],
      applicationCategory: "EducationalApplication",
      softwareVersion: VERSION,
      isAccessibleForFree: true,
    },
  ],
};

export const viewport: Viewport = {
  themeColor: [
    { media: "(prefers-color-scheme: light)", color: "#f4f6f3" },
    { media: "(prefers-color-scheme: dark)", color: "#121a18" },
  ],
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="es" className={`${sans.variable} ${mono.variable} antialiased`}>
      <body className="min-h-dvh">
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(structuredData).replace(/</g, "\\u003c") }}
        />
        {children}
      </body>
    </html>
  );
}
