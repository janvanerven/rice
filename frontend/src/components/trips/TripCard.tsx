import { Link } from 'react-router-dom'
import { Badge } from '../ui'
import type { BadgeVariant } from '../ui'
import type { Trip } from '../../types'
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
  const dateRange = formatDateRange(trip.start_date, trip.end_date)
  const hasDates = trip.start_date || trip.end_date
  const coverUrl = trip.cover_image_path
    ? `/api/uploads/${trip.cover_image_path}`
    : null

  return (
    <Link to={`/trips/${trip.id}`} className={styles.link} aria-label={`Open trip: ${trip.name}`}>
      <article className={styles.card}>
        {/* Cover image area */}
        <div className={styles.cover}>
          {coverUrl ? (
            <img
              src={coverUrl}
              alt={`Cover for ${trip.name}`}
              className={styles.coverImage}
            />
          ) : (
            <div className={styles.coverPlaceholder} aria-hidden="true" />
          )}
          {/* Gradient overlay — always present to ensure text legibility */}
          <div className={styles.coverOverlay} aria-hidden="true" />
          {/* Trip name overlaid at bottom of cover */}
          <h2 className={styles.tripName}>{trip.name}</h2>
        </div>

        {/* Card body */}
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
