# ─── Stage 1: Dependencies ────────────────────────────────────────────────────
FROM node:20-alpine AS deps
RUN apk add --no-cache libc6-compat

WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci --frozen-lockfile

# ─── Stage 2: Builder ─────────────────────────────────────────────────────────
FROM node:20-alpine AS builder
WORKDIR /app

COPY --from=deps /app/node_modules ./node_modules
COPY . .

# Build-time env vars — runtime values injected via nginx/compose env
ARG NEXT_PUBLIC_API_URL=http://quantum-api:8000
ARG NEXT_PUBLIC_WS_URL=ws://quantum-api:8000
ARG NEXT_PUBLIC_MAPBOX_TOKEN=""

ENV NEXT_PUBLIC_API_URL=$NEXT_PUBLIC_API_URL
ENV NEXT_PUBLIC_WS_URL=$NEXT_PUBLIC_WS_URL
ENV NEXT_PUBLIC_MAPBOX_TOKEN=$NEXT_PUBLIC_MAPBOX_TOKEN
ENV NEXT_TELEMETRY_DISABLED=1
ENV NODE_ENV=production

RUN npm run build

# ─── Stage 3: Production runner (node + nginx) ─────────────────────────────────
FROM node:20-alpine AS runner

WORKDIR /app

# Install nginx and other tools
RUN apk add --no-cache nginx curl wget

# Non-root execution
RUN addgroup -g 1001 -S biq && adduser -u 1001 -S biq -G biq

# Remove default nginx config
RUN rm -rf /etc/nginx/http.d/default.conf /etc/nginx/conf.d/default.conf

# Copy nginx site config
COPY nginx.frontend.conf /etc/nginx/http.d/biq-frontend.conf

# Copy built Next.js static export / standalone
COPY --from=builder /app/.next/standalone /app/server
COPY --from=builder /app/.next/static /app/server/.next/static
COPY --from=builder /app/public /app/server/public

# Switch to non-root for static files
RUN chown -R biq:biq /app /var/lib/nginx /var/log/nginx /run/nginx

EXPOSE 80
EXPOSE 3000

HEALTHCHECK --interval=15s --timeout=5s --start-period=30s --retries=3 \
  CMD wget -qO- http://localhost:80/ || exit 1

# Start nginx in background and node in foreground
ENV HOSTNAME "0.0.0.0"
CMD ["sh", "-c", "nginx && node /app/server/server.js"]
