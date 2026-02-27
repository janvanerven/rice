# Accommodations Feature Design

## Overview

Add accommodation tracking to trips — where you're staying each night. Basics only: name, dates, address, notes, and an optional cover image.

## Data Model

New `accommodations` table:

| Column | Type | Notes |
|--------|------|-------|
| `id` | TEXT (ULID) | Primary key |
| `trip_id` | TEXT | FK → trips, CASCADE delete |
| `name` | TEXT NOT NULL | e.g. "Hotel Akira" |
| `address` | TEXT | Optional |
| `check_in` | TEXT | ISO date, optional |
| `check_out` | TEXT | ISO date, optional |
| `notes` | TEXT | Optional freeform |
| `cover_image_url` | TEXT | Optional, reuses upload flow |
| `created_at` | TEXT | Default datetime('now') |
| `updated_at` | TEXT | Default datetime('now') |

## API

All under `/api/trips/{trip_id}/accommodations`, protected by `TripAccess` extractor.

- `GET /` — list all accommodations for a trip (any member)
- `POST /` — create accommodation (editor/owner)
- `PUT /{id}` — update (editor/owner)
- `DELETE /{id}` — delete (editor/owner)
- `POST /{id}/cover` — upload cover image (reuse existing pattern)

## Frontend

- Accommodations section on trip detail page, below existing content
- Accommodation cards: name, dates, address, cover image
- Add/edit modal reusing existing Modal, Input, Button components
- Date display as "Feb 27 → Mar 3" format

## Authorization

Same as trips — TripAccess extractor. Viewers see, editors/owners create/edit/delete.
