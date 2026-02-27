# Auto-Cover Image Service — Design Document

**Date:** 2026-02-27
**Status:** Draft

## Overview

An auto-cover service that uses the Unsplash API to find default cover images for trips, accommodations, and (future) activities when the user hasn't uploaded one. Images are fetched on-demand from the frontend, cached locally, and auto-applied silently.

## Decisions

| Aspect | Decision |
|---|---|
| Trigger | Frontend on-demand via shared context |
| Source | Unsplash API (free tier, 50 req/hr) |
| Search strategy | Smart fallback chain (specific → broad) |
| Storage | Download and cache locally in `/data/uploads/` |
| UX | Auto-apply silently, shimmer while loading |
| Attribution | `image_attributions` table, credit overlay on images |
| Config | `UNSPLASH_ACCESS_KEY` env var, feature disabled if absent |
| Authorization | Editor+ role required |

## Backend

### Configuration

New env var: `UNSPLASH_ACCESS_KEY`. The app starts without it — the feature is simply unavailable. Endpoints return `503 Service Unavailable` when the key is missing.

### New Endpoints

```
POST /api/trips/{trip_id}/auto-cover
POST /api/trips/{trip_id}/accommodations/{acc_id}/auto-cover
```

Both require editor+ role via the existing `TripAccess` extractor.

### Request Flow

1. Check rate limit: reject if entity had an auto-cover attempt within the last 10 minutes (via `fetched_at` in the attributions table).
2. Check idempotency: if `cover_image_path` is already set, return the existing path (no-op).
3. Run the smart fallback chain against Unsplash (see below).
4. If no results from any fallback step, return `404`.
5. Download the image (with safeguards, see below).
6. In a single DB transaction: update the entity's cover image field and upsert the attribution row.
7. Fire Unsplash download tracking endpoint (`GET /photos/{id}/download`) in a background task. Log failures at ERROR level.
8. Return the new image path and attribution data.

### Smart Fallback Chain

**Trip:** Search Unsplash for `destination`.

**Accommodation:** Try each in order, stop on first hit:
1. `name`
2. `name` + `address`
3. Trip's `destination`

**Activity (future):** Try each in order:
1. `name`
2. Trip's `destination`

Unsplash query: `GET https://api.unsplash.com/search/photos?query={term}&per_page=1&orientation=landscape`

### Download Safeguards

- **SSRF protection:** Whitelist Unsplash CDN hostnames (`images.unsplash.com`, `plus.unsplash.com`). Reject download URLs pointing elsewhere. Disable redirect following on the HTTP client.
- **Size cap:** Stream the download with a 5MB limit (same as manual uploads). Abort if exceeded.
- **Hard timeout:** 10 seconds total for the search + download flow.
- **Temp file strategy:** Write to a temp file first (`{ulid}.tmp`), then rename to final path (`{ulid}.jpg`) only after the DB transaction commits. Delete the temp file on any failure.
- **Access key redaction:** Strip `client_id` from any error messages or logs originating from Unsplash API calls.
- **Disk full handling:** Catch I/O errors during write, clean up partial files, return 503.

### Image Details

- Download the "regular" size (1080px wide) — good balance of quality vs. file size.
- Store at `/data/uploads/{trip_id}/{ulid}.jpg`.
- Same path convention as manual uploads.

### Service Module

New file: `backend/src/services/auto_cover.rs`

- `search_unsplash(query, access_key) -> Result<Option<UnsplashPhoto>>` — calls the Unsplash search API, returns photo metadata (URL, photographer, etc.)
- `download_photo(photo, trip_id, upload_dir) -> Result<PathBuf>` — downloads to temp file, validates, returns temp path. No DB interaction.
- `apply_auto_cover_trip(trip, pool, upload_dir, access_key) -> Result<Option<AutoCoverResult>>` — orchestrates the fallback chain for trips.
- `apply_auto_cover_accommodation(acc, trip, pool, upload_dir, access_key) -> Result<Option<AutoCoverResult>>` — orchestrates the fallback chain for accommodations.

Critical rule: HTTP calls and file I/O must never happen inside a DB transaction. The flow is: fetch externally → write temp file → begin transaction → update DB → commit → rename file.

### Database

New migration: `0003_image_attributions.sql`

```sql
CREATE TABLE image_attributions (
    entity_type TEXT NOT NULL CHECK (entity_type IN ('trip', 'accommodation', 'activity')),
    entity_id   TEXT NOT NULL,
    author_name TEXT NOT NULL,
    author_url  TEXT NOT NULL,
    source_url  TEXT NOT NULL,
    fetched_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    PRIMARY KEY (entity_type, entity_id)
);
```

Notes:
- No `image_path` column — the canonical path lives on the entity itself.
- CHECK constraint enforces valid entity types.
- `fetched_at` doubles as the rate-limit timestamp. Updated on upsert.
- When a user uploads their own cover, delete the attribution row in the same transaction as the cover path update.

### Attribution in API Responses

Bundle an optional `attribution` field in trip and accommodation responses by left-joining `image_attributions`. The join is on the primary key so it's indexed and cheap.

```json
{
  "id": "...",
  "name": "Kyoto Trip",
  "cover_image_path": "/uploads/trip123/01ARZ.jpg",
  "attribution": {
    "author_name": "Jane Doe",
    "author_url": "https://unsplash.com/@janedoe",
    "source_url": "https://unsplash.com/photos/abc123"
  }
}
```

The `attribution` field is `null` when the cover was user-uploaded or absent.

### Field Naming

The existing accommodation table uses `cover_image_url` while trips use `cover_image_path`. Both store local relative paths. This naming inconsistency should be addressed: rename `accommodations.cover_image_url` to `cover_image_path` in the migration for consistency.

## Frontend

### AutoCoverContext

A shared React context that manages all auto-cover requests. Solves three problems at once: deduplication, concurrency control, and shared state updates.

```
AutoCoverProvider
├── inFlight: Set<string>          // entity IDs with requests in progress
├── attempted: Set<string>         // entity IDs already attempted this session
├── queue: Array<PendingRequest>   // pending requests awaiting a slot
├── concurrencyLimit: 2            // max simultaneous auto-cover requests
└── requestAutoCover(entityType, entityId, tripId) -> void
```

**Deduplication:** Before dispatching, check both `inFlight` and `attempted`. Mark as `inFlight` immediately on dispatch, move to `attempted` on completion (success or failure).

**Concurrency cap:** Max 2 simultaneous requests. Additional requests are queued and dispatched as slots free up. Prevents hammering the backend when an accommodation list with 12 items mounts.

**State propagation:** On success, the context updates the shared trip/accommodation data (via query invalidation or context update) so all consumers see the new cover without refetching.

**Role guard:** Only fire if the current user's role is `'owner'` or `'editor'` — positive assertion, not negative. Skip if role is still loading/undefined.

**Strict Mode:** The context initializes sets as refs (not state) to survive React 18 StrictMode double-mount in development.

### Trigger Points

Components call `requestAutoCover()` from the context in a `useEffect` when they detect a missing cover image:

- **TripCard** — trip with no `cover_image_path`
- **TripDetail** — hero image area
- **AccommodationList** — each accommodation with no `cover_image_path`

Since dedup is handled by the context, it's safe for both TripCard and TripDetail to request the same trip — the second call is a no-op.

### Loading State

While a request is in flight for an entity, show a shimmer/pulse animation on the image placeholder.

- Lock the container's aspect ratio (16:9 for trip covers, 3:2 for accommodation thumbnails) to prevent layout shift (CLS).
- The shimmer replaces the empty state entirely — no visual conflict.
- On failure or 404, revert to the standard empty state. No error toast.

### Attribution Overlay

When an entity has attribution data, display a credit line overlaid on the bottom of the cover image.

**Content:** "Photo by {name} / Unsplash" — the photographer name links to their Unsplash profile.

**Styling:**
- Gradient scrim on the bottom of the image (transparent → rgba(0,0,0,0.6)) to guarantee contrast regardless of photo brightness.
- IBM Plex Mono, small size, semi-transparent white text on the scrim.
- Meets WCAG AA contrast (4.5:1 minimum).

**Interactivity:**
- The attribution link uses `stopPropagation()` to prevent triggering the parent card's click handler.
- `aria-label="Photo by {name} on Unsplash"` for screen readers.
- `target="_blank" rel="noopener noreferrer"` for the external link.

### Viewport Gating (Nice-to-Have)

For the trip grid landing screen, an `IntersectionObserver` gate could defer auto-cover requests until cards enter the viewport. This prevents unnecessary requests for off-screen cards. Not required for MVP but a good optimization if the grid grows large.

## Not In Scope

- Image selection UI (picking from multiple Unsplash results)
- Bulk auto-cover for all entities at once
- Retry/refresh if the user doesn't like the auto-selected image (they upload their own)
- Caching Unsplash search results across sessions
- Offline/service-worker support for auto-cover
