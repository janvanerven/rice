import { useState } from 'react'
import { Input } from '../ui'
import { Button } from '../ui'
import type { CreateTripRequest } from '../../types'
import styles from './TripForm.module.css'

interface TripFormInitialData {
  name: string
  destination: string
  start_date: string
  end_date: string
}

interface TripFormProps {
  initialData?: TripFormInitialData
  onSubmit: (data: CreateTripRequest) => Promise<void>
  submitLabel?: string
}

export function TripForm({
  initialData,
  onSubmit,
  submitLabel = 'Create Trip',
}: TripFormProps) {
  const [name, setName] = useState(initialData?.name ?? '')
  const [destination, setDestination] = useState(initialData?.destination ?? '')
  const [startDate, setStartDate] = useState(initialData?.start_date ?? '')
  const [endDate, setEndDate] = useState(initialData?.end_date ?? '')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [nameError, setNameError] = useState<string | undefined>(undefined)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError(null)
    setNameError(undefined)

    if (!name.trim()) {
      setNameError('Trip name is required')
      return
    }

    const data: CreateTripRequest = {
      name: name.trim(),
      destination: destination.trim() || undefined,
      start_date: startDate || undefined,
      end_date: endDate || undefined,
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
        label="Trip Name"
        id="trip-name"
        type="text"
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder="Kyoto Spring Journey"
        required
        autoComplete="off"
        error={nameError}
        disabled={loading}
      />

      <Input
        label="Destination"
        id="trip-destination"
        type="text"
        value={destination}
        onChange={(e) => setDestination(e.target.value)}
        placeholder="Kyoto, Japan"
        autoComplete="off"
        disabled={loading}
      />

      <div className={styles.dateRow}>
        <Input
          label="Start Date"
          id="trip-start-date"
          type="date"
          value={startDate}
          onChange={(e) => setStartDate(e.target.value)}
          disabled={loading}
        />
        <Input
          label="End Date"
          id="trip-end-date"
          type="date"
          value={endDate}
          onChange={(e) => setEndDate(e.target.value)}
          min={startDate || undefined}
          disabled={loading}
        />
      </div>

      <div className={styles.actions}>
        <Button
          type="submit"
          variant="primary"
          size="md"
          disabled={loading}
        >
          {loading ? 'Saving…' : submitLabel}
        </Button>
      </div>
    </form>
  )
}
