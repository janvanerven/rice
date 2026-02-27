# Accommodations Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add accommodation tracking to trips — name, dates, address, notes, optional cover image.

**Architecture:** New `accommodations` table with FK to trips. CRUD API at `/api/trips/{trip_id}/accommodations` protected by existing `TripAccess` extractor. Frontend adds an accommodations section to the trip detail page with cards + add/edit modal.

**Tech Stack:** Rust, Axum, sqlx, SQLite, React, CSS Modules (same as existing)

---

## Task 1: Database Migration

**Files:**
- Create: `backend/migrations/0002_accommodations.sql`

**Step 1: Write the migration**

```sql
-- backend/migrations/0002_accommodations.sql
CREATE TABLE accommodations (
    id TEXT PRIMARY KEY NOT NULL,
    trip_id TEXT NOT NULL REFERENCES trips(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    address TEXT,
    check_in TEXT,
    check_out TEXT,
    notes TEXT,
    cover_image_url TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_accommodations_trip ON accommodations(trip_id);
```

**Step 2: Regenerate sqlx offline cache**

```bash
cd /home/jan/rice/backend
DATABASE_URL=sqlite://rice-dev.db cargo sqlx prepare
```

If `cargo sqlx` is not installed, run: `cargo install sqlx-cli --no-default-features --features sqlite`

If the dev database doesn't exist yet, first run: `DATABASE_URL=sqlite://rice-dev.db cargo run` (let it start and migrate, then Ctrl-C).

**Step 3: Commit**

```bash
git add backend/migrations/0002_accommodations.sql backend/.sqlx/
git commit -m "feat: add accommodations table migration"
```

---

## Task 2: Backend Model + Request Types

**Files:**
- Modify: `backend/src/models.rs`

**Step 1: Add Accommodation model and request types**

Append to `backend/src/models.rs` (after the existing types):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Accommodation {
    pub id: String,
    pub trip_id: String,
    pub name: String,
    pub address: Option<String>,
    pub check_in: Option<String>,
    pub check_out: Option<String>,
    pub notes: Option<String>,
    pub cover_image_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateAccommodationRequest {
    pub name: String,
    pub address: Option<String>,
    pub check_in: Option<String>,
    pub check_out: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAccommodationRequest {
    pub name: Option<String>,
    pub address: Option<String>,
    pub check_in: Option<String>,
    pub check_out: Option<String>,
    pub notes: Option<String>,
}
```

**Step 2: Verify it compiles**

```bash
cd /home/jan/rice/backend && SQLX_OFFLINE=true cargo check
```

**Step 3: Commit**

```bash
git add backend/src/models.rs
git commit -m "feat: add Accommodation model and request types"
```

---

## Task 3: Backend CRUD API

**Files:**
- Create: `backend/src/api/accommodations.rs`
- Modify: `backend/src/api/mod.rs`

**Step 1: Write the CRUD handlers**

Create `backend/src/api/accommodations.rs`:

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::{
    errors::AppError,
    extractors::TripAccess,
    models::{Accommodation, CreateAccommodationRequest, UpdateAccommodationRequest},
    AppState,
};

pub async fn list_accommodations(
    State(state): State<AppState>,
    access: TripAccess,
) -> Result<Json<Vec<Accommodation>>, AppError> {
    let rows: Vec<Accommodation> = sqlx::query_as(
        "SELECT * FROM accommodations WHERE trip_id = ?1 ORDER BY check_in ASC, created_at ASC",
    )
    .bind(&access.trip_id)
    .fetch_all(&state.db.read)
    .await?;

    Ok(Json(rows))
}

pub async fn create_accommodation(
    State(state): State<AppState>,
    access: TripAccess,
    Json(req): Json<CreateAccommodationRequest>,
) -> Result<(StatusCode, Json<Accommodation>), AppError> {
    access.require_editor()?;

    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Accommodation name cannot be empty".into()));
    }

    let id = ulid::Ulid::new().to_string();

    sqlx::query(
        "INSERT INTO accommodations (id, trip_id, name, address, check_in, check_out, notes) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(&id)
    .bind(&access.trip_id)
    .bind(&name)
    .bind(&req.address)
    .bind(&req.check_in)
    .bind(&req.check_out)
    .bind(&req.notes)
    .execute(&state.db.write)
    .await?;

    let accommodation: Accommodation = sqlx::query_as(
        "SELECT * FROM accommodations WHERE id = ?1",
    )
    .bind(&id)
    .fetch_one(&state.db.read)
    .await?;

    Ok((StatusCode::CREATED, Json(accommodation)))
}

pub async fn update_accommodation(
    State(state): State<AppState>,
    access: TripAccess,
    Path(params): Path<std::collections::HashMap<String, String>>,
    Json(req): Json<UpdateAccommodationRequest>,
) -> Result<Json<Accommodation>, AppError> {
    access.require_editor()?;

    let accommodation_id = params
        .get("accommodation_id")
        .ok_or_else(|| AppError::BadRequest("Missing accommodation_id".into()))?;

    let existing: Accommodation = sqlx::query_as(
        "SELECT * FROM accommodations WHERE id = ?1 AND trip_id = ?2",
    )
    .bind(accommodation_id)
    .bind(&access.trip_id)
    .fetch_optional(&state.db.read)
    .await?
    .ok_or_else(|| AppError::NotFound("Accommodation not found".into()))?;

    let name = req.name.unwrap_or(existing.name);
    if name.trim().is_empty() {
        return Err(AppError::BadRequest("Accommodation name cannot be empty".into()));
    }
    let address = req.address.or(existing.address);
    let check_in = req.check_in.or(existing.check_in);
    let check_out = req.check_out.or(existing.check_out);
    let notes = req.notes.or(existing.notes);

    sqlx::query(
        "UPDATE accommodations SET name = ?1, address = ?2, check_in = ?3, \
         check_out = ?4, notes = ?5, updated_at = datetime('now') WHERE id = ?6",
    )
    .bind(name.trim())
    .bind(&address)
    .bind(&check_in)
    .bind(&check_out)
    .bind(&notes)
    .bind(accommodation_id)
    .execute(&state.db.write)
    .await?;

    let updated: Accommodation = sqlx::query_as(
        "SELECT * FROM accommodations WHERE id = ?1",
    )
    .bind(accommodation_id)
    .fetch_one(&state.db.read)
    .await?;

    Ok(Json(updated))
}

pub async fn delete_accommodation(
    State(state): State<AppState>,
    access: TripAccess,
    Path(params): Path<std::collections::HashMap<String, String>>,
) -> Result<StatusCode, AppError> {
    access.require_editor()?;

    let accommodation_id = params
        .get("accommodation_id")
        .ok_or_else(|| AppError::BadRequest("Missing accommodation_id".into()))?;

    let result = sqlx::query(
        "DELETE FROM accommodations WHERE id = ?1 AND trip_id = ?2",
    )
    .bind(accommodation_id)
    .bind(&access.trip_id)
    .execute(&state.db.write)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Accommodation not found".into()));
    }

    Ok(StatusCode::NO_CONTENT)
}
```

**Step 2: Register the module and routes in `api/mod.rs`**

Add to the module declarations at the top of `backend/src/api/mod.rs`:

```rust
pub mod accommodations;
```

Add to the `router()` function, after the existing `.route(...)` calls:

```rust
        .route(
            "/api/trips/{trip_id}/accommodations",
            get(accommodations::list_accommodations)
                .post(accommodations::create_accommodation),
        )
        .route(
            "/api/trips/{trip_id}/accommodations/{accommodation_id}",
            axum::routing::put(accommodations::update_accommodation)
                .delete(accommodations::delete_accommodation),
        )
```

**Step 3: Verify it compiles**

```bash
cd /home/jan/rice/backend && SQLX_OFFLINE=true cargo check
```

**Step 4: Commit**

```bash
git add backend/src/api/accommodations.rs backend/src/api/mod.rs
git commit -m "feat: add accommodation CRUD API endpoints"
```

---

## Task 4: Accommodation Cover Upload

**Files:**
- Modify: `backend/src/api/accommodations.rs`
- Modify: `backend/src/api/mod.rs`

**Step 1: Add upload handler**

Append to `backend/src/api/accommodations.rs`:

```rust
pub async fn upload_accommodation_cover(
    State(state): State<AppState>,
    access: TripAccess,
    Path(params): Path<std::collections::HashMap<String, String>>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    access.require_editor()?;

    let accommodation_id = params
        .get("accommodation_id")
        .ok_or_else(|| AppError::BadRequest("Missing accommodation_id".into()))?;

    // Verify accommodation belongs to this trip
    let _existing: Accommodation = sqlx::query_as(
        "SELECT * FROM accommodations WHERE id = ?1 AND trip_id = ?2",
    )
    .bind(accommodation_id)
    .bind(&access.trip_id)
    .fetch_optional(&state.db.read)
    .await?
    .ok_or_else(|| AppError::NotFound("Accommodation not found".into()))?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {e}")))?
    {
        if field.name() != Some("cover") {
            continue;
        }

        let content_type = field.content_type().unwrap_or("").to_string();
        if !["image/jpeg", "image/png", "image/webp"].contains(&content_type.as_str()) {
            return Err(AppError::BadRequest(
                "Only JPEG, PNG, and WebP images allowed".into(),
            ));
        }

        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("Failed to read upload: {e}")))?;

        if data.len() > 5 * 1024 * 1024 {
            return Err(AppError::BadRequest("Image must be under 5MB".into()));
        }

        let ext = match content_type.as_str() {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/webp" => "webp",
            _ => "bin",
        };

        let filename = format!("{}.{ext}", ulid::Ulid::new());
        let upload_base = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "/data/uploads".into());
        let dir = std::path::PathBuf::from(&upload_base).join(&access.trip_id);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to create upload dir: {e}")))?;

        let path = dir.join(&filename);
        tokio::fs::write(&path, &data)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to write file: {e}")))?;

        let relative_path = format!("/uploads/{}/{filename}", access.trip_id);

        sqlx::query(
            "UPDATE accommodations SET cover_image_url = ?1, updated_at = datetime('now') WHERE id = ?2",
        )
        .bind(&relative_path)
        .bind(accommodation_id)
        .execute(&state.db.write)
        .await?;

        return Ok(Json(serde_json::json!({ "path": relative_path })));
    }

    Err(AppError::BadRequest("No cover image provided".into()))
}
```

**Step 2: Add the route in `api/mod.rs`**

Add after the other accommodation routes:

```rust
        .route(
            "/api/trips/{trip_id}/accommodations/{accommodation_id}/cover",
            post(accommodations::upload_accommodation_cover),
        )
```

**Step 3: Verify it compiles**

```bash
cd /home/jan/rice/backend && SQLX_OFFLINE=true cargo check
```

**Step 4: Commit**

```bash
git add backend/src/api/accommodations.rs backend/src/api/mod.rs
git commit -m "feat: add accommodation cover image upload"
```

---

## Task 5: Frontend Types + API Client

**Files:**
- Modify: `frontend/src/types/index.ts`
- Modify: `frontend/src/lib/api.ts`

**Step 1: Add TypeScript types**

Append to `frontend/src/types/index.ts`:

```typescript
export interface Accommodation {
  id: string
  trip_id: string
  name: string
  address: string | null
  check_in: string | null
  check_out: string | null
  notes: string | null
  cover_image_url: string | null
  created_at: string
  updated_at: string
}

export interface CreateAccommodationRequest {
  name: string
  address?: string
  check_in?: string
  check_out?: string
  notes?: string
}

export interface UpdateAccommodationRequest {
  name?: string
  address?: string
  check_in?: string
  check_out?: string
  notes?: string
}
```

**Step 2: Add API methods**

Add to `frontend/src/lib/api.ts`, inside the `api` object (after the `invites` block):

```typescript
  accommodations: {
    list: (tripId: string) =>
      request<Accommodation[]>(`/api/trips/${tripId}/accommodations`),
    create: (tripId: string, data: CreateAccommodationRequest) =>
      request<Accommodation>(`/api/trips/${tripId}/accommodations`, {
        method: 'POST',
        body: JSON.stringify(data),
      }),
    update: (tripId: string, id: string, data: UpdateAccommodationRequest) =>
      request<Accommodation>(`/api/trips/${tripId}/accommodations/${id}`, {
        method: 'PUT',
        body: JSON.stringify(data),
      }),
    delete: (tripId: string, id: string) =>
      request<void>(`/api/trips/${tripId}/accommodations/${id}`, { method: 'DELETE' }),
    uploadCover: (tripId: string, id: string, file: File): Promise<{ path: string }> => {
      const form = new FormData()
      form.append('cover', file)
      return fetch(`/api/trips/${tripId}/accommodations/${id}/cover`, {
        method: 'POST',
        body: form,
      }).then(res => {
        if (res.status === 401) {
          window.location.href = '/auth/login'
          throw new Error('Unauthorized')
        }
        if (!res.ok) throw new Error('Upload failed')
        return res.json()
      })
    },
  },
```

Also update the import line at top of `api.ts`:

```typescript
import type { User, Trip, TripMember, CreateTripRequest, UpdateTripRequest, Accommodation, CreateAccommodationRequest, UpdateAccommodationRequest } from '../types'
```

**Step 3: Verify it compiles**

```bash
cd /home/jan/rice/frontend && npx tsc --noEmit
```

**Step 4: Commit**

```bash
git add frontend/src/types/index.ts frontend/src/lib/api.ts
git commit -m "feat: add accommodation types and API client methods"
```

---

## Task 6: AccommodationForm Component

**Files:**
- Create: `frontend/src/components/trips/AccommodationForm.tsx`
- Create: `frontend/src/components/trips/AccommodationForm.module.css`

**Step 1: Write the form component**

Create `frontend/src/components/trips/AccommodationForm.tsx`:

```tsx
import { useState } from 'react'
import { Input, Button } from '../ui'
import type { CreateAccommodationRequest } from '../../types'
import styles from './AccommodationForm.module.css'

interface AccommodationFormInitialData {
  name: string
  address: string
  check_in: string
  check_out: string
  notes: string
}

interface AccommodationFormProps {
  initialData?: AccommodationFormInitialData
  onSubmit: (data: CreateAccommodationRequest) => Promise<void>
  submitLabel?: string
}

export function AccommodationForm({
  initialData,
  onSubmit,
  submitLabel = 'Add Accommodation',
}: AccommodationFormProps) {
  const [name, setName] = useState(initialData?.name ?? '')
  const [address, setAddress] = useState(initialData?.address ?? '')
  const [checkIn, setCheckIn] = useState(initialData?.check_in ?? '')
  const [checkOut, setCheckOut] = useState(initialData?.check_out ?? '')
  const [notes, setNotes] = useState(initialData?.notes ?? '')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [nameError, setNameError] = useState<string | undefined>(undefined)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError(null)
    setNameError(undefined)

    if (!name.trim()) {
      setNameError('Name is required')
      return
    }

    const data: CreateAccommodationRequest = {
      name: name.trim(),
      address: address.trim() || undefined,
      check_in: checkIn || undefined,
      check_out: checkOut || undefined,
      notes: notes.trim() || undefined,
    }

    setLoading(true)
    try {
      await onSubmit(data)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Something went wrong')
    } finally {
      setLoading(false)
    }
  }

  return (
    <form className={styles.form} onSubmit={handleSubmit} noValidate>
      {error && (
        <div className={styles.formError} role="alert">
          <span className={styles.formErrorIcon} aria-hidden="true">⚠</span>
          {error}
        </div>
      )}

      <Input
        label="Name"
        id="acc-name"
        type="text"
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder="Hotel Akira"
        required
        autoComplete="off"
        error={nameError}
        disabled={loading}
      />

      <Input
        label="Address"
        id="acc-address"
        type="text"
        value={address}
        onChange={(e) => setAddress(e.target.value)}
        placeholder="123 Neon Street, Neo-Tokyo"
        autoComplete="off"
        disabled={loading}
      />

      <div className={styles.dateRow}>
        <Input
          label="Check-in"
          id="acc-check-in"
          type="date"
          value={checkIn}
          onChange={(e) => setCheckIn(e.target.value)}
          disabled={loading}
        />
        <Input
          label="Check-out"
          id="acc-check-out"
          type="date"
          value={checkOut}
          onChange={(e) => setCheckOut(e.target.value)}
          min={checkIn || undefined}
          disabled={loading}
        />
      </div>

      <Input
        label="Notes"
        id="acc-notes"
        type="text"
        value={notes}
        onChange={(e) => setNotes(e.target.value)}
        placeholder="Free parking, late check-out confirmed"
        autoComplete="off"
        disabled={loading}
      />

      <div className={styles.actions}>
        <Button type="submit" variant="primary" size="md" disabled={loading}>
          {loading ? 'Saving…' : submitLabel}
        </Button>
      </div>
    </form>
  )
}
```

**Step 2: Write the CSS module**

Create `frontend/src/components/trips/AccommodationForm.module.css`:

```css
/* AccommodationForm — reuses TripForm layout patterns */

.form {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.dateRow {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-4);
}

@media (max-width: 480px) {
  .dateRow {
    grid-template-columns: 1fr;
  }
}

.actions {
  display: flex;
  justify-content: flex-end;
  padding-top: var(--space-2);
}

.formError {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  background: rgba(255, 45, 85, 0.1);
  border: 1px solid rgba(255, 45, 85, 0.3);
  border-radius: var(--radius-md);
  color: var(--color-neon-red);
  font-size: var(--text-sm);
}

.formErrorIcon {
  flex-shrink: 0;
}
```

**Step 3: Verify it compiles**

```bash
cd /home/jan/rice/frontend && npx tsc --noEmit
```

**Step 4: Commit**

```bash
git add frontend/src/components/trips/AccommodationForm.tsx frontend/src/components/trips/AccommodationForm.module.css
git commit -m "feat: add AccommodationForm component"
```

---

## Task 7: AccommodationList Component

**Files:**
- Create: `frontend/src/components/trips/AccommodationList.tsx`
- Create: `frontend/src/components/trips/AccommodationList.module.css`

**Step 1: Write the list component**

Create `frontend/src/components/trips/AccommodationList.tsx`:

```tsx
import { useState, useRef } from 'react'
import { Button, Modal } from '../ui'
import { AccommodationForm } from './AccommodationForm'
import type { Accommodation, CreateAccommodationRequest } from '../../types'
import { api } from '../../lib/api'
import styles from './AccommodationList.module.css'

interface AccommodationListProps {
  tripId: string
  accommodations: Accommodation[]
  canEdit: boolean
  onUpdate: () => void
}

function formatDateRange(checkIn: string | null, checkOut: string | null): string {
  if (!checkIn && !checkOut) return ''

  const fmt = (d: string) => {
    const date = new Date(`${d}T00:00:00`)
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' })
  }

  if (checkIn && checkOut) return `${fmt(checkIn)} → ${fmt(checkOut)}`
  if (checkIn) return `From ${fmt(checkIn)}`
  return `Until ${fmt(checkOut!)}`
}

export function AccommodationList({ tripId, accommodations, canEdit, onUpdate }: AccommodationListProps) {
  const [addOpen, setAddOpen] = useState(false)
  const [editId, setEditId] = useState<string | null>(null)
  const [deletingId, setDeletingId] = useState<string | null>(null)
  const [uploadingId, setUploadingId] = useState<string | null>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)
  const uploadTargetRef = useRef<string | null>(null)

  const editingAccommodation = editId
    ? accommodations.find(a => a.id === editId)
    : null

  const handleCreate = async (data: CreateAccommodationRequest) => {
    await api.accommodations.create(tripId, data)
    setAddOpen(false)
    onUpdate()
  }

  const handleUpdate = async (data: CreateAccommodationRequest) => {
    if (!editId) return
    await api.accommodations.update(tripId, editId, data)
    setEditId(null)
    onUpdate()
  }

  const handleDelete = async (id: string, name: string) => {
    if (!window.confirm(`Delete "${name}"?`)) return
    setDeletingId(id)
    try {
      await api.accommodations.delete(tripId, id)
      onUpdate()
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to delete')
    } finally {
      setDeletingId(null)
    }
  }

  const handleCoverClick = (accommodationId: string) => {
    uploadTargetRef.current = accommodationId
    fileInputRef.current?.click()
  }

  const handleCoverUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    const targetId = uploadTargetRef.current
    if (!file || !targetId) return

    setUploadingId(targetId)
    try {
      await api.accommodations.uploadCover(tripId, targetId, file)
      onUpdate()
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to upload cover')
    } finally {
      setUploadingId(null)
      if (fileInputRef.current) fileInputRef.current.value = ''
    }
  }

  return (
    <section className={styles.section}>
      <div className={styles.sectionHeader}>
        <div className={styles.sectionTitle}>
          <span className={styles.sectionTitleLabel}>Accommodations</span>
          <span className={styles.itemCount}>{accommodations.length}</span>
        </div>
        {canEdit && (
          <Button variant="secondary" size="sm" onClick={() => setAddOpen(true)}>
            Add
          </Button>
        )}
      </div>

      {accommodations.length === 0 ? (
        <p className={styles.emptyText}>No accommodations added yet.</p>
      ) : (
        <ul className={styles.list}>
          {accommodations.map(acc => (
            <li key={acc.id} className={styles.card}>
              {/* Cover image area */}
              <div className={styles.cardCover}>
                {acc.cover_image_url ? (
                  <img
                    src={acc.cover_image_url}
                    alt={`Cover for ${acc.name}`}
                    className={styles.cardCoverImage}
                  />
                ) : (
                  <div className={styles.cardCoverPlaceholder} aria-hidden="true" />
                )}
                <div className={styles.cardCoverOverlay} aria-hidden="true" />
                {canEdit && (
                  <button
                    className={styles.cardCoverBtn}
                    onClick={() => handleCoverClick(acc.id)}
                    disabled={uploadingId === acc.id}
                  >
                    {uploadingId === acc.id ? '…' : '📷'}
                  </button>
                )}
              </div>

              {/* Card body */}
              <div className={styles.cardBody}>
                <h3 className={styles.cardName}>{acc.name}</h3>

                {(acc.check_in || acc.check_out) && (
                  <p className={styles.cardDates}>
                    {formatDateRange(acc.check_in, acc.check_out)}
                  </p>
                )}

                {acc.address && (
                  <p className={styles.cardAddress}>{acc.address}</p>
                )}

                {acc.notes && (
                  <p className={styles.cardNotes}>{acc.notes}</p>
                )}

                {canEdit && (
                  <div className={styles.cardActions}>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => setEditId(acc.id)}
                    >
                      Edit
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleDelete(acc.id, acc.name)}
                      disabled={deletingId === acc.id}
                    >
                      {deletingId === acc.id ? '…' : 'Delete'}
                    </Button>
                  </div>
                )}
              </div>
            </li>
          ))}
        </ul>
      )}

      {/* Hidden file input for cover upload */}
      <input
        ref={fileInputRef}
        type="file"
        accept="image/jpeg,image/png,image/webp"
        onChange={handleCoverUpload}
        className={styles.hiddenInput}
        aria-label="Upload accommodation cover"
      />

      {/* Add modal */}
      <Modal open={addOpen} onClose={() => setAddOpen(false)} title="Add Accommodation">
        <AccommodationForm onSubmit={handleCreate} />
      </Modal>

      {/* Edit modal */}
      <Modal open={!!editId} onClose={() => setEditId(null)} title="Edit Accommodation">
        {editingAccommodation && (
          <AccommodationForm
            initialData={{
              name: editingAccommodation.name,
              address: editingAccommodation.address ?? '',
              check_in: editingAccommodation.check_in ?? '',
              check_out: editingAccommodation.check_out ?? '',
              notes: editingAccommodation.notes ?? '',
            }}
            onSubmit={handleUpdate}
            submitLabel="Save Changes"
          />
        )}
      </Modal>
    </section>
  )
}
```

**Step 2: Write the CSS module**

Create `frontend/src/components/trips/AccommodationList.module.css`:

```css
/* AccommodationList — accommodation cards for trip detail */

.section {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.sectionHeader {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.sectionTitle {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}

.sectionTitleLabel {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: var(--tracking-wider);
  color: var(--color-text-secondary);
}

.itemCount {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--color-text-tertiary);
  background: var(--color-surface-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-full);
  padding: 1px var(--space-2);
  min-width: 20px;
  text-align: center;
  line-height: 1.6;
}

.emptyText {
  font-size: var(--text-sm);
  color: var(--color-text-tertiary);
  font-style: italic;
  padding: var(--space-4) 0;
}

.list {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

/* ---- Card ---- */
.card {
  background: var(--color-surface-2);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  overflow: hidden;
  transition:
    border-color var(--transition-fast),
    box-shadow var(--transition-fast);
}

.card:hover {
  border-color: var(--color-border-2);
}

/* Card cover area */
.cardCover {
  position: relative;
  height: 100px;
  overflow: hidden;
  background: var(--color-surface-3);
}

.cardCoverImage {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
  mix-blend-mode: luminosity;
  opacity: 0.85;
}

.cardCoverPlaceholder {
  position: absolute;
  inset: 0;
  background:
    linear-gradient(
      135deg,
      var(--color-surface-2) 0%,
      var(--color-surface-3) 60%,
      rgba(255, 107, 43, 0.06) 100%
    );
}

.cardCoverPlaceholder::after {
  content: '';
  position: absolute;
  inset: 0;
  background: repeating-linear-gradient(
    0deg,
    transparent,
    transparent 3px,
    rgba(255, 255, 255, 0.012) 3px,
    rgba(255, 255, 255, 0.012) 4px
  );
}

.cardCoverOverlay {
  position: absolute;
  inset: 0;
  background: linear-gradient(
    to bottom,
    transparent 40%,
    rgba(10, 10, 15, 0.7) 100%
  );
  pointer-events: none;
}

.cardCoverBtn {
  position: absolute;
  bottom: var(--space-2);
  right: var(--space-2);
  z-index: 2;
  appearance: none;
  background: rgba(10, 10, 15, 0.7);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-full);
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--text-xs);
  cursor: pointer;
  backdrop-filter: blur(4px);
  -webkit-backdrop-filter: blur(4px);
  transition: border-color var(--transition-fast);
}

.cardCoverBtn:hover:not(:disabled) {
  border-color: var(--color-neon-primary);
}

.cardCoverBtn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

/* Card body */
.cardBody {
  padding: var(--space-3) var(--space-4) var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.cardName {
  font-size: var(--text-base);
  font-weight: 600;
  color: var(--color-text-primary);
  letter-spacing: var(--tracking-tight);
}

.cardDates {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  text-transform: uppercase;
  letter-spacing: var(--tracking-wide);
  color: var(--color-neon-primary);
  opacity: 0.8;
}

.cardAddress {
  font-size: var(--text-sm);
  color: var(--color-text-secondary);
}

.cardNotes {
  font-size: var(--text-sm);
  color: var(--color-text-tertiary);
  font-style: italic;
  margin-top: var(--space-1);
}

.cardActions {
  display: flex;
  gap: var(--space-2);
  margin-top: var(--space-2);
}

.hiddenInput {
  display: none;
}
```

**Step 3: Verify it compiles**

```bash
cd /home/jan/rice/frontend && npx tsc --noEmit
```

**Step 4: Commit**

```bash
git add frontend/src/components/trips/AccommodationList.tsx frontend/src/components/trips/AccommodationList.module.css
git commit -m "feat: add AccommodationList component with cards and modals"
```

---

## Task 8: Wire Accommodations into Trip Detail Page

**Files:**
- Modify: `frontend/src/pages/TripDetailPage.tsx`
- Modify: `frontend/src/components/trips/TripDetail.tsx`

**Step 1: Fetch accommodations in TripDetailPage**

In `frontend/src/pages/TripDetailPage.tsx`:

Add `Accommodation` to the type import:

```typescript
import type { Trip, TripMember, Accommodation } from '../types'
```

Add state for accommodations:

```typescript
const [accommodations, setAccommodations] = useState<Accommodation[]>([])
```

Update `fetchData` to fetch accommodations in parallel (add to the `Promise.all`):

```typescript
const [fetchedTrip, fetchedMembers, fetchedAccommodations] = await Promise.all([
  api.trips.get(id),
  api.members.list(id),
  api.accommodations.list(id),
])
setTrip(fetchedTrip)
setMembers(fetchedMembers)
setAccommodations(fetchedAccommodations)
```

Pass accommodations to `TripDetail`:

```tsx
<TripDetail
  trip={trip}
  members={members}
  accommodations={accommodations}
  onUpdate={fetchData}
/>
```

**Step 2: Add accommodations section to TripDetail**

In `frontend/src/components/trips/TripDetail.tsx`:

Add import:

```typescript
import { AccommodationList } from './AccommodationList'
import type { Trip, TripMember, CreateTripRequest, Accommodation } from '../../types'
```

Update the props interface:

```typescript
interface TripDetailProps {
  trip: Trip
  members: TripMember[]
  accommodations: Accommodation[]
  onUpdate: () => void
}
```

Update the destructured props:

```typescript
export function TripDetail({ trip, members, accommodations, onUpdate }: TripDetailProps) {
```

Add the accommodations section in the `columnMain` div, after the `metaGrid` div:

```tsx
            <AccommodationList
              tripId={trip.id}
              accommodations={accommodations}
              canEdit={canEdit}
              onUpdate={onUpdate}
            />
```

**Step 3: Verify it compiles**

```bash
cd /home/jan/rice/frontend && npx tsc --noEmit
```

**Step 4: Verify full build**

```bash
cd /home/jan/rice/frontend && npm run build
```

**Step 5: Commit**

```bash
git add frontend/src/pages/TripDetailPage.tsx frontend/src/components/trips/TripDetail.tsx
git commit -m "feat: wire accommodations into trip detail page"
```

---

## Task 9: Backend Tests

**Files:**
- Modify: `backend/tests/api_trips_test.rs`

**Step 1: Add accommodation test**

Append to `backend/tests/api_trips_test.rs`:

```rust
#[tokio::test]
async fn test_accommodation_crud() {
    let pool = common::test_db().await;
    let user_id = common::create_test_user(&pool, "test@example.com").await;

    // Create a trip
    let trip_id = ulid::Ulid::new().to_string();
    sqlx::query("INSERT INTO trips (id, name, destination, created_by) VALUES (?1, ?2, ?3, ?4)")
        .bind(&trip_id)
        .bind("Test Trip")
        .bind("Tokyo")
        .bind(&user_id)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO trip_members (trip_id, user_id, role) VALUES (?1, ?2, 'owner')")
        .bind(&trip_id)
        .bind(&user_id)
        .execute(&pool)
        .await
        .unwrap();

    // Create accommodation
    let acc_id = ulid::Ulid::new().to_string();
    sqlx::query(
        "INSERT INTO accommodations (id, trip_id, name, address, check_in, check_out) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(&acc_id)
    .bind(&trip_id)
    .bind("Hotel Akira")
    .bind("123 Neon St")
    .bind("2026-03-15")
    .bind("2026-03-18")
    .execute(&pool)
    .await
    .unwrap();

    // Verify it exists
    let row: (String, String) =
        sqlx::query_as("SELECT name, address FROM accommodations WHERE id = ?1")
            .bind(&acc_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(row.0, "Hotel Akira");
    assert_eq!(row.1, "123 Neon St");

    // Verify cascade delete
    sqlx::query("DELETE FROM trips WHERE id = ?1")
        .bind(&trip_id)
        .execute(&pool)
        .await
        .unwrap();

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM accommodations WHERE trip_id = ?1")
            .bind(&trip_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count.0, 0, "Accommodations should cascade delete with trip");
}
```

**Step 2: Run tests**

```bash
cd /home/jan/rice/backend && cargo test
```

Expected: All tests pass including the new `test_accommodation_crud`.

**Step 3: Commit**

```bash
git add backend/tests/api_trips_test.rs
git commit -m "test: add accommodation CRUD and cascade delete tests"
```

---

## Task 10: Full Build + Docker Verification

**Step 1: Run all backend tests**

```bash
cd /home/jan/rice/backend && cargo test
```

**Step 2: Build frontend**

```bash
cd /home/jan/rice/frontend && npm run build
```

**Step 3: Build Docker image**

```bash
cd /home/jan/rice && docker build -t rice:dev .
```

**Step 4: Redeploy**

```bash
cd /home/jan/rice && docker compose down && docker compose up -d
```

**Step 5: Verify health**

```bash
sleep 3 && curl -s http://localhost:3000/health
```

Expected: `ok`

**Step 6: Commit everything (if any sqlx cache changes)**

```bash
git add -A && git status
# If there are changes:
git commit -m "chore: update sqlx offline cache for accommodations"
```

---

## Dependency Graph

```
Task 1 (Migration)
  └→ Task 2 (Models)
       └→ Task 3 (CRUD API)
            └→ Task 4 (Cover Upload)

Task 5 (Frontend Types + API)
  └→ Task 6 (AccommodationForm)
       └→ Task 7 (AccommodationList)
            └→ Task 8 (Wire into TripDetail)

Task 3 → Task 9 (Tests)
Task 8 + Task 4 → Task 10 (Full Build + Docker)
```

Backend tasks (1-4) and frontend tasks (5-8) can be worked in parallel after Task 2 is complete.
