import { SegmentedControl, Stack, Text } from '@mantine/core'
import { useMantineColorScheme } from '@mantine/core'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { IconDeviceDesktop, IconMoon, IconSun } from '@tabler/icons-react'
import { notifications } from '@mantine/notifications'
import { api, type UserSettings } from '../api'
import { applyTheme } from '../hooks/usePrefs'

const icon = { size: 14, stroke: 1.5 } as const

type ThemeValue = UserSettings['theme']

export function ThemeMenuSwitch() {
  const qc = useQueryClient()
  const { setColorScheme } = useMantineColorScheme()
  const me = useQuery({ queryKey: ['me'], queryFn: api.me })
  const value: ThemeValue = me.data?.settings.theme ?? 'system'

  const save = useMutation({
    mutationFn: (theme: ThemeValue) => api.updateSettings({ theme }),
    onSuccess: (res) => {
      qc.setQueryData(['me'], (old: unknown) => {
        if (!old || typeof old !== 'object') return old
        return { ...(old as object), settings: res.settings }
      })
      applyTheme(res.settings.theme, setColorScheme)
    },
    onError: (e) => notifications.show({ color: 'red', message: String(e) }),
  })

  function onChange(v: string) {
    const theme = v as ThemeValue
    applyTheme(theme, setColorScheme)
    save.mutate(theme)
  }

  return (
    <Stack gap={6} px="sm" py={4}>
      <Text size="xs" c="dimmed">
        Theme
      </Text>
      <SegmentedControl
        size="xs"
        fullWidth
        value={value}
        onChange={onChange}
        data={[
          { value: 'light', label: <IconSun {...icon} title="Light" /> },
          { value: 'dark', label: <IconMoon {...icon} title="Dark" /> },
          { value: 'system', label: <IconDeviceDesktop {...icon} title="System" /> },
        ]}
      />
    </Stack>
  )
}
