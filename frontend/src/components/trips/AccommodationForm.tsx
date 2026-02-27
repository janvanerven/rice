import { useState } from 'react'
import { Input, Button } from '../ui'
import type { CreateAccommodationRequest } from '../../types'
import styles from './AccommodationForm.module.css'

interface AccommodationFormInitialData {
  name: string
  address: string
  check_in: string
  check_out: string
  notes: string
}

interface AccommodationFormProps {
  initialData?: AccommodationFormInitialData
  onSubmit: (data: CreateAccommodationRequest) => Promise<void>
  submitLabel?: string
}

export function AccommodationForm({
  initialData,
  onSubmit,
  submitLabel = 'Add Accommodation',
}: AccommodationFormProps) {
  const [name, setName] = useState(initialData?.name ?? '')
  const [address, setAddress] = useState(initialData?.address ?? '')
  const [checkIn, setCheckIn] = useState(initialData?.check_in ?? '')
  const [checkOut, setCheckOut] = useState(initialData?.check_out ?? '')
  const [notes, setNotes] = useState(initialData?.notes ?? '')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [nameError, setNameError] = useState<string | undefined>(undefined)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError(null)
    setNameError(undefined)

    if (!name.trim()) {
      setNameError('Name is required')
      return
    }

    const data: CreateAccommodationRequest = {
      name: name.trim(),
      address: address.trim() || undefined,
      check_in: checkIn || undefined,
      check_out: checkOut || undefined,
      notes: notes.trim() || undefined,
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
        label="Name"
        id="acc-name"
        type="text"
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder="Hotel Akira"
        required
        autoComplete="off"
        error={nameError}
        disabled={loading}
      />

      <Input
        label="Address"
        id="acc-address"
        type="text"
        value={address}
        onChange={(e) => setAddress(e.target.value)}
        placeholder="123 Neon Street, Neo-Tokyo"
        autoComplete="off"
        disabled={loading}
      />

      <div className={styles.dateRow}>
        <Input
          label="Check-in"
          id="acc-check-in"
          type="date"
          value={checkIn}
          onChange={(e) => setCheckIn(e.target.value)}
          disabled={loading}
        />
        <Input
          label="Check-out"
          id="acc-check-out"
          type="date"
          value={checkOut}
          onChange={(e) => setCheckOut(e.target.value)}
          min={checkIn || undefined}
          disabled={loading}
        />
      </div>

      <div className={styles.textareaGroup}>
        <label htmlFor="acc-notes" className={styles.textareaLabel}>Notes</label>
        <textarea
          id="acc-notes"
          className={styles.textarea}
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          placeholder="Free parking, late check-out confirmed"
          rows={3}
          disabled={loading}
        />
      </div>

      <div className={styles.actions}>
        <Button type="submit" variant="primary" size="md" disabled={loading}>
          {loading ? 'Saving\u2026' : submitLabel}
        </Button>
      </div>
    </form>
  )
}
