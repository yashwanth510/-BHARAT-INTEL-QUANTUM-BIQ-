import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "./globals.css";
import { AppProviders } from "@/providers/AppProviders";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "BHARAT INTEL QUANTUM | BIQ",
  description: "AI-Powered Tactical Intelligence Fusion Platform",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className="dark">
      <body className={`${inter.className} antialiased bg-background text-foreground overflow-hidden`}>
        <AppProviders>
          {children}
        </AppProviders>
      </body>
    </html>
  );
}
