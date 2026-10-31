import { useEffect, useRef, type MutableRefObject } from 'react'
import { useMantineColorScheme } from '@mantine/core'
import type { UseQueryResult } from '@tanstack/react-query'
import type { MeResponse, UserSettings } from '@/schemas'
import { useUi } from '@/store'
import { useMe } from '@/hooks/auth'
import { useVaultLock } from '@/hooks/vault'

/** Hydrate show-hidden, theme, vault unlock flag, and idle auto-lock from /auth/me. */
export function usePrefsHydration(enabled: boolean): UseQueryResult<MeResponse> {
  const setShowHidden = useUi((s) => s.setShowHidden)
  const setVaultUnlocked = useUi((s) => s.setVaultUnlocked)
  const vaultUnlocked = useUi((s) => s.vaultUnlocked)
  const { setColorScheme } = useMantineColorScheme()
  const appliedTheme = useRef<string | null>(null)
  const me = useMe(enabled)
  const vaultLock = useVaultLock()
  const lockRef = useRef(vaultLock.mutateAsync)

  useEffect(() => {
    lockRef.current = vaultLock.mutateAsync
  }, [vaultLock.mutateAsync])

  useEffect(() => {
    if (!me.data) return
    const unlocked = me.data.vault_unlocked
    const show = me.data.settings.show_hidden
    const theme = me.data.settings.theme

    const st = useUi.getState()
    if (st.vaultUnlocked !== unlocked) setVaultUnlocked(unlocked)
    if (st.showHidden !== show) setShowHidden(show)

    if (appliedTheme.current !== theme) {
      appliedTheme.current = theme
      applyTheme(theme, setColorScheme)
    }
  }, [
    me.data,
    me.data?.vault_unlocked,
    me.data?.settings.show_hidden,
    me.data?.settings.theme,
    setVaultUnlocked,
    setShowHidden,
    setColorScheme,
  ])

  useVaultAutoLock(me.data?.settings, vaultUnlocked, lockRef)

  return me
}

export function applyTheme(
  theme: UserSettings['theme'],
  setColorScheme: (v: 'light' | 'dark' | 'auto') => void,
): void {
  if (theme === 'system') setColorScheme('auto')
  else setColorScheme(theme)
}

function useVaultAutoLock(
  settings: UserSettings | undefined,
  vaultUnlocked: boolean,
  lockRef: MutableRefObject<() => Promise<unknown>>,
): void {
  const setVaultUnlocked = useUi((s) => s.setVaultUnlocked)
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const minutes = settings?.vault_auto_lock_minutes ?? 0

  useEffect(() => {
    if (!vaultUnlocked || minutes <= 0) {
      if (timer.current) clearTimeout(timer.current)
      return
    }

    const reset = (): void => {
      if (timer.current) clearTimeout(timer.current)
      timer.current = setTimeout(() => {
        void lockRef
          .current()
          .catch((e: unknown) => {
            console.error('Vault auto-lock failed', e)
          })
          .finally(() => {
            setVaultUnlocked(false)
          })
      }, minutes * 60_000)
    }

    const events = ['mousemove', 'keydown', 'click', 'scroll', 'touchstart'] as const
    events.forEach((e) => window.addEventListener(e, reset, { passive: true }))
    reset()
    return () => {
      events.forEach((e) => window.removeEventListener(e, reset))
      if (timer.current) clearTimeout(timer.current)
    }
  }, [vaultUnlocked, minutes, setVaultUnlocked, lockRef])
}
