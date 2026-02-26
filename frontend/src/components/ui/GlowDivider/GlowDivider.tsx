import type { HTMLAttributes } from 'react'
import styles from './GlowDivider.module.css'

export interface GlowDividerProps extends HTMLAttributes<HTMLDivElement> {
  className?: string
}

export function GlowDivider({ className, ...props }: GlowDividerProps) {
  return (
    <div
      className={[styles.divider, className].filter(Boolean).join(' ')}
      role="separator"
      aria-hidden="true"
      {...props}
    />
  )
}
