import type { HTMLAttributes } from 'react'
import styles from './Badge.module.css'

export type BadgeVariant = 'primary' | 'success' | 'warning' | 'danger' | 'neutral'

export interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  variant?: BadgeVariant
  children: React.ReactNode
}

export function Badge({ variant = 'neutral', children, className, ...props }: BadgeProps) {
  return (
    <span
      className={[styles.badge, className].filter(Boolean).join(' ')}
      data-variant={variant}
      {...props}
    >
      {children}
    </span>
  )
}
