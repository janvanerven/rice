import { useState } from 'react'
import { Input, Button, Badge } from '../ui'
import type { BadgeVariant } from '../ui'
import type { TripMember } from '../../types'
import { api } from '../../lib/api'
import styles from './CollaboratorList.module.css'

interface CollaboratorListProps {
  tripId: string
  members: TripMember[]
  userRole: string
  onMemberRemoved: () => void
}

// Deterministic color from a string for avatar fallback backgrounds
function avatarColor(str: string): string {
  const colors = [
    'rgba(255, 107, 43, 0.25)',  // neon-primary
    'rgba(0, 212, 255, 0.2)',    // neon-cyan
    'rgba(255, 0, 128, 0.2)',    // neon-pink
    'rgba(255, 184, 48, 0.2)',   // neon-amber
    'rgba(255, 45, 85, 0.2)',    // neon-red
  ]
  let hash = 0
  for (let i = 0; i < str.length; i++) {
    hash = str.charCodeAt(i) + ((hash << 5) - hash)
  }
  return colors[Math.abs(hash) % colors.length]
}

function roleBadgeVariant(role: string): BadgeVariant {
  switch (role.toLowerCase()) {
    case 'owner':   return 'primary'
    case 'editor':  return 'success'
    default:        return 'neutral'
  }
}

function roleLabel(role: string): string {
  return role.charAt(0).toUpperCase() + role.slice(1).toLowerCase()
}

export function CollaboratorList({
  tripId,
  members,
  userRole,
  onMemberRemoved,
}: CollaboratorListProps) {
  const isOwner = userRole.toLowerCase() === 'owner'

  // Invite form state
  const [inviteEmail, setInviteEmail] = useState('')
  const [inviteRole, setInviteRole] = useState<'editor' | 'viewer'>('editor')
  const [inviteLoading, setInviteLoading] = useState(false)
  const [inviteError, setInviteError] = useState<string | null>(null)
  const [inviteSuccess, setInviteSuccess] = useState<string | null>(null)
  const [inviteEmailError, setInviteEmailError] = useState<string | undefined>(undefined)

  // Per-member remove loading state
  const [removingId, setRemovingId] = useState<string | null>(null)

  const handleRemove = async (userId: string, displayName: string) => {
    if (!window.confirm(`Remove ${displayName} from this trip?`)) return
    setRemovingId(userId)
    try {
      await api.members.remove(tripId, userId)
      onMemberRemoved()
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to remove member')
    } finally {
      setRemovingId(null)
    }
  }

  const handleInvite = async (e: React.FormEvent) => {
    e.preventDefault()
    setInviteError(null)
    setInviteSuccess(null)
    setInviteEmailError(undefined)

    if (!inviteEmail.trim()) {
      setInviteEmailError('Email is required')
      return
    }

    const emailPattern = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
    if (!emailPattern.test(inviteEmail.trim())) {
      setInviteEmailError('Enter a valid email address')
      return
    }

    setInviteLoading(true)
    try {
      await api.invites.create(tripId, inviteEmail.trim(), inviteRole)
      setInviteSuccess(`Invite sent to ${inviteEmail.trim()}`)
      setInviteEmail('')
      setInviteRole('editor')
    } catch (err) {
      setInviteError(err instanceof Error ? err.message : 'Failed to send invite')
    } finally {
      setInviteLoading(false)
    }
  }

  return (
    <section className={styles.section} aria-label="Collaborators">
      <h3 className={styles.sectionTitle}>
        <span className={styles.sectionTitleLabel}>CREW</span>
        <span className={styles.memberCount}>{members.length}</span>
      </h3>

      <ul className={styles.memberList} aria-label="Trip members">
        {members.map((member) => (
          <li key={member.user_id} className={styles.memberItem}>
            {/* Avatar */}
            <div
              className={styles.avatar}
              style={{ background: avatarColor(member.user_id) }}
              aria-hidden="true"
            >
              {member.avatar_url ? (
                <img
                  src={member.avatar_url}
                  alt=""
                  className={styles.avatarImage}
                />
              ) : (
                <span className={styles.avatarInitial}>
                  {member.display_name.charAt(0).toUpperCase()}
                </span>
              )}
            </div>

            {/* Info */}
            <div className={styles.memberInfo}>
              <span className={styles.memberName}>{member.display_name}</span>
              <span className={styles.memberEmail}>{member.email}</span>
            </div>

            {/* Role + actions */}
            <div className={styles.memberMeta}>
              <Badge variant={roleBadgeVariant(member.role)}>
                {roleLabel(member.role)}
              </Badge>

              {isOwner && member.role.toLowerCase() !== 'owner' && (
                <button
                  className={styles.removeButton}
                  onClick={() => handleRemove(member.user_id, member.display_name)}
                  disabled={removingId === member.user_id}
                  aria-label={`Remove ${member.display_name}`}
                  title="Remove from trip"
                >
                  {removingId === member.user_id ? '…' : '✕'}
                </button>
              )}
            </div>
          </li>
        ))}
      </ul>

      {/* Invite section — only owners can invite */}
      {isOwner && (
        <div className={styles.inviteSection}>
          <h4 className={styles.inviteTitle}>Invite Collaborator</h4>

          {inviteError && (
            <div className={styles.inviteError} role="alert">
              <span aria-hidden="true">⚠</span> {inviteError}
            </div>
          )}
          {inviteSuccess && (
            <div className={styles.inviteSuccess} role="status">
              <span aria-hidden="true">✓</span> {inviteSuccess}
            </div>
          )}

          <form className={styles.inviteForm} onSubmit={handleInvite} noValidate>
            <Input
              label="Email Address"
              id="invite-email"
              type="email"
              value={inviteEmail}
              onChange={(e) => {
                setInviteEmail(e.target.value)
                setInviteEmailError(undefined)
                setInviteSuccess(null)
              }}
              placeholder="traveler@example.com"
              error={inviteEmailError}
              disabled={inviteLoading}
              autoComplete="email"
            />

            <div className={styles.roleRow}>
              <fieldset className={styles.roleFieldset}>
                <legend className={styles.roleLegend}>Role</legend>
                <div className={styles.roleOptions}>
                  <label className={styles.roleOption}>
                    <input
                      type="radio"
                      name="invite-role"
                      value="editor"
                      checked={inviteRole === 'editor'}
                      onChange={() => setInviteRole('editor')}
                      disabled={inviteLoading}
                      className={styles.roleRadio}
                    />
                    <span className={styles.roleOptionLabel}>Editor</span>
                  </label>
                  <label className={styles.roleOption}>
                    <input
                      type="radio"
                      name="invite-role"
                      value="viewer"
                      checked={inviteRole === 'viewer'}
                      onChange={() => setInviteRole('viewer')}
                      disabled={inviteLoading}
                      className={styles.roleRadio}
                    />
                    <span className={styles.roleOptionLabel}>Viewer</span>
                  </label>
                </div>
              </fieldset>

              <Button
                type="submit"
                variant="secondary"
                size="sm"
                disabled={inviteLoading}
                className={styles.inviteButton}
              >
                {inviteLoading ? 'Sending…' : 'Send Invite'}
              </Button>
            </div>
          </form>
        </div>
      )}
    </section>
  )
}
