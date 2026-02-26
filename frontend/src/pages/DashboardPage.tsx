import { useNavigate } from 'react-router-dom'
import { AppShell, PageHeader } from '../components/layout'
import { Button } from '../components/ui'
import { TripGrid } from '../components/trips/TripGrid'
import { useTrips } from '../hooks/useTrips'
import styles from './DashboardPage.module.css'

export default function DashboardPage() {
  const { trips, loading, error } = useTrips()
  const navigate = useNavigate()

  return (
    <AppShell>
      <div className="page-enter">
        <PageHeader
          title="Your Trips"
          action={
            <Button
              variant="primary"
              size="md"
              onClick={() => navigate('/trips/new')}
            >
              New Trip
            </Button>
          }
        />

        {loading && <div className="loading-scanner" aria-label="Loading trips..." />}

        {!loading && error && (
          <div className={styles.errorState} role="alert">
            <span className={styles.errorIcon} aria-hidden="true">⚠</span>
            <p className={styles.errorMessage}>{error}</p>
            <p className={styles.errorHint}>Check your connection and try again.</p>
          </div>
        )}

        {!loading && !error && trips.length === 0 && (
          <div className={styles.emptyState}>
            <p className={styles.emptyKanji} aria-hidden="true">旅</p>
            <h2 className={styles.emptyHeading}>旅はまだない</h2>
            <p className={styles.emptySubtext}>No trips yet. Begin your journey.</p>
            <Button
              variant="primary"
              size="lg"
              onClick={() => navigate('/trips/new')}
            >
              Create Your First Trip
            </Button>
          </div>
        )}

        {!loading && !error && trips.length > 0 && (
          <TripGrid trips={trips} />
        )}
      </div>
    </AppShell>
  )
}
