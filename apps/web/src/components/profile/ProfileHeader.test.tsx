import type { ReactElement } from 'react'
import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Profile } from '@/schemas'
import { ProfileHeader } from './ProfileHeader'

vi.mock('@/hooks', () => ({ useAvatarImage: () => undefined }))

const profile: Profile = {
  id: 'u1',
  display_name: 'Ana',
  bio: 'Reads a lot',
  has_avatar: false,
  is_self: false,
  follow: { following: false, followers: 3, following_count: 1 },
}

function renderHeader(ui: ReactElement): void {
  render(<MantineProvider>{ui}</MantineProvider>)
}

describe('ProfileHeader', () => {
  afterEach(cleanup)

  it('offers follow, not edit, on someone else', async () => {
    const onToggleFollow = vi.fn()
    renderHeader(
      <ProfileHeader profile={profile} onToggleFollow={onToggleFollow} onEdit={vi.fn()} onOpenList={vi.fn()} />
    )
    expect(screen.queryByRole('button', { name: /edit profile/i })).toBeNull()
    await userEvent.click(screen.getByRole('button', { name: 'Follow' }))
    expect(onToggleFollow).toHaveBeenCalled()
  })

  it('shows email and edit on your own profile', () => {
    renderHeader(
      <ProfileHeader
        profile={{ ...profile, is_self: true, email: 'ana@example.com' }}
        onToggleFollow={vi.fn()}
        onEdit={vi.fn()}
        onOpenList={vi.fn()}
      />
    )
    expect(screen.getByText('ana@example.com')).toBeTruthy()
    expect(screen.getByRole('button', { name: /edit profile/i })).toBeTruthy()
    expect(screen.queryByRole('button', { name: 'Follow' })).toBeNull()
  })

  it('opens the follower list from the count', async () => {
    const onOpenList = vi.fn()
    renderHeader(<ProfileHeader profile={profile} onToggleFollow={vi.fn()} onEdit={vi.fn()} onOpenList={onOpenList} />)
    await userEvent.click(screen.getByRole('button', { name: '3 followers' }))
    expect(onOpenList).toHaveBeenCalledWith('followers')
  })
})
