import { useParams } from 'react-router-dom'
import { useState, useEffect, useCallback } from 'react'
import { AppShell, PageHeader } from '../components/layout'
import { TripDetail } from '../components/trips/TripDetail'
import { api } from '../lib/api'
import type { Trip, TripMember } from '../types'

export default function TripDetailPage() {
  const { id } = useParams<{ id: string }>()

  const [trip, setTrip] = useState<Trip | null>(null)
  const [members, setMembers] = useState<TripMember[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const fetchData = useCallback(async () => {
    if (!id) return
    setError(null)
    try {
      // Fetch trip and members in parallel
      const [fetchedTrip, fetchedMembers] = await Promise.all([
        api.trips.get(id),
        api.members.list(id),
      ])
      setTrip(fetchedTrip)
      setMembers(fetchedMembers)
    } catch (err) {
      if (err instanceof Error && (err as { status?: number }).status === 404) {
        setError('Trip not found')
      } else {
        setError(err instanceof Error ? err.message : 'Failed to load trip')
      }
    } finally {
      setLoading(false)
    }
  }, [id])

  useEffect(() => {
    fetchData()
  }, [fetchData])

  // Loading state
  if (loading) {
    return (
      <AppShell>
        <div className="loading-scanner" aria-label="Loading trip…" />
      </AppShell>
    )
  }

  // Error / not found state
  if (error || !trip) {
    return (
      <AppShell>
        <PageHeader title="Trip Not Found" />
        <div
          style={{
            padding: 'var(--space-10)',
            textAlign: 'center',
            color: 'var(--color-text-secondary)',
          }}
        >
          <p style={{ fontSize: 'var(--text-2xl)', marginBottom: 'var(--space-4)' }} aria-hidden="true">
            404
          </p>
          <p>{error ?? 'This trip does not exist or you do not have access.'}</p>
        </div>
      </AppShell>
    )
  }

  return (
    <AppShell>
      <PageHeader title={trip.name} />
      <TripDetail
        trip={trip}
        members={members}
        onUpdate={fetchData}
      />
    </AppShell>
  )
}
