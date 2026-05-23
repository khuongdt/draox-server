# ================================================
# Draox Admin — Multi-stage Docker Build
# ================================================
# Stage 1: build with Node.js
# Stage 2: serve with nginx (static files + API proxy)
# ================================================

# ── Stage 1: Build ──────────────────────────────
FROM node:24-alpine AS builder

WORKDIR /app

# Copy package manifests first — layer-cache npm install
COPY package.json package-lock.json .npmrc ./

RUN npm ci --legacy-peer-deps

# Copy source
COPY . .

# Production build (UmiJS/Webpack)
RUN npm run build

# ── Stage 2: Serve ──────────────────────────────
FROM nginx:1.29.8-alpine AS runtime

# Remove default nginx page
RUN rm -rf /usr/share/nginx/html/*

# Copy built assets from builder
COPY --from=builder /app/dist /usr/share/nginx/html

# Copy nginx site config
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -qO- http://localhost/ || exit 1

CMD ["nginx", "-g", "daemon off;"]
