import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { useState, useEffect, createContext, useContext } from 'react'
import type { User } from './types'
import { api } from './lib/api'
import LoginPage from './pages/LoginPage'
import DashboardPage from './pages/DashboardPage'
import TripNewPage from './pages/TripNewPage'
import TripDetailPage from './pages/TripDetailPage'
import { AutoCoverProvider } from './components/AutoCoverContext'

interface AuthContextType {
  user: User | null
  loading: boolean
  logout: () => Promise<void>
}

export const AuthContext = createContext<AuthContextType>({
  user: null,
  loading: true,
  logout: async () => {},
})

export function useAuth() {
  return useContext(AuthContext)
}

function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<User | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    api.me()
      .then(setUser)
      .catch(() => setUser(null))
      .finally(() => setLoading(false))
  }, [])

  const logout = async () => {
    await api.logout()
    setUser(null)
    window.location.href = '/auth/login'
  }

  return (
    <AuthContext.Provider value={{ user, loading, logout }}>
      {children}
    </AuthContext.Provider>
  )
}

function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { user, loading } = useAuth()

  if (loading) return <div className="loading-scanner" />
  if (!user) {
    window.location.href = '/auth/login'
    return null
  }

  return <>{children}</>
}


export default function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <AutoCoverProvider>
          <Routes>
          <Route path="/login" element={<LoginPage />} />
          <Route
            path="/"
            element={
              <ProtectedRoute>
                <DashboardPage />
              </ProtectedRoute>
            }
          />
          <Route
            path="/trips/new"
            element={
              <ProtectedRoute>
                <TripNewPage />
              </ProtectedRoute>
            }
          />
          <Route
            path="/trips/:id"
            element={
              <ProtectedRoute>
                <TripDetailPage />
              </ProtectedRoute>
            }
          />
          <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </AutoCoverProvider>
      </AuthProvider>
    </BrowserRouter>
  )
}
