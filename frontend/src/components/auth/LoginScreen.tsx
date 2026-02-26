import { Button, GlowDivider } from '../ui'
import styles from './LoginScreen.module.css'

export default function LoginScreen() {
  const handleSignIn = () => {
    window.location.href = '/auth/login'
  }

  return (
    <main className={styles.root}>
      <div className={styles.content}>
        {/* Kanji mark */}
        <div className={styles.mark} aria-hidden="true">旅</div>

        {/* Brand wordmark */}
        <div className={styles.wordmark}>
          <span className={styles.appName}>RICE</span>
          <span className={styles.tagline}>plan your journey</span>
        </div>

        <GlowDivider className={styles.divider} />

        {/* Sign in action */}
        <div className={styles.actions}>
          <Button
            variant="primary"
            size="lg"
            className={styles.signInButton}
            onClick={handleSignIn}
          >
            Sign in with Authentik
          </Button>
        </div>
      </div>
    </main>
  )
}
