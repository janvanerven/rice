import { NavLink } from 'react-router-dom'
import { useAuth } from '../../App'
import styles from './SideNav.module.css'

export default function SideNav() {
  const { user, logout } = useAuth()

  const avatarLetter = user?.display_name
    ? user.display_name.charAt(0).toUpperCase()
    : '?'

  return (
    <nav className={styles.nav} aria-label="Primary navigation">
      {/* Logo */}
      <div className={styles.logo} aria-label="Rice">
        <span className={styles.logoText}>Rice</span>
        <span className={styles.logoKanji}>旅</span>
      </div>

      {/* Nav items */}
      <ul className={styles.navList} role="list">
        <li>
          <NavLink
            to="/"
            end
            className={({ isActive }) =>
              isActive ? `${styles.navItem} ${styles.navItemActive}` : styles.navItem
            }
          >
            <span className={styles.navIcon} aria-hidden="true">◈</span>
            <span>Trips</span>
          </NavLink>
        </li>
        <li>
          <NavLink
            to="/trips/new"
            className={({ isActive }) =>
              isActive ? `${styles.navItem} ${styles.navItemActive}` : styles.navItem
            }
          >
            <span className={styles.navIcon} aria-hidden="true">+</span>
            <span>New Trip</span>
          </NavLink>
        </li>
      </ul>

      {/* User section */}
      <div className={styles.userSection}>
        <div className={styles.userInfo}>
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
          <span className={styles.displayName} title={user?.display_name ?? ''}>
            {user?.display_name ?? 'Guest'}
          </span>
        </div>
        <button
          onClick={logout}
          className={styles.logoutBtn}
          aria-label="Log out"
        >
          Exit
        </button>
      </div>
    </nav>
  )
}
