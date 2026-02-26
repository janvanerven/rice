# Stage 1: Build frontend
FROM node:22-alpine AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ .
RUN npm run build

# Stage 2: Build backend
FROM rust:1.88-bookworm AS backend-builder
WORKDIR /app
COPY backend/Cargo.toml backend/Cargo.lock ./backend/
COPY backend/.sqlx ./backend/.sqlx

# Build deps only (layer caching)
RUN mkdir -p backend/src && echo "fn main() {}" > backend/src/main.rs
RUN cd backend && SQLX_OFFLINE=true cargo build --release
RUN rm -rf backend/src

# Copy real source + frontend assets
COPY backend/src ./backend/src
COPY backend/migrations ./backend/migrations
COPY --from=frontend-builder /app/frontend/dist ./frontend/dist
RUN touch backend/src/main.rs
RUN cd backend && SQLX_OFFLINE=true cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates wget && rm -rf /var/lib/apt/lists/*

COPY --from=backend-builder /app/backend/target/release/rice /usr/local/bin/rice

RUN mkdir -p /data/db /data/uploads

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD wget -q --spider http://localhost:3000/health || exit 1

CMD ["rice"]
