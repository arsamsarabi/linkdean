import { useEffect, type ReactElement } from 'react'
import { notifications } from '@mantine/notifications'
import { useNavigate } from '@tanstack/react-router'
import { useMe } from '@/hooks'
import { LibraryPage } from './LibraryPage'

/** Library scoped to open vaults. Leaves as soon as none are open. */
export function VaultsPage(): ReactElement | null {
  const nav = useNavigate()
  const me = useMe()
  const allLocked = !!me.data && !me.data.vault_unlocked

  useEffect(() => {
    if (!allLocked) return
    notifications.show({ message: 'No vaults are open' })
    void nav({ to: '/library', replace: true })
  }, [allLocked, nav])

  if (!me.data || allLocked) return null
  return <LibraryPage scope="vault" />
}
