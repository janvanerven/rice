import { useNavigate } from 'react-router-dom'
import { AppShell, PageHeader } from '../components/layout'
import { TripForm } from '../components/trips/TripForm'
import { api } from '../lib/api'
import type { CreateTripRequest } from '../types'
import styles from './TripNewPage.module.css'

export default function TripNewPage() {
  const navigate = useNavigate()

  const handleSubmit = async (data: CreateTripRequest) => {
    const trip = await api.trips.create(data)
    navigate(`/trips/${trip.id}`)
  }

  return (
    <AppShell>
      <div className="page-enter">
        <PageHeader title="New Trip" />
        <div className={styles.content}>
          <TripForm onSubmit={handleSubmit} submitLabel="Create Trip" />
        </div>
      </div>
    </AppShell>
  )
}
