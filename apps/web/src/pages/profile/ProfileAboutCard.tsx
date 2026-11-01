import { type ReactElement, type RefObject } from 'react'
import {
  Button,
  Divider,
  Fieldset,
  FileButton,
  Group,
  Paper,
  Select,
  Stack,
  Textarea,
  TextInput,
} from '@mantine/core'
import { IconDeviceFloppy, IconPhoto, IconTrash } from '@tabler/icons-react'
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
  saving: boolean
  resetRef: RefObject<(() => void) | null>
  onDisplayName: (v: string) => void
  onBio: (v: string) => void
  onTimezone: (v: string | null) => void
  onEmail: (v: string) => void
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
  saving,
  resetRef,
  onDisplayName,
  onBio,
  onTimezone,
  onEmail,
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
            <FileButton
              resetRef={resetRef}
              accept="image/jpeg,image/png,image/webp,image/gif"
              onChange={onPickAvatar}
            >
              {(props) => (
                <Button {...props} variant="light" leftSection={<IconPhoto {...iconSm} />}>
                  Upload avatar
                </Button>
              )}
            </FileButton>
            {hasAvatar ? (
              <Button
                variant="subtle"
                color="red"
                leftSection={<IconTrash {...iconSm} />}
                onClick={onRemoveAvatar}
              >
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
            <Textarea
              label="Bio"
              value={bio}
              onChange={(e) => onBio(e.currentTarget.value)}
              minRows={2}
              autosize
            />
            <Select
              label="Timezone"
              searchable
              clearable
              data={TIMEZONES}
              value={timezone}
              onChange={onTimezone}
            />
            <TextInput
              label="Email"
              value={email}
              onChange={(e) => onEmail(e.currentTarget.value)}
            />
          </Stack>
        </Fieldset>

        <Button
          leftSection={<IconDeviceFloppy {...iconSm} />}
          loading={saving}
          onClick={onSave}
        >
          Save profile
        </Button>
      </Stack>
    </Paper>
  )
}
