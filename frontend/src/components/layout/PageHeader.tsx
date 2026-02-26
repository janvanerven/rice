import styles from './PageHeader.module.css'

interface PageHeaderProps {
  title: string
  action?: React.ReactNode
}

export default function PageHeader({ title, action }: PageHeaderProps) {
  return (
    <header className={styles.header}>
      <h1 className={styles.title}>{title}</h1>
      {action && <div className={styles.action}>{action}</div>}
    </header>
  )
}
