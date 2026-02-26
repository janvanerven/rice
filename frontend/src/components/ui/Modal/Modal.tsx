import { useEffect, useRef } from 'react'
import type { ReactNode } from 'react'
import styles from './Modal.module.css'

export interface ModalProps {
  open: boolean
  onClose: () => void
  title?: string
  children: ReactNode
}

export function Modal({ open, onClose, title, children }: ModalProps) {
  const dialogRef = useRef<HTMLDivElement>(null)

  // Close on Escape key
  useEffect(() => {
    if (!open) return

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [open, onClose])

  // Trap focus inside modal and restore on close
  useEffect(() => {
    if (!open) return

    const previousFocus = document.activeElement as HTMLElement | null

    // Focus the dialog container on open
    requestAnimationFrame(() => {
      dialogRef.current?.focus()
    })

    return () => {
      previousFocus?.focus()
    }
  }, [open])

  // Lock body scroll while modal is open
  useEffect(() => {
    if (open) {
      document.body.style.overflow = 'hidden'
    } else {
      document.body.style.overflow = ''
    }
    return () => {
      document.body.style.overflow = ''
    }
  }, [open])

  if (!open) return null

  return (
    <div
      className={styles.backdrop}
      onClick={(e) => {
        // Close when clicking the backdrop itself, not the panel
        if (e.target === e.currentTarget) onClose()
      }}
      aria-modal="true"
      role="dialog"
      aria-label={title}
    >
      <div
        ref={dialogRef}
        className={styles.panel}
        tabIndex={-1}
      >
        {title && (
          <div className={styles.header}>
            <h2 className={styles.title}>{title}</h2>
            <button
              className={styles.closeButton}
              onClick={onClose}
              aria-label="Close"
            >
              ✕
            </button>
          </div>
        )}
        <div className={styles.body}>
          {children}
        </div>
      </div>
    </div>
  )
}
