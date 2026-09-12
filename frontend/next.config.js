// Vercel must never publish a bundle pointing at localhost or the old backend.
if (process.env.VERCEL === '1') {
  const api = process.env.NEXT_PUBLIC_API_URL;
  if (!api) throw new Error('Set NEXT_PUBLIC_API_URL to your actual Render HTTPS URL in Vercel, then redeploy.');
  const url = new URL(api);
  if (url.protocol !== 'https:' || ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname) || url.pathname !== '/' || url.search || url.hash) {
    throw new Error('NEXT_PUBLIC_API_URL must be an HTTPS backend origin, without an API path.');
  }
  const ws = process.env.NEXT_PUBLIC_WS_URL;
  if (ws) {
    const socket = new URL(ws);
    if (socket.protocol !== 'wss:' || socket.host !== url.host || socket.pathname !== '/ws/live') {
      throw new Error('NEXT_PUBLIC_WS_URL must match the backend host with wss:// and /ws/live. Remove it to derive it automatically.');
    }
  }
}

/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  devIndicators: false,
  outputFileTracingRoot: __dirname,
  transpilePackages: ['deck.gl', '@deck.gl/core', '@deck.gl/layers', '@deck.gl/mapbox'],
};

module.exports = nextConfig;
