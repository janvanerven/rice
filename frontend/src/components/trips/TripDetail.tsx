import { useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Button, Modal, GlowDivider, Attribution as AttributionOverlay } from '../ui'
import { CollaboratorList } from './CollaboratorList'
import { TripForm } from './TripForm'
import { AccommodationList } from './AccommodationList'
import type { Trip, TripMember, CreateTripRequest, Accommodation, Attribution } from '../../types'
import { api } from '../../lib/api'
import { useAutoCover } from '../AutoCoverContext'
import styles from './TripDetail.module.css'

interface TripDetailProps {
  trip: Trip
  members: TripMember[]
  accommodations: Accommodation[]
  onUpdate: () => void
}

function formatDateRange(start: string | null, end: string | null): string {
  if (!start && !end) return 'No dates set'

  const fmt = (d: string) => {
    // Parse YYYY-MM-DD as local time by appending T00:00:00 to avoid UTC shift
    const date = new Date(`${d}T00:00:00`)
    return date.toLocaleDateString('en-US', {
      month: 'long',
      day: 'numeric',
      year: 'numeric',
    })
  }

  if (start && end) return `${fmt(start)} — ${fmt(end)}`
  if (start) return `From ${fmt(start)}`
  return `Until ${fmt(end!)}`
}

export function TripDetail({ trip, members, accommodations, onUpdate }: TripDetailProps) {
  const navigate = useNavigate()
  const { requestAutoCover } = useAutoCover()

  const canEdit = ['owner', 'editor'].includes(trip.role.toLowerCase())
  const isOwner = trip.role.toLowerCase() === 'owner'

  const [autoCoverPath, setAutoCoverPath] = useState<string | null>(null)
  const [autoCoverAttribution, setAutoCoverAttribution] = useState<Attribution | null>(null)
  const [autoCoverLoading, setAutoCoverLoading] = useState(false)

  const effectiveCoverPath = trip.cover_image_path || autoCoverPath
  const coverUrl = effectiveCoverPath ? `/uploads${effectiveCoverPath}` : null
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

  const [editOpen, setEditOpen] = useState(false)
  const [deleteLoading, setDeleteLoading] = useState(false)
  const [uploadingCover, setUploadingCover] = useState(false)
  const fileInputRef = useRef<HTMLInputElement>(null)

  // ---- Edit handler ----
  const handleEdit = async (data: CreateTripRequest) => {
    await api.trips.update(trip.id, data)
    setEditOpen(false)
    onUpdate()
  }

  // ---- Delete handler ----
  const handleDelete = async () => {
    if (!window.confirm(`Delete "${trip.name}"? This cannot be undone.`)) return
    setDeleteLoading(true)
    try {
      await api.trips.delete(trip.id)
      navigate('/')
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to delete trip')
      setDeleteLoading(false)
    }
  }

  // ---- Cover upload handler ----
  const handleCoverUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return

    setUploadingCover(true)
    try {
      await api.uploadCover(trip.id, file)
      onUpdate()
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to upload cover image')
    } finally {
      setUploadingCover(false)
      // Reset input so re-selecting same file triggers change
      if (fileInputRef.current) fileInputRef.current.value = ''
    }
  }

  return (
    <article className={styles.article}>
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
        <div className={styles.heroBadges} aria-hidden="true">
          {trip.destination && (
            <span className={styles.heroBadgeChip}>{trip.destination}</span>
          )}
        </div>

        {/* Cover upload button */}
        {canEdit && (
          <button
            className={styles.coverUploadBtn}
            onClick={() => fileInputRef.current?.click()}
            disabled={uploadingCover}
            title="Change cover image"
          >
            {uploadingCover ? 'Uploading\u2026' : 'Change Cover'}
          </button>
        )}
        <input
          ref={fileInputRef}
          type="file"
          accept="image/jpeg,image/png,image/webp"
          onChange={handleCoverUpload}
          className={styles.hiddenInput}
          aria-label="Upload cover image"
        />
      </div>

      {/* ---- Main content ---- */}
      <div className={styles.content}>
        {/* Trip title + meta */}
        <div className={styles.titleSection}>
          <h1 className={styles.tripName}>{trip.name}</h1>

          {trip.destination && (
            <p className={styles.destination}>{trip.destination}</p>
          )}

          <p className={styles.dateRange}>
            {formatDateRange(trip.start_date, trip.end_date)}
          </p>
        </div>

        {/* Action buttons */}
        {canEdit && (
          <div className={styles.actions}>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setEditOpen(true)}
            >
              Edit
            </Button>

            {isOwner && (
              <Button
                variant="danger"
                size="sm"
                onClick={handleDelete}
                disabled={deleteLoading}
              >
                {deleteLoading ? 'Deleting…' : 'Delete Trip'}
              </Button>
            )}
          </div>
        )}

        <GlowDivider />

        {/* Two-column layout: info + collaborators */}
        <div className={styles.columns}>
          {/* Left column: trip metadata / future itinerary */}
          <div className={styles.columnMain}>
            <div className={styles.metaGrid}>
              <div className={styles.metaItem}>
                <span className={styles.metaLabel}>DESTINATION</span>
                <span className={styles.metaValue}>
                  {trip.destination || <span className={styles.metaNone}>Not set</span>}
                </span>
              </div>
              <div className={styles.metaItem}>
                <span className={styles.metaLabel}>DATES</span>
                <span className={styles.metaValue}>
                  {trip.start_date || trip.end_date
                    ? formatDateRange(trip.start_date, trip.end_date)
                    : <span className={styles.metaNone}>Not set</span>
                  }
                </span>
              </div>
              <div className={styles.metaItem}>
                <span className={styles.metaLabel}>YOUR ROLE</span>
                <span className={styles.metaValue} style={{ textTransform: 'capitalize' }}>
                  {trip.role.toLowerCase()}
                </span>
              </div>
              <div className={styles.metaItem}>
                <span className={styles.metaLabel}>CREATED</span>
                <span className={styles.metaValue}>
                  {new Date(trip.created_at).toLocaleDateString('en-US', {
                    month: 'short',
                    day: 'numeric',
                    year: 'numeric',
                  })}
                </span>
              </div>
            </div>

            <AccommodationList
              tripId={trip.id}
              accommodations={accommodations}
              canEdit={canEdit}
              onUpdate={onUpdate}
            />
          </div>

          {/* Right column: collaborators */}
          <div className={styles.columnSide}>
            <CollaboratorList
              tripId={trip.id}
              members={members}
              userRole={trip.role}
              onMemberRemoved={onUpdate}
            />
          </div>
        </div>
      </div>

      {/* ---- Edit modal ---- */}
      <Modal
        open={editOpen}
        onClose={() => setEditOpen(false)}
        title="Edit Trip"
      >
        <TripForm
          initialData={{
            name: trip.name,
            destination: trip.destination ?? '',
            start_date: trip.start_date ?? '',
            end_date: trip.end_date ?? '',
          }}
          onSubmit={handleEdit}
          submitLabel="Save Changes"
        />
      </Modal>
    </article>
  )
}
