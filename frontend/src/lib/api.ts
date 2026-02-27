import type { User, Trip, TripMember, CreateTripRequest, UpdateTripRequest, Accommodation, CreateAccommodationRequest, UpdateAccommodationRequest, AutoCoverResponse } from '../types'

class ApiError extends Error {
  status: number

  constructor(status: number, message: string) {
    super(message)
    this.status = status
  }
}

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  })

  if (res.status === 401) {
    window.location.href = '/auth/login'
    throw new ApiError(401, 'Unauthorized')
  }

  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: 'Unknown error' }))
    throw new ApiError(res.status, body.error || 'Request failed')
  }

  if (res.status === 204) return undefined as T

  return res.json()
}

export const api = {
  me: () => request<User>('/api/me'),

  trips: {
    list: () => request<Trip[]>('/api/trips'),
    get: (id: string) => request<Trip>(`/api/trips/${id}`),
    create: (data: CreateTripRequest) =>
      request<Trip>('/api/trips', { method: 'POST', body: JSON.stringify(data) }),
    update: (id: string, data: UpdateTripRequest) =>
      request<Trip>(`/api/trips/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
    delete: (id: string) =>
      request<void>(`/api/trips/${id}`, { method: 'DELETE' }),
    autoCover: (tripId: string) =>
      request<AutoCoverResponse>(`/api/trips/${tripId}/auto-cover`, { method: 'POST' }),
  },

  members: {
    list: (tripId: string) => request<TripMember[]>(`/api/trips/${tripId}/members`),
    remove: (tripId: string, userId: string) =>
      request<void>(`/api/trips/${tripId}/members/${userId}`, { method: 'DELETE' }),
  },

  invites: {
    create: (tripId: string, email: string, role: string) =>
      request<void>(`/api/trips/${tripId}/invites`, {
        method: 'POST',
        body: JSON.stringify({ email, role }),
      }),
  },

  accommodations: {
    list: (tripId: string) =>
      request<Accommodation[]>(`/api/trips/${tripId}/accommodations`),
    create: (tripId: string, data: CreateAccommodationRequest) =>
      request<Accommodation>(`/api/trips/${tripId}/accommodations`, {
        method: 'POST',
        body: JSON.stringify(data),
      }),
    update: (tripId: string, id: string, data: UpdateAccommodationRequest) =>
      request<Accommodation>(`/api/trips/${tripId}/accommodations/${id}`, {
        method: 'PUT',
        body: JSON.stringify(data),
      }),
    delete: (tripId: string, id: string) =>
      request<void>(`/api/trips/${tripId}/accommodations/${id}`, { method: 'DELETE' }),
    autoCover: (tripId: string, id: string) =>
      request<AutoCoverResponse>(`/api/trips/${tripId}/accommodations/${id}/auto-cover`, { method: 'POST' }),
    uploadCover: async (tripId: string, id: string, file: File): Promise<{ path: string }> => {
      const form = new FormData()
      form.append('cover', file)
      const res = await fetch(`/api/trips/${tripId}/accommodations/${id}/cover`, {
        method: 'POST',
        body: form,
      })
      if (res.status === 401) {
        window.location.href = '/auth/login'
        throw new ApiError(401, 'Unauthorized')
      }
      if (!res.ok) {
        const body = await res.json().catch(() => ({ error: 'Upload failed' }))
        throw new ApiError(res.status, body.error || 'Upload failed')
      }
      return res.json()
    },
  },

  uploadCover: (tripId: string, file: File): Promise<{ path: string }> => {
    const form = new FormData()
    form.append('cover', file)
    return fetch(`/api/trips/${tripId}/cover`, { method: 'POST', body: form })
      .then(res => {
        if (res.status === 401) {
          window.location.href = '/auth/login'
          throw new Error('Unauthorized')
        }
        if (!res.ok) throw new Error('Upload failed')
        return res.json()
      })
  },

  logout: () => request<void>('/auth/logout', { method: 'POST' }),
}
