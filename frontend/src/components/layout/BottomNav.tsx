import { NavLink } from 'react-router-dom'
import { useAuth } from '../../App'
import styles from './BottomNav.module.css'

export default function BottomNav() {
  const { user, logout } = useAuth()

  const avatarLetter = user?.display_name
    ? user.display_name.charAt(0).toUpperCase()
    : '?'

  return (
    <nav className={styles.nav} aria-label="Primary navigation">
      {/* Trips */}
      <NavLink
        to="/"
        end
        className={({ isActive }) =>
          isActive ? `${styles.tab} ${styles.tabActive}` : styles.tab
        }
        aria-label="Trips"
      >
        <span className={styles.tabIcon} aria-hidden="true">◈</span>
        <span className={styles.tabLabel}>Trips</span>
      </NavLink>

      {/* New Trip */}
      <NavLink
        to="/trips/new"
        className={({ isActive }) =>
          isActive ? `${styles.tab} ${styles.tabActive}` : styles.tab
        }
        aria-label="New trip"
      >
        <span className={styles.tabIcon} aria-hidden="true">+</span>
        <span className={styles.tabLabel}>New</span>
      </NavLink>

      {/* Profile / Logout */}
      <button
        onClick={logout}
        className={styles.tab}
        aria-label={`Log out ${user?.display_name ?? ''}`}
      >
        {user?.avatar_url ? (
          <img
            src={user.avatar_url}
            alt={user.display_name}
            className={styles.avatar}
          />
        ) : (
          <div className={styles.avatarFallback} aria-hidden="true">
            {avatarLetter}
          </div>
        )}
        <span className={styles.tabLabel}>Exit</span>
      </button>
    </nav>
  )
}
