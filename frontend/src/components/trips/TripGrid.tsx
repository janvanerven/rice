import type { Trip } from '../../types'
import { TripCard } from './TripCard'
import styles from './TripGrid.module.css'

interface TripGridProps {
  trips: Trip[]
}

export function TripGrid({ trips }: TripGridProps) {
  return (
    <ul className={styles.grid} role="list" aria-label="Your trips">
      {trips.map((trip) => (
        <li key={trip.id} className={styles.item}>
          <TripCard trip={trip} />
        </li>
      ))}
    </ul>
  )
}
