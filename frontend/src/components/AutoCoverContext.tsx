import { createContext, useContext, useRef, useCallback } from 'react'
import { api } from '../lib/api'
import type { AutoCoverResponse } from '../types'

type EntityType = 'trip' | 'accommodation'

interface AutoCoverRequest {
  entityType: EntityType
  entityId: string
  tripId: string
  onSuccess: (result: AutoCoverResponse) => void
}

interface AutoCoverContextType {
  requestAutoCover: (req: AutoCoverRequest) => void
}

const AutoCoverContext = createContext<AutoCoverContextType>({
  requestAutoCover: () => {},
})

export function useAutoCover() {
  return useContext(AutoCoverContext)
}

const MAX_CONCURRENT = 2

export function AutoCoverProvider({ children }: { children: React.ReactNode }) {
  // Refs survive StrictMode double-mount and avoid stale closures
  const inFlightRef = useRef(new Set<string>())
  const attemptedRef = useRef(new Set<string>())
  const queueRef = useRef<AutoCoverRequest[]>([])
  const activeCountRef = useRef(0)

  const processQueue = useCallback(() => {
    while (activeCountRef.current < MAX_CONCURRENT && queueRef.current.length > 0) {
      const req = queueRef.current.shift()!
      executeRequest(req)
    }
  }, [])

  const executeRequest = useCallback((req: AutoCoverRequest) => {
    const key = `${req.entityType}:${req.entityId}`
    activeCountRef.current++
    inFlightRef.current.add(key)

    const promise =
      req.entityType === 'trip'
        ? api.trips.autoCover(req.tripId)
        : api.accommodations.autoCover(req.tripId, req.entityId)

    promise
      .then((result) => {
        req.onSuccess(result)
      })
      .catch(() => {
        // Silent failure for background operations
      })
      .finally(() => {
        inFlightRef.current.delete(key)
        attemptedRef.current.add(key)
        activeCountRef.current--
        processQueue()
      })
  }, [processQueue])

  const requestAutoCover = useCallback((req: AutoCoverRequest) => {
    const key = `${req.entityType}:${req.entityId}`

    if (attemptedRef.current.has(key) || inFlightRef.current.has(key)) {
      return
    }

    if (activeCountRef.current < MAX_CONCURRENT) {
      executeRequest(req)
    } else {
      queueRef.current.push(req)
    }
  }, [executeRequest])

  return (
    <AutoCoverContext.Provider value={{ requestAutoCover }}>
      {children}
    </AutoCoverContext.Provider>
  )
}
