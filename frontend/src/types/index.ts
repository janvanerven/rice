export interface User {
  id: string
  email: string
  display_name: string
  avatar_url: string | null
  created_at: string
  updated_at: string
}

export interface Trip {
  id: string
  name: string
  destination: string
  start_date: string | null
  end_date: string | null
  cover_image_path: string | null
  created_by: string
  created_at: string
  updated_at: string
  role: string
  attribution: Attribution | null
}

export interface TripMember {
  user_id: string
  email: string
  display_name: string
  avatar_url: string | null
  role: string
  joined_at: string
}

export interface CreateTripRequest {
  name: string
  destination?: string
  start_date?: string
  end_date?: string
}

export interface UpdateTripRequest {
  name?: string
  destination?: string
  start_date?: string
  end_date?: string
}

export interface Accommodation {
  id: string
  trip_id: string
  name: string
  address: string | null
  check_in: string | null
  check_out: string | null
  notes: string | null
  cover_image_path: string | null
  created_at: string
  updated_at: string
  attribution: Attribution | null
}

export interface CreateAccommodationRequest {
  name: string
  address?: string
  check_in?: string
  check_out?: string
  notes?: string
}

export interface UpdateAccommodationRequest {
  name?: string
  address?: string
  check_in?: string
  check_out?: string
  notes?: string
}

export interface Attribution {
  author_name: string
  author_url: string
  source_url: string
}

export interface AutoCoverResponse {
  path: string
  attribution: Attribution
}
