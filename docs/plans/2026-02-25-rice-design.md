# Rice (旅) — Design Document

**Date:** 2026-02-25
**Status:** Approved

## Overview

Rice is a self-hosted, collaborative trip/vacation management webapp. Named after the Dutch word "reis" (travel), themed with a Japanese cyberpunk aesthetic.

## Tech Stack

| Layer | Choice | Why |
|-------|--------|-----|
| Backend | Rust + Axum | Single binary, fast, small Docker image |
| Database | SQLite via sqlx | Self-contained, WAL mode, compile-time query checks |
| Frontend | React + Vite + React Router | SPA, small bundles, embedded in Rust binary |
| Styling | CSS Modules + CSS custom properties | Full design control, zero runtime, tree-shakeable |
| Auth | Authentik OAuth2 PKCE | Centralized identity, no local passwords |
| Deploy | Docker (ARM64 + AMD64) | Single container, cross-compiled |

## Architecture

```
┌─────────────────────────────────────────────────┐
│                 Docker Container                 │
│                                                  │
│  ┌───────────────────────────────────────────┐  │
│  │           Axum (Rust) Server              │  │
│  │                                           │  │
│  │  /api/*  →  REST API handlers             │  │
│  │  /auth/* →  OAuth2 flow (Authentik)       │  │
│  │  /*      →  Static React SPA (embedded)   │  │
│  │                                           │  │
│  │  ┌─────────────┐   ┌──────────────────┐  │  │
│  │  │   SQLite     │   │  rust-embed /    │  │  │
│  │  │  (vol: /data │   │  static assets   │  │  │
│  │  │   /db/)      │   └──────────────────┘  │  │
│  │  └─────────────┘                          │  │
│  │  ┌─────────────┐                          │  │
│  │  │  Uploads     │                          │  │
│  │  │  (vol: /data │                          │  │
│  │  │   /uploads/) │                          │  │
│  │  └─────────────┘                          │  │
│  └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

- Axum serves both the API and the static SPA from a single binary
- Static assets embedded via `rust-embed` at compile time
- Two Docker volumes: database + user uploads
- Startup: validate env vars → check DB path writable → run migrations → start server

## Domain Model (MVP)

### Entities

**users**
- `id` — ULID, primary key
- `email` — from Authentik, unique
- `display_name` — from Authentik
- `avatar_url` — from Authentik (nullable)
- `created_at`, `updated_at` — timestamps

**trips**
- `id` — ULID, primary key
- `name` — trip title
- `destination` — free-text destination
- `start_date`, `end_date` — date range (nullable, trips can be undated)
- `cover_image_path` — path to uploaded image (nullable)
- `created_by` — FK to users
- `created_at`, `updated_at` — timestamps

**trip_members**
- `trip_id` — FK to trips (CASCADE DELETE)
- `user_id` — FK to users
- `role` — enum: `owner`, `editor`, `viewer`
- `joined_at` — timestamp
- Primary key: (trip_id, user_id)

**invites**
- `id` — ULID
- `trip_id` — FK to trips (CASCADE DELETE)
- `email` — invited email address
- `token_hash` — hashed single-use token
- `role` — role to assign on claim
- `expires_at` — expiry timestamp (7 days)
- `claimed_by` — FK to users (nullable, set on claim)
- `created_at` — timestamp

**sessions**
- `id` — JTI claim, primary key
- `user_id` — FK to users
- `expires_at` — session expiry
- `created_at` — timestamp

### Relationships

```
users 1──N trip_members N──1 trips
users 1──N sessions
users 1──N invites (claimed_by)
trips 1──N invites
trips 1──N trip_members
```

## API Surface (MVP)

### Auth
| Method | Path | Description |
|--------|------|-------------|
| GET | `/auth/login` | Redirect to Authentik PKCE flow |
| GET | `/auth/callback` | Exchange code, create session, set cookies |
| POST | `/auth/logout` | Delete session, clear cookies |
| GET | `/api/me` | Current user info |

### Trips
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/trips` | user | List user's trips |
| POST | `/api/trips` | user | Create trip |
| GET | `/api/trips/:id` | member | Trip detail |
| PUT | `/api/trips/:id` | editor+ | Update trip |
| DELETE | `/api/trips/:id` | owner | Delete trip |
| POST | `/api/trips/:id/cover` | editor+ | Upload cover image |

### Members & Invites
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/trips/:id/members` | member | List members |
| POST | `/api/trips/:id/invites` | owner | Create invite |
| DELETE | `/api/trips/:id/members/:uid` | owner | Remove member |

### System
| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | DB ping + status |

## Auth & Security

### OAuth2 Flow
1. SPA redirects to `/auth/login`
2. Axum redirects to Authentik with PKCE challenge + state
3. Authentik authenticates user, redirects to `/auth/callback`
4. Axum exchanges code for token, fetches user info
5. Creates/updates user in SQLite, creates session record
6. Signs JWT (containing `jti` from sessions table), sets as httponly cookie
7. On subsequent requests, JWT is validated and `jti` checked against sessions table

### Session Management
- **Access token:** JWT in httponly/secure/SameSite=Lax cookie, 15 min expiry
- **Refresh token:** Opaque token in httponly cookie, 30 day expiry, stored in sessions table
- **Revocation:** Delete session row → JWT validation fails on `jti` check
- **Logout:** Clears both cookies + deletes session row

### Security Measures
- JWT_SECRET validated at startup (refuse to boot if missing/weak)
- Rate limiting on auth endpoints via `tower-governor`
- CSP + X-Frame-Options + X-Content-Type-Options headers
- `TripAccess` extractor for centralized authorization (no ad-hoc checks)
- Foreign key cascades: deleting a trip removes members + invites
- `updated_at` + audit trail on all entities

### Invite Flow
1. Owner enters collaborator's email + role via API
2. Server creates invite record with hashed single-use token, expires in 7 days
3. Server sends invite email via SMTP with link to the app (e.g., `APP_BASE_URL/invite?token=...`)
4. Recipient clicks link → redirected to Authentik login if not authenticated
5. After authentication, if email matches, auto-claim the invite
6. User appears as trip member immediately
7. Fallback: pending invites also auto-claimed on any login with matching email

### Deployment Context
- Hosted behind a reverse proxy (TLS termination handled externally)
- `secure` cookie flag still set (proxy forwards HTTPS)
- SMTP available for transactional emails (invites, future notifications)

## SQLite Configuration

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 5000;
PRAGMA foreign_keys = ON;
```

Connection pooling:
- 1 write connection (serialized writes)
- N read connections (concurrent reads)

Migrations: `sqlx migrate!()` embedded in binary, run at startup.
Offline mode: `.sqlx/` cache committed, `SQLX_OFFLINE=true` in Docker build.

## Visual Design

### Japanese Cyberpunk Aesthetic

**Palette:**
- Backgrounds: `#0a0a0f` (void), `#0f0f18` (surface), `#141420` (elevated)
- Neons: `#ff6b2b` (orange/primary), `#ff2d55` (red), `#ff0080` (pink), `#ffb830` (amber), `#00d4ff` (cyan)
- Text: `#f0ede8` (primary, warm white), `#9896a4` (secondary), `#5c5a6e` (tertiary)

**Typography:**
- Primary: IBM Plex Sans
- Mono: IBM Plex Mono (data, IDs, coordinates)
- Japanese: Noto Sans JP (accent only — logo, destination names)
- Scale: 1.25 Major Third ratio

**Visual Language:**
- HUD-style labels: monospace, uppercase, tracked wide
- Corner bracket accents on cards (CSS pseudo-elements)
- Scan-line texture at 2-3% opacity on root background
- Neon accents at max 10% of UI — restraint is key
- Cover images with luminosity blend + gradient overlay
- Left border accent on active/selected cards

**Animation (CSS-only, GPU-accelerated):**
- Button hover: translateY(-1px) + neon glow shadow
- Card lift: translateY(-2px) + shadow + glow
- Neon pulse: 3s ease-in-out on primary CTA only
- Page transitions: fade-up (opacity + translateY)
- Loading: scanner line (2px neon gradient traversing viewport)
- All animations respect `prefers-reduced-motion`

### Responsive Strategy

| Element | Mobile | Desktop |
|---------|--------|---------|
| Nav | Bottom tab bar (56px) | Left sidebar (200px) |
| Trip grid | 1 column | 3-4 columns (auto-fill, minmax 260px) |
| Modals | Bottom sheets | Centered dialogs |
| Page padding | 16px | 40px |
| Typography | `--text-lg` for h1 | `--text-xl`+ for h1 |

- Breakpoints: 480 / 768 / 1024 / 1280
- Touch targets: minimum 44px
- Safe area insets for future Capacitor wrapping

### Performance Budget
- Initial JS: < 80KB gzipped
- Initial CSS: < 10KB gzipped
- Total first load: < 400KB
- TTI: < 2s on 3G
- No animation libraries (pure CSS)
- No heavy state management (useState + useContext for MVP)

## Docker & Deployment

### Environment Variables
```
DATABASE_URL=sqlite:///data/db/rice.db
JWT_SECRET=<32+ byte random secret>
AUTHENTIK_CLIENT_ID=<client id>
AUTHENTIK_CLIENT_SECRET=<client secret>
AUTHENTIK_BASE_URL=https://auth.yourdomain.com
APP_BASE_URL=https://rice.yourdomain.com
SMTP_HOST=<smtp host>
SMTP_PORT=587
SMTP_USERNAME=<smtp user>
SMTP_PASSWORD=<smtp password>
SMTP_FROM=rice@yourdomain.com
RUST_LOG=info
```

### Build
- Multi-stage: Node builds Vite SPA → Rust cross-compiles + embeds
- Cross-compilation for ARM64 (not QEMU emulation)
- Cargo layer caching: copy manifests → build deps → copy source → incremental build
- `.sqlx/` cache committed for offline compilation

### Runtime
- Health check: `GET /health` → DB ping
- Graceful shutdown for in-flight requests
- Startup validation: env vars present, DB writable, migrations run
- Backup: `sqlite3 /data/db/rice.db ".backup /backup/rice.db"`

### docker-compose.yml
```yaml
services:
  rice:
    image: rice:latest
    ports:
      - "3000:3000"
    environment:
      - DATABASE_URL=sqlite:///data/db/rice.db
      - JWT_SECRET=${JWT_SECRET}
      - AUTHENTIK_CLIENT_ID=${AUTHENTIK_CLIENT_ID}
      - AUTHENTIK_CLIENT_SECRET=${AUTHENTIK_CLIENT_SECRET}
      - AUTHENTIK_BASE_URL=${AUTHENTIK_BASE_URL}
      - APP_BASE_URL=${APP_BASE_URL}
    volumes:
      - rice_db:/data/db
      - rice_uploads:/data/uploads
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:3000/health"]
      interval: 30s
      timeout: 5s
      retries: 3

volumes:
  rice_db:
  rice_uploads:
```

## Future Extensions (Not in MVP)

- Accommodations (hotels/campsites/bnbs per trip)
- Day-by-day itinerary with activities
- Interactive map view
- iOS app via Capacitor
- Real-time collaborative editing (would need WebSocket + CRDT)
- Trip templates
- Budget tracking
- Document/photo sharing
