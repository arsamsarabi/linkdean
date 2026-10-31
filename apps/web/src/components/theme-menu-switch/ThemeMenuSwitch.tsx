import type { ReactElement } from 'react'
import { SegmentedControl, Stack, Text, useMantineColorScheme } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconDeviceDesktop, IconMoon, IconSun } from '@tabler/icons-react'
import type { UserSettings } from '@/schemas'
import { useMe, applyTheme, useUpdateSettings } from '@/hooks'

const icon = { size: 14, stroke: 1.5 } as const

type ThemeValue = UserSettings['theme']

export function ThemeMenuSwitch(): ReactElement {
  const { setColorScheme } = useMantineColorScheme()
  const me = useMe()
  const value: ThemeValue = me.data?.settings.theme ?? 'system'
  const save = useUpdateSettings()

  function onChange(v: string): void {
    const theme = v as ThemeValue
    applyTheme(theme, setColorScheme)
    save.mutate(
      { theme },
      {
        onError: (e) => notifications.show({ color: 'red', message: String(e) }),
      },
    )
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
          {
            value: 'light',
            label: <IconSun {...icon} aria-label="Light theme" />,
          },
          {
            value: 'dark',
            label: <IconMoon {...icon} aria-label="Dark theme" />,
          },
          {
            value: 'system',
            label: <IconDeviceDesktop {...icon} aria-label="System theme" />,
          },
        ]}
      />
    </Stack>
  )
}
