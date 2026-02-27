import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { Badge, Attribution as AttributionOverlay } from '../ui'
import type { BadgeVariant } from '../ui'
import type { Trip, Attribution } from '../../types'
import { useAutoCover } from '../AutoCoverContext'
import styles from './TripCard.module.css'

interface TripCardProps {
  trip: Trip
}

function formatDateRange(start: string | null, end: string | null): string {
  if (!start && !end) return 'No dates set'

  const fmt = (d: string) => {
    const date = new Date(d)
    return date.toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    })
  }

  if (start && end) return `${fmt(start)} — ${fmt(end)}`
  if (start) return `From ${fmt(start)}`
  return `Until ${fmt(end!)}`
}

function roleBadgeVariant(role: string): BadgeVariant {
  switch (role.toLowerCase()) {
    case 'owner':
      return 'primary'
    case 'editor':
      return 'success'
    default:
      return 'neutral'
  }
}

function roleLabel(role: string): string {
  return role.charAt(0).toUpperCase() + role.slice(1).toLowerCase()
}

export function TripCard({ trip }: TripCardProps) {
  const { requestAutoCover } = useAutoCover()
  const [coverPath, setCoverPath] = useState(trip.cover_image_path)
  const [attribution, setAttribution] = useState<Attribution | null>(trip.attribution ?? null)
  const [loading, setLoading] = useState(false)

  const dateRange = formatDateRange(trip.start_date, trip.end_date)
  const hasDates = trip.start_date || trip.end_date
  const coverUrl = coverPath ? `/api/uploads${coverPath}` : null
  const canEdit = ['owner', 'editor'].includes(trip.role.toLowerCase())

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
