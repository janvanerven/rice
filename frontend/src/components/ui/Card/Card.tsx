import type { HTMLAttributes } from 'react'
import styles from './Card.module.css'

export interface CardProps extends HTMLAttributes<HTMLDivElement> {
  active?: boolean
  onClick?: () => void
  className?: string
  children: React.ReactNode
}

export function Card({ active, onClick, className, children, ...props }: CardProps) {
  const isClickable = typeof onClick === 'function'

  return (
    <div
      className={[styles.card, className].filter(Boolean).join(' ')}
      data-active={active ? 'true' : undefined}
      data-clickable={isClickable ? 'true' : undefined}
      onClick={onClick}
      role={isClickable ? 'button' : undefined}
      tabIndex={isClickable ? 0 : undefined}
      onKeyDown={
        isClickable
          ? (e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault()
                onClick()
              }
            }
          : undefined
      }
      {...props}
    >
      {children}
    </div>
  )
}
