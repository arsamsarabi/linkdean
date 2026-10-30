import { useEffect, useRef } from 'react'
import { useMantineColorScheme } from '@mantine/core'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api, type UserSettings } from '../api'
import { useUi } from '../store'

/** Hydrate show-hidden, theme, vault unlock flag, and idle auto-lock from /auth/me. */
export function usePrefsHydration(enabled: boolean) {
  const qc = useQueryClient()
  const setShowHidden = useUi((s) => s.setShowHidden)
  const setVaultUnlocked = useUi((s) => s.setVaultUnlocked)
  const vaultUnlocked = useUi((s) => s.vaultUnlocked)
  const { setColorScheme } = useMantineColorScheme()
  const appliedTheme = useRef<string | null>(null)
  const me = useQuery({
    queryKey: ['me'],
    queryFn: api.me,
    enabled,
  })

  useEffect(() => {
    if (!me.data) return
    const unlocked = me.data.vault_unlocked
    const show = me.data.settings.show_hidden
    const theme = me.data.settings.theme

    // Only write when value changes — Zustand set() always new state object.
    const st = useUi.getState()
    if (st.vaultUnlocked !== unlocked) setVaultUnlocked(unlocked)
    if (st.showHidden !== show) setShowHidden(show)

    // Sync from server; menu switcher also writes theme — keep ref in sync
    if (appliedTheme.current !== theme) {
      appliedTheme.current = theme
      applyTheme(theme, setColorScheme)
    }
  }, [
    me.data?.vault_unlocked,
    me.data?.settings.show_hidden,
    me.data?.settings.theme,
    setVaultUnlocked,
    setShowHidden,
    setColorScheme,
  ])

  useVaultAutoLock(me.data?.settings, vaultUnlocked, qc)

  return me
}

function applyTheme(
  theme: UserSettings['theme'],
  setColorScheme: (v: 'light' | 'dark' | 'auto') => void,
) {
  if (theme === 'system') setColorScheme('auto')
  else setColorScheme(theme)
}

function useVaultAutoLock(
  settings: UserSettings | undefined,
  vaultUnlocked: boolean,
  qc: ReturnType<typeof useQueryClient>,
) {
  const setVaultUnlocked = useUi((s) => s.setVaultUnlocked)
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const minutes = settings?.vault_auto_lock_minutes ?? 0

  useEffect(() => {
    if (!vaultUnlocked || minutes <= 0) {
      if (timer.current) clearTimeout(timer.current)
      return
    }

    const reset = () => {
      if (timer.current) clearTimeout(timer.current)
      timer.current = setTimeout(async () => {
        try {
          await api.vaultLock()
        } catch {
          /* ignore */
        }
        setVaultUnlocked(false)
        qc.invalidateQueries({ queryKey: ['vault'] })
        qc.invalidateQueries({ queryKey: ['me'] })
        qc.invalidateQueries({ queryKey: ['categories'] })
        qc.invalidateQueries({ queryKey: ['bookmarks'] })
      }, minutes * 60_000)
    }

    const events = ['mousemove', 'keydown', 'click', 'scroll', 'touchstart'] as const
    events.forEach((e) => window.addEventListener(e, reset, { passive: true }))
    reset()
    return () => {
      events.forEach((e) => window.removeEventListener(e, reset))
      if (timer.current) clearTimeout(timer.current)
    }
  }, [vaultUnlocked, minutes, setVaultUnlocked, qc])
}

export { applyTheme }
