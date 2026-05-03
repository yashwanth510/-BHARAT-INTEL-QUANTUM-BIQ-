import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  reactCompiler: true,
  turbopack: {},

  // Standalone output for Docker/nginx deployment
  output: "standalone",

  // API rewrites — frontend proxies to backend (avoids CORS in dev)
  async rewrites() {
    const apiBase =
      process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8000";
    return [
      // REST API endpoints
      {
        source: "/api/:path*",
        destination: `${apiBase}/api/:path*`,
      },
      // Legacy non-prefixed endpoints
      {
        source: "/health",
        destination: `${apiBase}/health`,
      },
      {
        source: "/quantum-health",
        destination: `${apiBase}/quantum-health`,
      },
      {
        source: "/metrics",
        destination: `${apiBase}/metrics`,
      },
      {
        source: "/ops-log",
        destination: `${apiBase}/ops-log`,
      },
      {
        source: "/maritime-threats",
        destination: `${apiBase}/maritime-threats`,
      },
      {
        source: "/news-threats",
        destination: `${apiBase}/news-threats`,
      },
      {
        source: "/weather-threats",
        destination: `${apiBase}/weather-threats`,
      },
      {
        source: "/geospatial-threats",
        destination: `${apiBase}/geospatial-threats`,
      },
      {
        source: "/satellite-alerts",
        destination: `${apiBase}/satellite-alerts`,
      },
      {
        source: "/crypto-threats",
        destination: `${apiBase}/crypto-threats`,
      },
      {
        source: "/ml-anomaly",
        destination: `${apiBase}/ml-anomaly`,
      },
      {
        source: "/pakistan-threats",
        destination: `${apiBase}/pakistan-threats`,
      },
      {
        source: "/china-threats",
        destination: `${apiBase}/china-threats`,
      },
    ];
  },

  // Security headers
  async headers() {
    return [
      {
        source: "/(.*)",
        headers: [
          { key: "X-Frame-Options", value: "SAMEORIGIN" },
          { key: "X-Content-Type-Options", value: "nosniff" },
          { key: "X-XSS-Protection", value: "1; mode=block" },
          {
            key: "Referrer-Policy",
            value: "strict-origin-when-cross-origin",
          },
        ],
      },
    ];
  },

  // Webpack config for mapbox-gl
  webpack: (config) => {
    config.resolve.alias = {
      ...config.resolve.alias,
      "mapbox-gl": "mapbox-gl",
    };
    return config;
  },
};

export default nextConfig;
