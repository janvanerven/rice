import type { Attribution as AttributionType } from '../../../types'
import styles from './Attribution.module.css'

interface AttributionProps {
  attribution: AttributionType
}

export function Attribution({ attribution }: AttributionProps) {
  return (
    <div className={styles.container}>
      <span className={styles.text}>
        Photo by{' '}
        <a
          href={`${attribution.author_url}?utm_source=rice&utm_medium=referral`}
          target="_blank"
          rel="noopener noreferrer"
          className={styles.link}
          onClick={(e) => e.stopPropagation()}
          aria-label={`Photo by ${attribution.author_name} on Unsplash`}
        >
          {attribution.author_name}
        </a>
        {' / '}
        <a
          href="https://unsplash.com/?utm_source=rice&utm_medium=referral"
          target="_blank"
          rel="noopener noreferrer"
          className={styles.link}
          onClick={(e) => e.stopPropagation()}
        >
          Unsplash
        </a>
      </span>
    </div>
  )
}
