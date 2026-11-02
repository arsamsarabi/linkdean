import { useState } from 'react'
import { notifications } from '@mantine/notifications'
import { useMe } from '@/hooks/auth'
import { useUpdateSettings } from '@/hooks/settings'
import type { LibraryViewMode } from './types'

/** Sync grid/list/explorer preference with user settings. */
export function useLibraryViewMode(): {
  viewMode: LibraryViewMode
  onViewModeChange: (next: LibraryViewMode) => void
} {
  const me = useMe()
  const updateSettings = useUpdateSettings()
  const settingsView: LibraryViewMode = me.data?.settings.library_view ?? 'grid'
  const [viewMode, setViewMode] = useState<LibraryViewMode>(settingsView)
  const [viewFromSettings, setViewFromSettings] = useState<LibraryViewMode | null>(null)
  if (me.data && viewFromSettings !== settingsView) {
    setViewFromSettings(settingsView)
    setViewMode(settingsView)
  }

  function onViewModeChange(next: LibraryViewMode): void {
    setViewMode(next)
    if (next === settingsView || updateSettings.isPending) return
    updateSettings.mutate(
      { library_view: next },
      {
        onError: () => {
          setViewMode(settingsView)
          notifications.show({ color: 'red', message: 'Could not save view preference' })
        },
      }
    )
  }

  return { viewMode, onViewModeChange }
}
