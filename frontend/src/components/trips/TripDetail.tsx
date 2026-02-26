import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Button, Modal, GlowDivider } from '../ui'
import { CollaboratorList } from './CollaboratorList'
import { TripForm } from './TripForm'
import type { Trip, TripMember, CreateTripRequest } from '../../types'
import { api } from '../../lib/api'
import styles from './TripDetail.module.css'

interface TripDetailProps {
  trip: Trip
  members: TripMember[]
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

export function TripDetail({ trip, members, onUpdate }: TripDetailProps) {
  const navigate = useNavigate()
  const coverUrl = trip.cover_image_path
    ? `/api/uploads/${trip.cover_image_path}`
    : null

  const canEdit = ['owner', 'editor'].includes(trip.role.toLowerCase())
  const isOwner = trip.role.toLowerCase() === 'owner'

  const [editOpen, setEditOpen] = useState(false)
  const [deleteLoading, setDeleteLoading] = useState(false)

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
        ) : (
          <div className={styles.heroPlaceholder} aria-hidden="true" />
        )}
        <div className={styles.heroOverlay} aria-hidden="true" />

        {/* Badge chips overlaid on hero */}
        <div className={styles.heroBadges} aria-hidden="true">
          {trip.destination && (
            <span className={styles.heroBadgeChip}>{trip.destination}</span>
          )}
        </div>
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
