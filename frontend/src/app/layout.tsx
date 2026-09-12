import type { Metadata } from 'next';
import './globals.css';

export const metadata: Metadata = {
  title: 'BIQ — Bharat Intel Quantum',
  description: 'Map-first India border surveillance',
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
