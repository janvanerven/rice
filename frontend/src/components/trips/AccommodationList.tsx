import { useEffect, useState, useRef } from 'react'
import { Button, Modal, Attribution as AttributionOverlay } from '../ui'
import { AccommodationForm } from './AccommodationForm'
import type { Accommodation, Attribution, CreateAccommodationRequest } from '../../types'
import { api } from '../../lib/api'
import { useAutoCover } from '../AutoCoverContext'
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

  if (checkIn && checkOut) return `${fmt(checkIn)} \u2192 ${fmt(checkOut)}`
  if (checkIn) return `From ${fmt(checkIn)}`
  return `Until ${fmt(checkOut!)}`
}

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
    <section className={styles.section} aria-label="Accommodations">
      <div className={styles.sectionHeader}>
        <h3 className={styles.sectionTitle}>
          <span className={styles.sectionTitleLabel}>Accommodations</span>
          <span className={styles.itemCount}>{accommodations.length}</span>
        </h3>
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
      )}

      <input
        ref={fileInputRef}
        type="file"
        accept="image/jpeg,image/png,image/webp"
        onChange={handleCoverUpload}
        className={styles.hiddenInput}
        aria-label="Upload accommodation cover"
      />

      <Modal open={addOpen} onClose={() => setAddOpen(false)} title="Add Accommodation">
        <AccommodationForm onSubmit={handleCreate} />
      </Modal>

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
