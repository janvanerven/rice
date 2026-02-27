# Auto-Cover Image Service Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a service that automatically finds and applies cover images from Unsplash for trips, accommodations, and future activities when the user hasn't uploaded their own.

**Architecture:** Frontend detects missing covers and requests them on-demand via a shared `AutoCoverContext` (dedup + concurrency control). Backend searches the Unsplash API with a smart fallback chain, downloads and caches the image locally, and stores attribution metadata. Feature is opt-in via `UNSPLASH_ACCESS_KEY` env var.

**Tech Stack:** Rust/Axum backend with reqwest (already a dependency), Unsplash API, React context for frontend coordination.

**Design doc:** `docs/plans/2026-02-27-auto-cover-design.md`

---

### Task 1: Database Migration — Rename accommodation field + add attributions table

This migration renames `accommodations.cover_image_url` to `cover_image_path` for consistency with trips, and adds the `image_attributions` table.

**Files:**
- Create: `backend/migrations/0003_auto_cover.sql`

**Step 1: Write the migration**

```sql
-- Rename cover_image_url to cover_image_path for consistency with trips table.
-- SQLite doesn't support ALTER COLUMN RENAME, so we recreate the table.
CREATE TABLE accommodations_new (
    id TEXT PRIMARY KEY NOT NULL,
    trip_id TEXT NOT NULL REFERENCES trips(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    address TEXT,
    check_in TEXT,
    check_out TEXT,
    notes TEXT,
    cover_image_path TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

INSERT INTO accommodations_new (id, trip_id, name, address, check_in, check_out, notes, cover_image_path, created_at, updated_at)
SELECT id, trip_id, name, address, check_in, check_out, notes, cover_image_url, created_at, updated_at
FROM accommodations;

DROP TABLE accommodations;
ALTER TABLE accommodations_new RENAME TO accommodations;

-- Image attributions for auto-fetched cover images
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

**Step 2: Update the Rust model**

Modify: `backend/src/models.rs`

Change the `Accommodation` struct field from `cover_image_url` to `cover_image_path`:

```rust
// In struct Accommodation:
// Before:
pub cover_image_url: Option<String>,
// After:
pub cover_image_path: Option<String>,
```

Add the new `ImageAttribution` model and `AutoCoverResponse` after the existing models:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ImageAttribution {
    pub entity_type: String,
    pub entity_id: String,
    pub author_name: String,
    pub author_url: String,
    pub source_url: String,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribution {
    pub author_name: String,
    pub author_url: String,
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoCoverResponse {
    pub path: String,
    pub attribution: Attribution,
}
```

**Step 3: Update accommodation SQL queries**

Modify: `backend/src/api/accommodations.rs`

Find and replace all occurrences of `cover_image_url` with `cover_image_path` in SQL strings:

```
"UPDATE accommodations SET cover_image_url = ?1"
→
"UPDATE accommodations SET cover_image_path = ?1"
```

This affects `upload_accommodation_cover` only (the other queries use `SELECT *`).

**Step 4: Update the .sqlx offline cache**

Run: `cd backend && cargo sqlx prepare`

This regenerates the `.sqlx/` cache files for the new column name.

**Step 5: Update frontend types**

Modify: `frontend/src/types/index.ts`

In the `Accommodation` interface, rename:
```typescript
// Before:
cover_image_url: string | null
// After:
cover_image_path: string | null
```

Add new types:

```typescript
export interface Attribution {
  author_name: string
  author_url: string
  source_url: string
}

export interface AutoCoverResponse {
  path: string
  attribution: Attribution
}
```

**Step 6: Update frontend component references**

Modify: `frontend/src/components/trips/AccommodationList.tsx`

Replace `acc.cover_image_url` with `acc.cover_image_path` (2 occurrences: the truthy check and the `src` prop).

**Step 7: Verify the build compiles**

Run: `cd backend && cargo check`
Run: `cd frontend && npx tsc --noEmit`

**Step 8: Commit**

```bash
git add backend/migrations/0003_auto_cover.sql backend/src/models.rs backend/src/api/accommodations.rs frontend/src/types/index.ts frontend/src/components/trips/AccommodationList.tsx backend/.sqlx/
git commit -m "refactor: rename cover_image_url to cover_image_path, add image_attributions table"
```

---

### Task 2: Backend — Add Unsplash config

**Files:**
- Modify: `backend/src/config.rs`
- Modify: `backend/src/main.rs` (AppState)

**Step 1: Add unsplash_access_key to Config**

Modify: `backend/src/config.rs`

Add field to the `Config` struct:

```rust
pub unsplash_access_key: Option<String>,
```

Add to `Config::from_env()` in the `Ok(Config { ... })` block:

```rust
unsplash_access_key: env::var("UNSPLASH_ACCESS_KEY").ok(),
```

**Step 2: Verify**

Run: `cd backend && cargo check`

**Step 3: Commit**

```bash
git add backend/src/config.rs
git commit -m "feat: add UNSPLASH_ACCESS_KEY config option"
```

---

### Task 3: Backend — Unsplash service module

**Files:**
- Create: `backend/src/services/mod.rs`
- Create: `backend/src/services/unsplash.rs`
- Modify: `backend/src/main.rs` (add `mod services;`)

**Step 1: Create service module files**

Create `backend/src/services/mod.rs`:

```rust
pub mod unsplash;
```

Create `backend/src/services/unsplash.rs`:

```rust
use reqwest::Client;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;
use tokio::io::AsyncWriteExt;

const UNSPLASH_API: &str = "https://api.unsplash.com";
const ALLOWED_HOSTS: &[&str] = &["images.unsplash.com", "plus.unsplash.com"];
const MAX_DOWNLOAD_SIZE: usize = 5 * 1024 * 1024; // 5MB
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Deserialize)]
pub struct UnsplashSearchResponse {
    pub results: Vec<UnsplashPhoto>,
}

#[derive(Debug, Deserialize)]
pub struct UnsplashPhoto {
    pub id: String,
    pub urls: UnsplashUrls,
    pub user: UnsplashUser,
    pub links: UnsplashLinks,
}

#[derive(Debug, Deserialize)]
pub struct UnsplashUrls {
    pub regular: String,
}

#[derive(Debug, Deserialize)]
pub struct UnsplashUser {
    pub name: String,
    pub links: UnsplashUserLinks,
}

#[derive(Debug, Deserialize)]
pub struct UnsplashUserLinks {
    pub html: String,
}

#[derive(Debug, Deserialize)]
pub struct UnsplashLinks {
    pub html: String,
    pub download_location: String,
}

/// Search Unsplash for a landscape photo matching the query.
/// Returns None if no results found.
pub async fn search(
    client: &Client,
    access_key: &str,
    query: &str,
) -> Result<Option<UnsplashPhoto>, String> {
    let resp = client
        .get(format!("{UNSPLASH_API}/search/photos"))
        .header("Authorization", format!("Client-ID {access_key}"))
        .query(&[
            ("query", query),
            ("per_page", "1"),
            ("orientation", "landscape"),
        ])
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("Unsplash search failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        // Don't log the response body as it may contain the access key
        return Err(format!("Unsplash API returned {status}"));
    }

    let data: UnsplashSearchResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Unsplash response: {e}"))?;

    Ok(data.results.into_iter().next())
}

/// Download a photo to a temp file in the given directory.
/// Returns the temp file path on success.
/// The caller must rename the temp file after committing the DB transaction.
pub async fn download(
    client: &Client,
    photo: &UnsplashPhoto,
    dir: &Path,
) -> Result<(PathBuf, String), String> {
    // SSRF protection: verify the download URL points to an allowed host
    let url = &photo.urls.regular;
    let parsed = reqwest::Url::parse(url)
        .map_err(|e| format!("Invalid photo URL: {e}"))?;
    let host = parsed.host_str().unwrap_or("");
    if !ALLOWED_HOSTS.iter().any(|h| host == *h || host.ends_with(&format!(".{h}"))) {
        return Err(format!("Blocked download from untrusted host: {host}"));
    }

    // Download with size limit (streaming)
    let resp = client
        .get(url)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("Failed to download photo: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Photo download returned {}", resp.status()));
    }

    let filename = format!("{}.jpg", ulid::Ulid::new());
    let tmp_filename = format!("{filename}.tmp");
    let tmp_path = dir.join(&tmp_filename);
    let final_path = dir.join(&filename);

    fs::create_dir_all(dir)
        .await
        .map_err(|e| format!("Failed to create upload dir: {e}"))?;

    // Stream to temp file with size cap
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read photo data: {e}"))?;

    if bytes.len() > MAX_DOWNLOAD_SIZE {
        return Err("Downloaded photo exceeds 5MB size limit".into());
    }

    let mut file = fs::File::create(&tmp_path)
        .await
        .map_err(|e| {
            format!("Failed to create temp file: {e}")
        })?;

    file.write_all(&bytes)
        .await
        .map_err(|e| {
            // Try to clean up on failure
            let tmp = tmp_path.clone();
            tokio::spawn(async move { let _ = fs::remove_file(tmp).await; });
            format!("Failed to write photo: {e}")
        })?;

    file.flush().await.map_err(|e| format!("Failed to flush: {e}"))?;

    Ok((tmp_path, filename))
}

/// Rename temp file to final path. Call after DB commit.
pub async fn finalize(tmp_path: &Path, final_name: &str) -> Result<PathBuf, String> {
    let final_path = tmp_path.parent().unwrap().join(final_name);
    fs::rename(tmp_path, &final_path)
        .await
        .map_err(|e| format!("Failed to finalize file: {e}"))?;
    Ok(final_path)
}

/// Clean up a temp file on failure.
pub async fn cleanup_temp(tmp_path: &Path) {
    let _ = fs::remove_file(tmp_path).await;
}

/// Trigger Unsplash download tracking (required by TOS).
/// Fire-and-forget but log errors.
pub fn track_download(client: Client, access_key: String, photo: &UnsplashPhoto) {
    let url = photo.links.download_location.clone();
    tokio::spawn(async move {
        let result = client
            .get(&url)
            .header("Authorization", format!("Client-ID {access_key}"))
            .timeout(Duration::from_secs(5))
            .send()
            .await;
        match result {
            Ok(resp) if resp.status().is_success() => {}
            Ok(resp) => {
                tracing::error!("Unsplash download tracking returned {}", resp.status());
            }
            Err(e) => {
                tracing::error!("Unsplash download tracking failed: {e}");
            }
        }
    });
}
```

**Step 2: Register the services module**

Modify: `backend/src/main.rs`

Add `mod services;` to the module declarations at the top (after `mod models;`).

**Step 3: Verify**

Run: `cd backend && cargo check`

**Step 4: Commit**

```bash
git add backend/src/services/ backend/src/main.rs
git commit -m "feat: add Unsplash service module with search, download, and tracking"
```

---

### Task 4: Backend — Auto-cover API endpoints

**Files:**
- Create: `backend/src/api/auto_cover.rs`
- Modify: `backend/src/api/mod.rs` (register module + routes)

**Step 1: Create the auto-cover handler module**

Create `backend/src/api/auto_cover.rs`:

```rust
use axum::{
    extract::{Path, State},
    Json,
};

use crate::{
    errors::AppError,
    extractors::TripAccess,
    models::{Accommodation, Attribution, AutoCoverResponse, Trip},
    services::unsplash,
    AppState,
};

/// POST /api/trips/{trip_id}/auto-cover
pub async fn auto_cover_trip(
    State(state): State<AppState>,
    access: TripAccess,
) -> Result<Json<AutoCoverResponse>, AppError> {
    access.require_editor()?;

    let access_key = state
        .config
        .unsplash_access_key
        .as_deref()
        .ok_or_else(|| AppError::Internal("Auto-cover service not configured".into()))?;

    // Idempotency: if cover already set, return early
    let trip: Trip = sqlx::query_as("SELECT * FROM trips WHERE id = ?1")
        .bind(&access.trip_id)
        .fetch_one(&state.db.read)
        .await?;

    if trip.cover_image_path.is_some() {
        return Err(AppError::BadRequest("Trip already has a cover image".into()));
    }

    // Rate limit: check if we tried recently
    let recent: Option<String> = sqlx::query_scalar(
        "SELECT fetched_at FROM image_attributions \
         WHERE entity_type = 'trip' AND entity_id = ?1 \
         AND fetched_at > strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-10 minutes')",
    )
    .bind(&access.trip_id)
    .fetch_optional(&state.db.read)
    .await?;

    if recent.is_some() {
        return Err(AppError::BadRequest("Auto-cover was attempted recently, please wait".into()));
    }

    // Smart fallback: search for destination
    let query = trip.destination.trim().to_string();
    if query.is_empty() {
        return Err(AppError::NotFound("No destination set for image search".into()));
    }

    let client = reqwest::Client::new();
    let photo = unsplash::search(&client, access_key, &query)
        .await
        .map_err(|e| AppError::Internal(e))?
        .ok_or_else(|| AppError::NotFound("No matching image found".into()))?;

    // Download to temp file (outside transaction)
    let upload_base = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "/data/uploads".into());
    let dir = std::path::PathBuf::from(&upload_base).join(&access.trip_id);
    let (tmp_path, filename) = unsplash::download(&client, &photo, &dir)
        .await
        .map_err(|e| AppError::Internal(e))?;

    let relative_path = format!("/uploads/{}/{filename}", access.trip_id);

    // DB transaction: update cover + upsert attribution
    let mut tx = state.db.write.begin().await?;

    // Double-check cover_image_path is still NULL (race protection)
    let still_null: bool = sqlx::query_scalar(
        "SELECT cover_image_path IS NULL FROM trips WHERE id = ?1",
    )
    .bind(&access.trip_id)
    .fetch_one(&mut *tx)
    .await?;

    if !still_null {
        drop(tx);
        unsplash::cleanup_temp(&tmp_path).await;
        return Err(AppError::BadRequest("Trip already has a cover image".into()));
    }

    sqlx::query(
        "UPDATE trips SET cover_image_path = ?1, updated_at = datetime('now') WHERE id = ?2",
    )
    .bind(&relative_path)
    .bind(&access.trip_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO image_attributions (entity_type, entity_id, author_name, author_url, source_url) \
         VALUES ('trip', ?1, ?2, ?3, ?4) \
         ON CONFLICT(entity_type, entity_id) DO UPDATE SET \
         author_name = excluded.author_name, author_url = excluded.author_url, \
         source_url = excluded.source_url, fetched_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')",
    )
    .bind(&access.trip_id)
    .bind(&photo.user.name)
    .bind(&photo.user.links.html)
    .bind(&photo.links.html)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // Finalize: rename temp file to final name
    if let Err(e) = unsplash::finalize(&tmp_path, &filename).await {
        tracing::error!("Failed to finalize auto-cover file: {e}");
    }

    // Track download (Unsplash TOS, fire-and-forget)
    unsplash::track_download(client, access_key.to_string(), &photo);

    Ok(Json(AutoCoverResponse {
        path: relative_path,
        attribution: Attribution {
            author_name: photo.user.name,
            author_url: photo.user.links.html,
            source_url: photo.links.html,
        },
    }))
}

/// Path params for accommodation auto-cover
#[derive(serde::Deserialize)]
pub struct AccommodationAutoPath {
    pub trip_id: String,
    pub accommodation_id: String,
}

/// POST /api/trips/{trip_id}/accommodations/{accommodation_id}/auto-cover
pub async fn auto_cover_accommodation(
    State(state): State<AppState>,
    access: TripAccess,
    Path(path): Path<AccommodationAutoPath>,
) -> Result<Json<AutoCoverResponse>, AppError> {
    access.require_editor()?;

    let access_key = state
        .config
        .unsplash_access_key
        .as_deref()
        .ok_or_else(|| AppError::Internal("Auto-cover service not configured".into()))?;

    // Load accommodation + trip
    let acc: Accommodation = sqlx::query_as(
        "SELECT * FROM accommodations WHERE id = ?1 AND trip_id = ?2",
    )
    .bind(&path.accommodation_id)
    .bind(&access.trip_id)
    .fetch_optional(&state.db.read)
    .await?
    .ok_or_else(|| AppError::NotFound("Accommodation not found".into()))?;

    if acc.cover_image_path.is_some() {
        return Err(AppError::BadRequest("Accommodation already has a cover image".into()));
    }

    // Rate limit
    let recent: Option<String> = sqlx::query_scalar(
        "SELECT fetched_at FROM image_attributions \
         WHERE entity_type = 'accommodation' AND entity_id = ?1 \
         AND fetched_at > strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-10 minutes')",
    )
    .bind(&path.accommodation_id)
    .fetch_optional(&state.db.read)
    .await?;

    if recent.is_some() {
        return Err(AppError::BadRequest("Auto-cover was attempted recently, please wait".into()));
    }

    let trip: Trip = sqlx::query_as("SELECT * FROM trips WHERE id = ?1")
        .bind(&access.trip_id)
        .fetch_one(&state.db.read)
        .await?;

    // Smart fallback chain: name → name+address → trip destination
    let client = reqwest::Client::new();
    let queries = build_accommodation_queries(&acc, &trip);
    let mut photo = None;

    for query in &queries {
        if query.is_empty() {
            continue;
        }
        match unsplash::search(&client, access_key, query).await {
            Ok(Some(p)) => {
                photo = Some(p);
                break;
            }
            Ok(None) => continue,
            Err(e) => {
                tracing::warn!("Unsplash search failed for '{query}': {e}");
                continue;
            }
        }
    }

    let photo = photo.ok_or_else(|| AppError::NotFound("No matching image found".into()))?;

    // Download to temp file
    let upload_base = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "/data/uploads".into());
    let dir = std::path::PathBuf::from(&upload_base).join(&access.trip_id);
    let (tmp_path, filename) = unsplash::download(&client, &photo, &dir)
        .await
        .map_err(|e| AppError::Internal(e))?;

    let relative_path = format!("/uploads/{}/{filename}", access.trip_id);

    // DB transaction
    let mut tx = state.db.write.begin().await?;

    let still_null: bool = sqlx::query_scalar(
        "SELECT cover_image_path IS NULL FROM accommodations WHERE id = ?1",
    )
    .bind(&path.accommodation_id)
    .fetch_one(&mut *tx)
    .await?;

    if !still_null {
        drop(tx);
        unsplash::cleanup_temp(&tmp_path).await;
        return Err(AppError::BadRequest("Accommodation already has a cover image".into()));
    }

    sqlx::query(
        "UPDATE accommodations SET cover_image_path = ?1, updated_at = datetime('now') WHERE id = ?2",
    )
    .bind(&relative_path)
    .bind(&path.accommodation_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO image_attributions (entity_type, entity_id, author_name, author_url, source_url) \
         VALUES ('accommodation', ?1, ?2, ?3, ?4) \
         ON CONFLICT(entity_type, entity_id) DO UPDATE SET \
         author_name = excluded.author_name, author_url = excluded.author_url, \
         source_url = excluded.source_url, fetched_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')",
    )
    .bind(&path.accommodation_id)
    .bind(&photo.user.name)
    .bind(&photo.user.links.html)
    .bind(&photo.links.html)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    if let Err(e) = unsplash::finalize(&tmp_path, &filename).await {
        tracing::error!("Failed to finalize auto-cover file: {e}");
    }

    unsplash::track_download(client, access_key.to_string(), &photo);

    Ok(Json(AutoCoverResponse {
        path: relative_path,
        attribution: Attribution {
            author_name: photo.user.name,
            author_url: photo.user.links.html,
            source_url: photo.links.html,
        },
    }))
}

/// Build the fallback search query list for an accommodation.
fn build_accommodation_queries(acc: &Accommodation, trip: &Trip) -> Vec<String> {
    let mut queries = Vec::new();

    // 1. Accommodation name
    let name = acc.name.trim().to_string();
    if !name.is_empty() {
        queries.push(name.clone());
    }

    // 2. Name + address
    if let Some(addr) = &acc.address {
        let combined = format!("{} {}", name, addr.trim());
        if combined.trim().len() > name.len() {
            queries.push(combined);
        }
    }

    // 3. Trip destination
    let dest = trip.destination.trim().to_string();
    if !dest.is_empty() {
        queries.push(dest);
    }

    queries
}
```

**Step 2: Register the module and routes**

Modify: `backend/src/api/mod.rs`

Add at the top with other module declarations:
```rust
pub mod auto_cover;
```

Add these routes to the `router()` function:
```rust
.route(
    "/api/trips/{trip_id}/auto-cover",
    post(auto_cover::auto_cover_trip),
)
.route(
    "/api/trips/{trip_id}/accommodations/{accommodation_id}/auto-cover",
    post(auto_cover::auto_cover_accommodation),
)
```

**Step 3: Update .sqlx offline cache**

Run: `cd backend && cargo sqlx prepare`

**Step 4: Verify**

Run: `cd backend && cargo check`

**Step 5: Commit**

```bash
git add backend/src/api/auto_cover.rs backend/src/api/mod.rs backend/.sqlx/
git commit -m "feat: add auto-cover API endpoints for trips and accommodations"
```

---

### Task 5: Backend — Bundle attribution in API responses + clean up on manual upload

**Files:**
- Modify: `backend/src/api/trips.rs`
- Modify: `backend/src/api/accommodations.rs`
- Modify: `backend/src/api/uploads.rs`
- Modify: `backend/src/models.rs`

**Step 1: Add response types with attribution**

Modify: `backend/src/models.rs`

Add after existing types:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TripWithRoleAndAttribution {
    #[serde(flatten)]
    pub trip: Trip,
    pub role: String,
    pub attribution: Option<Attribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccommodationWithAttribution {
    #[serde(flatten)]
    pub accommodation: Accommodation,
    pub attribution: Option<Attribution>,
}
```

**Step 2: Update trip list and detail to include attribution**

Modify: `backend/src/api/trips.rs`

Update the import to include new types:
```rust
use crate::{
    errors::AppError,
    extractors::{AuthUser, TripAccess},
    models::{Attribution, CreateTripRequest, Trip, TripWithRole, TripWithRoleAndAttribution, UpdateTripRequest},
    AppState,
};
```

Update `list_trips` return type from `Json<Vec<TripWithRole>>` to `Json<Vec<TripWithRoleAndAttribution>>`. After building the `trips` vec, load attributions:

```rust
pub async fn list_trips(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Result<Json<Vec<TripWithRoleAndAttribution>>, AppError> {
    let rows: Vec<(
        String, String, String, Option<String>, Option<String>,
        Option<String>, String, String, String, String,
    )> = sqlx::query_as(
        "SELECT t.id, t.name, t.destination, t.start_date, t.end_date, \
         t.cover_image_path, t.created_by, t.created_at, t.updated_at, tm.role \
         FROM trips t \
         JOIN trip_members tm ON tm.trip_id = t.id \
         WHERE tm.user_id = ?1 \
         ORDER BY t.created_at DESC",
    )
    .bind(&user.id)
    .fetch_all(&state.db.read)
    .await?;

    let trip_ids: Vec<String> = rows.iter().map(|r| r.0.clone()).collect();

    // Batch load attributions for all trips
    let attributions: Vec<(String, String, String, String)> = if trip_ids.is_empty() {
        vec![]
    } else {
        let placeholders: String = trip_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            "SELECT entity_id, author_name, author_url, source_url \
             FROM image_attributions WHERE entity_type = 'trip' AND entity_id IN ({placeholders})"
        );
        let mut q = sqlx::query_as(&query);
        for id in &trip_ids {
            q = q.bind(id);
        }
        q.fetch_all(&state.db.read).await?
    };

    let attr_map: std::collections::HashMap<String, Attribution> = attributions
        .into_iter()
        .map(|(id, name, url, source)| {
            (id, Attribution {
                author_name: name,
                author_url: url,
                source_url: source,
            })
        })
        .collect();

    let trips = rows
        .into_iter()
        .map(|row| {
            let id = row.0.clone();
            TripWithRoleAndAttribution {
                trip: Trip {
                    id: row.0,
                    name: row.1,
                    destination: row.2,
                    start_date: row.3,
                    end_date: row.4,
                    cover_image_path: row.5,
                    created_by: row.6,
                    created_at: row.7,
                    updated_at: row.8,
                },
                role: row.9,
                attribution: attr_map.get(&id).cloned(),
            }
        })
        .collect();

    Ok(Json(trips))
}
```

Update `get_trip` similarly — change return type to `Json<TripWithRoleAndAttribution>` and add attribution lookup:

```rust
pub async fn get_trip(
    State(state): State<AppState>,
    access: TripAccess,
) -> Result<Json<TripWithRoleAndAttribution>, AppError> {
    let trip: Trip = sqlx::query_as("SELECT * FROM trips WHERE id = ?1")
        .bind(&access.trip_id)
        .fetch_one(&state.db.read)
        .await?;

    let attribution: Option<(String, String, String)> = sqlx::query_as(
        "SELECT author_name, author_url, source_url FROM image_attributions \
         WHERE entity_type = 'trip' AND entity_id = ?1",
    )
    .bind(&access.trip_id)
    .fetch_optional(&state.db.read)
    .await?;

    Ok(Json(TripWithRoleAndAttribution {
        trip,
        role: access.role,
        attribution: attribution.map(|(name, url, source)| Attribution {
            author_name: name,
            author_url: url,
            source_url: source,
        }),
    }))
}
```

**Step 3: Update accommodation list to include attribution**

Modify: `backend/src/api/accommodations.rs`

Update imports to include `Attribution` and `AccommodationWithAttribution`.

Update `list_accommodations` to return `Json<Vec<AccommodationWithAttribution>>`:

```rust
pub async fn list_accommodations(
    State(state): State<AppState>,
    access: TripAccess,
) -> Result<Json<Vec<AccommodationWithAttribution>>, AppError> {
    let rows: Vec<Accommodation> = sqlx::query_as(
        "SELECT * FROM accommodations WHERE trip_id = ?1 \
         ORDER BY check_in IS NULL ASC, check_in ASC, created_at ASC",
    )
    .bind(&access.trip_id)
    .fetch_all(&state.db.read)
    .await?;

    let acc_ids: Vec<String> = rows.iter().map(|a| a.id.clone()).collect();

    let attributions: Vec<(String, String, String, String)> = if acc_ids.is_empty() {
        vec![]
    } else {
        let placeholders: String = acc_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            "SELECT entity_id, author_name, author_url, source_url \
             FROM image_attributions WHERE entity_type = 'accommodation' AND entity_id IN ({placeholders})"
        );
        let mut q = sqlx::query_as(&query);
        for id in &acc_ids {
            q = q.bind(id);
        }
        q.fetch_all(&state.db.read).await?
    };

    let attr_map: std::collections::HashMap<String, Attribution> = attributions
        .into_iter()
        .map(|(id, name, url, source)| {
            (id, Attribution {
                author_name: name,
                author_url: url,
                source_url: source,
            })
        })
        .collect();

    let result = rows
        .into_iter()
        .map(|acc| {
            let id = acc.id.clone();
            AccommodationWithAttribution {
                accommodation: acc,
                attribution: attr_map.get(&id).cloned(),
            }
        })
        .collect();

    Ok(Json(result))
}
```

**Step 4: Delete attribution on manual cover upload**

Modify: `backend/src/api/uploads.rs`

After the `UPDATE trips SET cover_image_path` query, add:

```rust
// Remove auto-cover attribution if user uploaded their own
sqlx::query(
    "DELETE FROM image_attributions WHERE entity_type = 'trip' AND entity_id = ?1",
)
.bind(&access.trip_id)
.execute(&state.db.write)
.await?;
```

Modify: `backend/src/api/accommodations.rs` — in `upload_accommodation_cover`

After the `UPDATE accommodations SET cover_image_path` query, add:

```rust
// Remove auto-cover attribution if user uploaded their own
sqlx::query(
    "DELETE FROM image_attributions WHERE entity_type = 'accommodation' AND entity_id = ?1",
)
.bind(&path.accommodation_id)
.execute(&state.db.write)
.await?;
```

**Step 5: Update .sqlx offline cache**

Run: `cd backend && cargo sqlx prepare`

**Step 6: Verify**

Run: `cd backend && cargo check`

**Step 7: Commit**

```bash
git add backend/src/api/ backend/src/models.rs backend/.sqlx/
git commit -m "feat: bundle attribution in trip/accommodation responses, clean up on manual upload"
```

---

### Task 6: Frontend — API client methods for auto-cover

**Files:**
- Modify: `frontend/src/lib/api.ts`
- Modify: `frontend/src/types/index.ts`

**Step 1: Update Trip type to include attribution**

Modify: `frontend/src/types/index.ts`

Add `attribution` to the `Trip` interface:

```typescript
export interface Trip {
  id: string
  name: string
  destination: string
  start_date: string | null
  end_date: string | null
  cover_image_path: string | null
  created_by: string
  created_at: string
  updated_at: string
  role: string
  attribution: Attribution | null
}
```

Add to `Accommodation` interface:

```typescript
export interface Accommodation {
  id: string
  trip_id: string
  name: string
  address: string | null
  check_in: string | null
  check_out: string | null
  notes: string | null
  cover_image_path: string | null
  created_at: string
  updated_at: string
  attribution: Attribution | null
}
```

**Step 2: Add auto-cover API methods**

Modify: `frontend/src/lib/api.ts`

Add import for new types:
```typescript
import type { ..., AutoCoverResponse } from '../types'
```

Add to the `trips` namespace in the `api` object:

```typescript
autoCover: (tripId: string) =>
  request<AutoCoverResponse>(`/api/trips/${tripId}/auto-cover`, { method: 'POST' }),
```

Add to the `accommodations` namespace:

```typescript
autoCover: (tripId: string, id: string) =>
  request<AutoCoverResponse>(`/api/trips/${tripId}/accommodations/${id}/auto-cover`, { method: 'POST' }),
```

**Step 3: Verify**

Run: `cd frontend && npx tsc --noEmit`

**Step 4: Commit**

```bash
git add frontend/src/types/index.ts frontend/src/lib/api.ts
git commit -m "feat: add auto-cover types and API client methods"
```

---

### Task 7: Frontend — AutoCoverContext

**Files:**
- Create: `frontend/src/components/AutoCoverContext.tsx`
- Modify: `frontend/src/App.tsx` (wrap with provider)

**Step 1: Create the AutoCoverContext**

Create `frontend/src/components/AutoCoverContext.tsx`:

```typescript
import { createContext, useContext, useRef, useCallback } from 'react'
import { api } from '../lib/api'
import type { AutoCoverResponse } from '../types'

type EntityType = 'trip' | 'accommodation'

interface AutoCoverRequest {
  entityType: EntityType
  entityId: string
  tripId: string
  onSuccess: (result: AutoCoverResponse) => void
}

interface AutoCoverContextType {
  requestAutoCover: (req: AutoCoverRequest) => void
}

const AutoCoverContext = createContext<AutoCoverContextType>({
  requestAutoCover: () => {},
})

export function useAutoCover() {
  return useContext(AutoCoverContext)
}

const MAX_CONCURRENT = 2

export function AutoCoverProvider({ children }: { children: React.ReactNode }) {
  // Using refs to survive StrictMode double-mount and avoid stale closures
  const inFlightRef = useRef(new Set<string>())
  const attemptedRef = useRef(new Set<string>())
  const queueRef = useRef<AutoCoverRequest[]>([])
  const activeCountRef = useRef(0)

  const processQueue = useCallback(() => {
    while (activeCountRef.current < MAX_CONCURRENT && queueRef.current.length > 0) {
      const req = queueRef.current.shift()!
      executeRequest(req)
    }
  }, [])

  const executeRequest = useCallback((req: AutoCoverRequest) => {
    const key = `${req.entityType}:${req.entityId}`
    activeCountRef.current++
    inFlightRef.current.add(key)

    const promise =
      req.entityType === 'trip'
        ? api.trips.autoCover(req.tripId)
        : api.accommodations.autoCover(req.tripId, req.entityId)

    promise
      .then((result) => {
        req.onSuccess(result)
      })
      .catch(() => {
        // Silent failure — no toast for background operations
      })
      .finally(() => {
        inFlightRef.current.delete(key)
        attemptedRef.current.add(key)
        activeCountRef.current--
        processQueue()
      })
  }, [processQueue])

  const requestAutoCover = useCallback((req: AutoCoverRequest) => {
    const key = `${req.entityType}:${req.entityId}`

    // Deduplicate: skip if already attempted or in flight
    if (attemptedRef.current.has(key) || inFlightRef.current.has(key)) {
      return
    }

    if (activeCountRef.current < MAX_CONCURRENT) {
      executeRequest(req)
    } else {
      queueRef.current.push(req)
    }
  }, [executeRequest])

  return (
    <AutoCoverContext.Provider value={{ requestAutoCover }}>
      {children}
    </AutoCoverContext.Provider>
  )
}
```

**Step 2: Wrap the app with AutoCoverProvider**

Modify: `frontend/src/App.tsx`

Add import:
```typescript
import { AutoCoverProvider } from './components/AutoCoverContext'
```

Wrap the `<Routes>` inside `AuthProvider` with `<AutoCoverProvider>`:

```tsx
<AuthProvider>
  <AutoCoverProvider>
    <Routes>
      ...
    </Routes>
  </AutoCoverProvider>
</AuthProvider>
```

**Step 3: Verify**

Run: `cd frontend && npx tsc --noEmit`

**Step 4: Commit**

```bash
git add frontend/src/components/AutoCoverContext.tsx frontend/src/App.tsx
git commit -m "feat: add AutoCoverContext with dedup and concurrency control"
```

---

### Task 8: Frontend — Shimmer animation CSS

**Files:**
- Modify: `frontend/src/globals.css`

**Step 1: Add shimmer keyframes and class**

Modify: `frontend/src/globals.css`

Add after the existing `@keyframes` block (before `.page-enter`):

```css
@keyframes shimmer {
  0%   { background-position: -200% 0; }
  100% { background-position: 200% 0; }
}

.cover-shimmer {
  background: linear-gradient(
    90deg,
    var(--color-surface-2) 25%,
    var(--color-surface-3) 50%,
    var(--color-surface-2) 75%
  );
  background-size: 200% 100%;
  animation: shimmer 1.5s ease-in-out infinite;
}
```

**Step 2: Commit**

```bash
git add frontend/src/globals.css
git commit -m "feat: add shimmer animation for auto-cover loading state"
```

---

### Task 9: Frontend — Attribution overlay component

**Files:**
- Create: `frontend/src/components/ui/Attribution/Attribution.tsx`
- Create: `frontend/src/components/ui/Attribution/Attribution.module.css`
- Modify: `frontend/src/components/ui/index.ts` (export)

**Step 1: Create the Attribution component**

Create `frontend/src/components/ui/Attribution/Attribution.tsx`:

```typescript
import type { Attribution as AttributionType } from '../../../types'
import styles from './Attribution.module.css'

interface AttributionProps {
  attribution: AttributionType
}

export function Attribution({ attribution }: AttributionProps) {
  return (
    <div className={styles.container} aria-hidden="true">
      <span className={styles.text}>
        Photo by{' '}
        <a
          href={`${attribution.author_url}?utm_source=rice&utm_medium=referral`}
          target="_blank"
          rel="noopener noreferrer"
          className={styles.link}
          onClick={(e) => e.stopPropagation()}
          aria-label={`Photo by ${attribution.author_name} on Unsplash`}
        >
          {attribution.author_name}
        </a>
        {' / '}
        <a
          href="https://unsplash.com/?utm_source=rice&utm_medium=referral"
          target="_blank"
          rel="noopener noreferrer"
          className={styles.link}
          onClick={(e) => e.stopPropagation()}
        >
          Unsplash
        </a>
      </span>
    </div>
  )
}
```

**Step 2: Create the CSS module**

Create `frontend/src/components/ui/Attribution/Attribution.module.css`:

```css
.container {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  padding: var(--space-1) var(--space-3);
  /* Gradient scrim for guaranteed contrast */
  background: linear-gradient(
    to top,
    rgba(0, 0, 0, 0.6) 0%,
    rgba(0, 0, 0, 0.3) 60%,
    transparent 100%
  );
  pointer-events: auto;
  z-index: 1;
}

.text {
  font-family: var(--font-mono);
  font-size: 0.55rem;
  letter-spacing: var(--tracking-wide);
  color: rgba(240, 237, 232, 0.6);
}

.link {
  color: rgba(240, 237, 232, 0.7);
  text-decoration: none;
  transition: color var(--transition-fast);
  /* Make links clickable even inside cards */
  position: relative;
  z-index: 2;
}

.link:hover {
  color: rgba(240, 237, 232, 0.95);
  text-decoration: underline;
}
```

**Step 3: Export from ui index**

Modify: `frontend/src/components/ui/index.ts`

Add:
```typescript
export { Attribution } from './Attribution/Attribution'
```

**Step 4: Verify**

Run: `cd frontend && npx tsc --noEmit`

**Step 5: Commit**

```bash
git add frontend/src/components/ui/Attribution/ frontend/src/components/ui/index.ts
git commit -m "feat: add Attribution overlay component"
```

---

### Task 10: Frontend — Integrate auto-cover into TripCard

**Files:**
- Modify: `frontend/src/components/trips/TripCard.tsx`
- Modify: `frontend/src/components/trips/TripCard.module.css`

**Step 1: Add auto-cover trigger to TripCard**

Modify: `frontend/src/components/trips/TripCard.tsx`

Add imports:
```typescript
import { useEffect, useState } from 'react'
import { useAuth } from '../../App'
import { useAutoCover } from '../AutoCoverContext'
import { Attribution as AttributionOverlay } from '../ui'
import type { Attribution } from '../../types'
```

Update the component to trigger auto-cover and display attribution:

```typescript
export function TripCard({ trip }: TripCardProps) {
  const { user } = useAuth()
  const { requestAutoCover } = useAutoCover()
  const [coverPath, setCoverPath] = useState(trip.cover_image_path)
  const [attribution, setAttribution] = useState<Attribution | null>(trip.attribution ?? null)
  const [loading, setLoading] = useState(false)

  const dateRange = formatDateRange(trip.start_date, trip.end_date)
  const hasDates = trip.start_date || trip.end_date
  const coverUrl = coverPath ? `/api/uploads${coverPath}` : null

  const canEdit = ['owner', 'editor'].includes(trip.role.toLowerCase())

  // Trigger auto-cover for trips without a cover image
  useEffect(() => {
    if (coverPath || !canEdit || !trip.destination?.trim()) return
    setLoading(true)
    requestAutoCover({
      entityType: 'trip',
      entityId: trip.id,
      tripId: trip.id,
      onSuccess: (result) => {
        setCoverPath(result.path)
        setAttribution(result.attribution)
        setLoading(false)
      },
    })
    // If request is deduped, clear loading after a short delay
    const timeout = setTimeout(() => setLoading(false), 15000)
    return () => clearTimeout(timeout)
  }, [trip.id, coverPath, canEdit, trip.destination, requestAutoCover])

  return (
    <Link to={`/trips/${trip.id}`} className={styles.link} aria-label={`Open trip: ${trip.name}`}>
      <article className={styles.card}>
        <div className={styles.cover}>
          {coverUrl ? (
            <img
              src={coverUrl}
              alt={`Cover for ${trip.name}`}
              className={styles.coverImage}
            />
          ) : loading ? (
            <div className={`${styles.coverPlaceholder} cover-shimmer`} aria-hidden="true" />
          ) : (
            <div className={styles.coverPlaceholder} aria-hidden="true" />
          )}
          <div className={styles.coverOverlay} aria-hidden="true" />
          {attribution && coverUrl && <AttributionOverlay attribution={attribution} />}
          <h2 className={styles.tripName}>{trip.name}</h2>
        </div>

        <div className={styles.body}>
          {trip.destination && (
            <p className={styles.destination}>{trip.destination}</p>
          )}
          <p className={hasDates ? styles.dates : styles.datesEmpty}>
            {dateRange}
          </p>
          <div className={styles.footer}>
            <Badge variant={roleBadgeVariant(trip.role)}>
              {roleLabel(trip.role)}
            </Badge>
          </div>
        </div>
      </article>
    </Link>
  )
}
```

**Step 2: Lock cover aspect ratio**

Modify: `frontend/src/components/trips/TripCard.module.css`

The `.cover` class already has `height: 140px` which prevents layout shift. No change needed.

**Step 3: Verify**

Run: `cd frontend && npx tsc --noEmit`

**Step 4: Commit**

```bash
git add frontend/src/components/trips/TripCard.tsx
git commit -m "feat: integrate auto-cover into TripCard with shimmer loading"
```

---

### Task 11: Frontend — Integrate auto-cover into TripDetail

**Files:**
- Modify: `frontend/src/components/trips/TripDetail.tsx`

**Step 1: Add auto-cover trigger to TripDetail**

Modify: `frontend/src/components/trips/TripDetail.tsx`

Add imports:
```typescript
import { useAutoCover } from '../AutoCoverContext'
import { Attribution as AttributionOverlay } from '../ui'
import type { Attribution } from '../../types'
```

Add state and effect inside the component:

```typescript
const [autoCoverPath, setAutoCoverPath] = useState<string | null>(null)
const [autoCoverAttribution, setAutoCoverAttribution] = useState<Attribution | null>(null)
const [autoCoverLoading, setAutoCoverLoading] = useState(false)
const { requestAutoCover } = useAutoCover()

// Use auto-cover path if available, otherwise use trip's cover
const effectiveCoverUrl = trip.cover_image_path || autoCoverPath
const coverUrl = effectiveCoverUrl ? `/api/uploads${effectiveCoverUrl}` : null
const attribution = trip.attribution ?? autoCoverAttribution

useEffect(() => {
  if (trip.cover_image_path || !canEdit || !trip.destination?.trim()) return
  setAutoCoverLoading(true)
  requestAutoCover({
    entityType: 'trip',
    entityId: trip.id,
    tripId: trip.id,
    onSuccess: (result) => {
      setAutoCoverPath(result.path)
      setAutoCoverAttribution(result.attribution)
      setAutoCoverLoading(false)
    },
  })
  const timeout = setTimeout(() => setAutoCoverLoading(false), 15000)
  return () => clearTimeout(timeout)
}, [trip.id, trip.cover_image_path, canEdit, trip.destination, requestAutoCover])
```

Remove the old `const coverUrl = trip.cover_image_path || null` line.

Update the hero JSX to include shimmer and attribution:

```tsx
{/* ---- Hero cover ---- */}
<div className={styles.hero}>
  {coverUrl ? (
    <img
      src={coverUrl}
      alt={`Cover for ${trip.name}`}
      className={styles.heroImage}
    />
  ) : autoCoverLoading ? (
    <div className={`${styles.heroPlaceholder} cover-shimmer`} aria-hidden="true" />
  ) : (
    <div className={styles.heroPlaceholder} aria-hidden="true" />
  )}
  <div className={styles.heroOverlay} aria-hidden="true" />

  {attribution && coverUrl && <AttributionOverlay attribution={attribution} />}

  {/* Badge chips overlaid on hero */}
  ...rest unchanged
</div>
```

**Step 2: Verify**

Run: `cd frontend && npx tsc --noEmit`

**Step 3: Commit**

```bash
git add frontend/src/components/trips/TripDetail.tsx
git commit -m "feat: integrate auto-cover into TripDetail hero image"
```

---

### Task 12: Frontend — Integrate auto-cover into AccommodationList

**Files:**
- Modify: `frontend/src/components/trips/AccommodationList.tsx`

**Step 1: Add auto-cover to AccommodationList**

Modify: `frontend/src/components/trips/AccommodationList.tsx`

Add imports:
```typescript
import { useAutoCover } from '../AutoCoverContext'
import { Attribution as AttributionOverlay } from '../ui'
```

Create a child component `AccommodationCard` to encapsulate per-item auto-cover logic. Replace the inline `<li>` rendering with this component. Add it inside the same file, before `AccommodationList`:

```typescript
import type { Attribution, Accommodation, CreateAccommodationRequest } from '../../types'

interface AccommodationCardProps {
  acc: Accommodation
  tripId: string
  canEdit: boolean
  uploadingId: string | null
  deletingId: string | null
  onCoverClick: (id: string) => void
  onEdit: (id: string) => void
  onDelete: (id: string, name: string) => void
}

function AccommodationCard({
  acc, tripId, canEdit, uploadingId, deletingId,
  onCoverClick, onEdit, onDelete,
}: AccommodationCardProps) {
  const { requestAutoCover } = useAutoCover()
  const [coverPath, setCoverPath] = useState(acc.cover_image_path)
  const [attribution, setAttribution] = useState<Attribution | null>(acc.attribution ?? null)
  const [loading, setLoading] = useState(false)

  const coverUrl = coverPath ? `/api/uploads${coverPath}` : null

  useEffect(() => {
    if (coverPath || !canEdit) return
    setLoading(true)
    requestAutoCover({
      entityType: 'accommodation',
      entityId: acc.id,
      tripId,
      onSuccess: (result) => {
        setCoverPath(result.path)
        setAttribution(result.attribution)
        setLoading(false)
      },
    })
    const timeout = setTimeout(() => setLoading(false), 15000)
    return () => clearTimeout(timeout)
  }, [acc.id, coverPath, canEdit, tripId, requestAutoCover])

  return (
    <li className={styles.card}>
      <div className={styles.cardCover}>
        {coverUrl ? (
          <img
            src={coverUrl}
            alt={`Cover for ${acc.name}`}
            className={styles.cardCoverImage}
          />
        ) : loading ? (
          <div className={`${styles.cardCoverPlaceholder} cover-shimmer`} aria-hidden="true" />
        ) : (
          <div className={styles.cardCoverPlaceholder} aria-hidden="true" />
        )}
        <div className={styles.cardCoverOverlay} aria-hidden="true" />
        {attribution && coverUrl && <AttributionOverlay attribution={attribution} />}
        {canEdit && (
          <button
            className={styles.cardCoverBtn}
            onClick={() => onCoverClick(acc.id)}
            disabled={uploadingId === acc.id}
            title="Change cover"
            aria-label={uploadingId === acc.id ? `Uploading cover for ${acc.name}` : `Change cover for ${acc.name}`}
          >
            {uploadingId === acc.id ? '...' : '+'}
          </button>
        )}
      </div>

      <div className={styles.cardBody}>
        <p className={styles.cardName}>{acc.name}</p>
        {(acc.check_in || acc.check_out) && (
          <p className={styles.cardDates}>
            {formatDateRange(acc.check_in, acc.check_out)}
          </p>
        )}
        {acc.address && <p className={styles.cardAddress}>{acc.address}</p>}
        {acc.notes && <p className={styles.cardNotes}>{acc.notes}</p>}
        {canEdit && (
          <div className={styles.cardActions}>
            <Button variant="ghost" size="sm" onClick={() => onEdit(acc.id)} aria-label={`Edit ${acc.name}`}>
              Edit
            </Button>
            <Button
              variant="ghost" size="sm"
              onClick={() => onDelete(acc.id, acc.name)}
              disabled={deletingId === acc.id}
              aria-label={deletingId === acc.id ? `Deleting ${acc.name}` : `Delete ${acc.name}`}
            >
              {deletingId === acc.id ? '...' : 'Delete'}
            </Button>
          </div>
        )}
      </div>
    </li>
  )
}
```

Then update the `AccommodationList` component's list rendering to use the new child component:

```tsx
<ul className={styles.list}>
  {accommodations.map(acc => (
    <AccommodationCard
      key={acc.id}
      acc={acc}
      tripId={tripId}
      canEdit={canEdit}
      uploadingId={uploadingId}
      deletingId={deletingId}
      onCoverClick={handleCoverClick}
      onEdit={(id) => setEditId(id)}
      onDelete={handleDelete}
    />
  ))}
</ul>
```

**Step 2: Verify**

Run: `cd frontend && npx tsc --noEmit`

**Step 3: Commit**

```bash
git add frontend/src/components/trips/AccommodationList.tsx
git commit -m "feat: integrate auto-cover into AccommodationList cards"
```

---

### Task 13: End-to-end verification

**Step 1: Run the full backend build**

Run: `cd backend && cargo build`

**Step 2: Run the full frontend build**

Run: `cd frontend && npm run build`

**Step 3: Manual testing checklist**

Set `UNSPLASH_ACCESS_KEY` in your env and start the dev server. Verify:

- [ ] Create a new trip with destination "Tokyo" — cover image auto-fetches on the dashboard
- [ ] Navigate to trip detail — hero image shows the same auto-fetched cover (deduped, no second request)
- [ ] Attribution overlay shows "Photo by X / Unsplash" at the bottom of the cover
- [ ] Click the attribution link — opens photographer's Unsplash profile in new tab (does not navigate the card)
- [ ] Upload a manual cover — attribution overlay disappears
- [ ] Add an accommodation "Park Hyatt Tokyo" — cover auto-fetches
- [ ] Create a trip without a destination — no auto-cover attempt, no errors
- [ ] Without `UNSPLASH_ACCESS_KEY` set — no auto-cover attempts, no errors in console

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat: auto-cover image service with Unsplash integration"
```
