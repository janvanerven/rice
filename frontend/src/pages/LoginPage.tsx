import { Navigate } from 'react-router-dom'
import { useAuth } from '../App'
import LoginScreen from '../components/auth/LoginScreen'

export default function LoginPage() {
  const { user, loading } = useAuth()

  if (loading) return <div className="loading-scanner" />
  if (user) return <Navigate to="/" replace />

  return <LoginScreen />
}
