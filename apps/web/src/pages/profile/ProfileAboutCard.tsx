import { type ReactElement, type RefObject } from 'react'
import { Button, Divider, Fieldset, FileButton, Group, Paper, Select, Stack, Textarea, TextInput } from '@mantine/core'
import { IconDeviceFloppy, IconMailCog, IconPhoto, IconTrash } from '@tabler/icons-react'
import { UserAvatar } from '@/components'

const iconSm = { size: 16, stroke: 1.5 } as const

const TIMEZONES = Intl.supportedValuesOf
  ? Intl.supportedValuesOf('timeZone')
  : ['UTC', 'Europe/Amsterdam', 'America/New_York', 'Asia/Tokyo']

type Props = {
  label: string
  hasAvatar: boolean
  avatarBust: number
  displayName: string
  bio: string
  timezone: string | null
  email: string
  /** SSO-only accounts have no password; their identity provider owns the address. */
  hasPassword: boolean
  /** A change needs a confirmation email, so with mail off only an admin can change it. */
  mailEnabled: boolean
  saving: boolean
  resetRef: RefObject<(() => void) | null>
  onDisplayName: (v: string) => void
  onBio: (v: string) => void
  onTimezone: (v: string | null) => void
  onChangeEmail: () => void
  onPickAvatar: (f: File | null) => void
  onRemoveAvatar: () => void
  onSave: () => void
}

export function ProfileAboutCard({
  label,
  hasAvatar,
  avatarBust,
  displayName,
  bio,
  timezone,
  email,
  hasPassword,
  mailEnabled,
  saving,
  resetRef,
  onDisplayName,
  onBio,
  onTimezone,
  onChangeEmail,
  onPickAvatar,
  onRemoveAvatar,
  onSave,
}: Props): ReactElement {
  return (
    <Paper p="xl" withBorder>
      <Stack gap="lg">
        <Group gap="lg" align="flex-start">
          <UserAvatar hasAvatar={hasAvatar} label={label} size={80} bust={avatarBust} />
          <Stack gap="xs">
            <FileButton resetRef={resetRef} accept="image/jpeg,image/png,image/webp,image/gif" onChange={onPickAvatar}>
              {(props) => (
                <Button {...props} variant="light" leftSection={<IconPhoto {...iconSm} />}>
                  Upload avatar
                </Button>
              )}
            </FileButton>
            {hasAvatar ? (
              <Button variant="subtle" color="red" leftSection={<IconTrash {...iconSm} />} onClick={onRemoveAvatar}>
                Remove
              </Button>
            ) : null}
          </Stack>
        </Group>

        <Divider />

        <Fieldset legend="About you" variant="unstyled">
          <Stack gap="md">
            <TextInput
              label="Display name"
              value={displayName}
              onChange={(e) => onDisplayName(e.currentTarget.value)}
            />
            <Textarea label="Bio" value={bio} onChange={(e) => onBio(e.currentTarget.value)} minRows={2} autosize />
            <Select label="Timezone" searchable clearable data={TIMEZONES} value={timezone} onChange={onTimezone} />
            <Group gap="sm" align="flex-end" wrap="nowrap">
              <TextInput
                label="Email"
                value={email}
                readOnly
                flex={1}
                description={
                  !hasPassword
                    ? "Your identity provider manages this account's email."
                    : !mailEnabled
                      ? 'Ask your admin to change it.'
                      : undefined
                }
              />
              {hasPassword && mailEnabled ? (
                <Button variant="default" leftSection={<IconMailCog {...iconSm} />} onClick={onChangeEmail}>
                  Change email
                </Button>
              ) : null}
            </Group>
          </Stack>
        </Fieldset>

        <Button leftSection={<IconDeviceFloppy {...iconSm} />} loading={saving} onClick={onSave}>
          Save profile
        </Button>
      </Stack>
    </Paper>
  )
}
