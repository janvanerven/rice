# Rice 旅

Self-hosted trip planning app with a Japanese cyberpunk aesthetic. Built with Rust + React, deployed as a single Docker container.

## Quick Start (Docker)

```bash
cp .env.example .env
# Edit .env with your values (see Configuration below)
docker compose up -d
```

The app will be available at `http://localhost:3000`.

## Configuration

All configuration is via environment variables. See `.env.example` for the full list.

**Required:**

| Variable | Description |
|----------|-------------|
| `DATABASE_URL` | SQLite path (default: `sqlite:///data/db/rice.db`) |
| `JWT_SECRET` | Random string, minimum 32 characters |
| `AUTHENTIK_CLIENT_ID` | OAuth2 client ID from Authentik |
| `AUTHENTIK_CLIENT_SECRET` | OAuth2 client secret from Authentik |
| `AUTHENTIK_BASE_URL` | Your Authentik instance URL |
| `APP_BASE_URL` | Public URL where Rice is hosted |

**Optional (email invites):**

| Variable | Default | Description |
|----------|---------|-------------|
| `SMTP_HOST` | — | SMTP server hostname |
| `SMTP_PORT` | `587` | SMTP server port |
| `SMTP_USERNAME` | — | SMTP auth username |
| `SMTP_PASSWORD` | — | SMTP auth password |
| `SMTP_FROM` | — | Sender email address |

If SMTP is not configured, the app runs without email invite support.

## Authentik Setup

1. Create a new OAuth2/OpenID provider in Authentik
2. Set the redirect URI to `{APP_BASE_URL}/auth/callback`
3. Copy the client ID and secret to your `.env`
4. Ensure the provider includes `email`, `openid`, and `profile` scopes

## Development

**Prerequisites:** Rust 1.88+, Node.js 22+, SQLite

```bash
# Backend
cd backend
cp ../.env.example ../.env  # edit with local values
DATABASE_URL=sqlite://rice-dev.db cargo run

# Frontend (separate terminal)
cd frontend
npm install
npm run dev
```

The Vite dev server runs on `http://localhost:5173` and proxies API requests to the backend on port 3000.

**Run tests:**

```bash
cd backend && cargo test
```

## Architecture

```
rice/
├── backend/          Rust (Axum) API server
│   ├── src/
│   │   ├── api/      Trip, member, invite, upload handlers
│   │   ├── auth/     OAuth2 PKCE + JWT sessions
│   │   └── ...       Config, DB, email, middleware, models
│   ├── migrations/   SQLite migrations (auto-run on start)
│   └── tests/        Integration tests
├── frontend/         React (Vite) SPA
│   └── src/
│       ├── components/  UI + layout + trip components
│       ├── pages/       Login, Dashboard, Trip detail/new
│       ├── hooks/       useTrips, useMediaQuery
│       └── lib/         API client, auth context
├── Dockerfile        Multi-stage build (Node → Rust → Debian slim)
└── docker-compose.yml
```

The frontend is embedded into the Rust binary at compile time via `rust-embed` and served as a fallback for SPA routing. Two Docker volumes persist data: `/data/db/` (SQLite) and `/data/uploads/` (cover images).

## Tech Stack

- **Backend:** Rust, Axum, sqlx, SQLite (WAL mode)
- **Frontend:** React 19, Vite, React Router, CSS Modules
- **Auth:** Authentik OAuth2 PKCE → JWT (httponly cookies)
- **Email:** Lettre (SMTP)
- **Deploy:** Single Docker container (171MB), ARM64 + AMD64
