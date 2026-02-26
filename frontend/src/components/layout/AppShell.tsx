import { useMediaQuery } from '../../hooks/useMediaQuery'
import SideNav from './SideNav'
import BottomNav from './BottomNav'
import styles from './AppShell.module.css'

interface AppShellProps {
  children: React.ReactNode
}

export default function AppShell({ children }: AppShellProps) {
  const isDesktop = useMediaQuery('(min-width: 1024px)')

  return (
    <div className={styles.shell}>
      {isDesktop ? <SideNav /> : <BottomNav />}
      <main className={isDesktop ? styles.mainDesktop : styles.mainMobile}>
        {children}
      </main>
    </div>
  )
}
