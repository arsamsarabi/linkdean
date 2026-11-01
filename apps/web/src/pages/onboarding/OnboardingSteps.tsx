import { type ReactElement, type RefObject } from 'react'
import {
  Button,
  FileButton,
  FileInput,
  Group,
  Paper,
  PasswordInput,
  Select,
  Stack,
  Stepper,
  Text,
  TextInput,
} from '@mantine/core'
import {
  IconLockPlus,
  IconPhoto,
  IconPlayerSkipForward,
  IconRocket,
  IconTrash,
  IconUpload,
  IconUserPlus,
} from '@tabler/icons-react'

const iconSm = { size: 16, stroke: 1.5 } as const

/** Welcome and Admin only run for the first account on an instance. */
const SETUP_STEP_COUNT = 2

type Props = {
  firstAccount: boolean
  step: number
  busy: boolean
  displayName: string
  email: string
  password: string
  password2: string
  pendingAvatar: Blob | null
  resetRef: RefObject<(() => void) | null>
  vaultPass: string
  vaultPass2: string
  format: string | null
  file: File | null
  onStep: (step: number) => void
  onDisplayName: (v: string) => void
  onEmail: (v: string) => void
  onPassword: (v: string) => void
  onPassword2: (v: string) => void
  onPickAvatar: (f: File | null) => void
  onClearAvatar: () => void
  onCreateAdmin: () => void
  onVaultPass: (v: string) => void
  onVaultPass2: (v: string) => void
  onSetupVault: () => void
  onFormat: (v: string | null) => void
  onFile: (f: File | null) => void
  onImportOrSkip: (skip: boolean) => void
}

export function OnboardingSteps({
  firstAccount,
  step,
  busy,
  displayName,
  email,
  password,
  password2,
  pendingAvatar,
  resetRef,
  vaultPass,
  vaultPass2,
  format,
  file,
  onStep,
  onDisplayName,
  onEmail,
  onPassword,
  onPassword2,
  onPickAvatar,
  onClearAvatar,
  onCreateAdmin,
  onVaultPass,
  onVaultPass2,
  onSetupVault,
  onFormat,
  onFile,
  onImportOrSkip,
}: Props): ReactElement {
  return (
    <Stepper active={firstAccount ? step : step - SETUP_STEP_COUNT} allowNextStepsSelect={false}>
      {firstAccount ? (
        <Stepper.Step label="Welcome" description="What this is">
          <Paper p="xl" mt="md" withBorder>
            <Stack>
              <Text>Self-hosted bookmarks. You own the data. Import from browser / Pocket / Raindrop next.</Text>
              <Text size="sm" c="dimmed">
                Next: create your admin account.
              </Text>
              <Button leftSection={<IconRocket {...iconSm} />} onClick={() => onStep(1)}>
                Get started
              </Button>
            </Stack>
          </Paper>
        </Stepper.Step>
      ) : null}

      {firstAccount ? (
        <Stepper.Step label="Admin" description="Your account">
          <Paper p="xl" mt="md" withBorder>
            <Stack>
              <TextInput
                label="Display name"
                placeholder="Ada"
                value={displayName}
                onChange={(e) => onDisplayName(e.currentTarget.value)}
              />
              <TextInput label="Email" required value={email} onChange={(e) => onEmail(e.currentTarget.value)} />
              <PasswordInput
                label="Password"
                required
                description="Min 8 characters"
                value={password}
                onChange={(e) => onPassword(e.currentTarget.value)}
              />
              <PasswordInput
                label="Confirm password"
                required
                value={password2}
                onChange={(e) => onPassword2(e.currentTarget.value)}
              />
              <Group>
                <FileButton
                  resetRef={resetRef}
                  accept="image/jpeg,image/png,image/webp,image/gif"
                  onChange={onPickAvatar}
                >
                  {(props) => (
                    <Button {...props} variant="light" leftSection={<IconPhoto {...iconSm} />}>
                      {pendingAvatar ? 'Avatar ready — change' : 'Optional avatar'}
                    </Button>
                  )}
                </FileButton>
                {pendingAvatar ? (
                  <Button variant="subtle" color="red" leftSection={<IconTrash {...iconSm} />} onClick={onClearAvatar}>
                    Clear
                  </Button>
                ) : null}
              </Group>
              <Button
                leftSection={<IconUserPlus {...iconSm} />}
                loading={busy}
                onClick={onCreateAdmin}
                disabled={!email || !password}
              >
                Create admin account
              </Button>
            </Stack>
          </Paper>
        </Stepper.Step>
      ) : null}

      <Stepper.Step label="Vault" description="Optional">
        <Paper p="xl" mt="md" withBorder>
          <Stack>
            <Text>Vault categories encrypt titles/URLs at rest. Skip now — set up later under Vault.</Text>
            <PasswordInput
              label="Vault passphrase"
              value={vaultPass}
              onChange={(e) => onVaultPass(e.currentTarget.value)}
            />
            <PasswordInput
              label="Confirm passphrase"
              value={vaultPass2}
              onChange={(e) => onVaultPass2(e.currentTarget.value)}
            />
            <Group>
              <Button leftSection={<IconLockPlus {...iconSm} />} loading={busy} onClick={onSetupVault}>
                Enable vault
              </Button>
              <Button variant="subtle" leftSection={<IconPlayerSkipForward {...iconSm} />} onClick={() => onStep(3)}>
                Skip for now
              </Button>
            </Group>
          </Stack>
        </Paper>
      </Stepper.Step>

      <Stepper.Step label="Import" description="Optional">
        <Paper p="xl" mt="md" withBorder>
          <Stack>
            <Text>Bring bookmarks from elsewhere, or start empty.</Text>
            <Select
              label="Format"
              value={format}
              onChange={onFormat}
              data={[
                { value: 'netscape', label: 'Netscape HTML (Chrome/Firefox)' },
                { value: 'json', label: 'Linkdean JSON' },
                { value: 'pocket', label: 'Pocket CSV' },
                { value: 'raindrop', label: 'Raindrop CSV' },
              ]}
            />
            <FileInput label="File" value={file} onChange={onFile} clearable />
            <Group>
              <Button
                leftSection={<IconUpload {...iconSm} />}
                loading={busy}
                disabled={!file}
                onClick={() => onImportOrSkip(false)}
              >
                Import & finish
              </Button>
              <Button
                variant="light"
                leftSection={<IconPlayerSkipForward {...iconSm} />}
                loading={busy}
                onClick={() => onImportOrSkip(true)}
              >
                Skip & finish
              </Button>
            </Group>
          </Stack>
        </Paper>
      </Stepper.Step>
    </Stepper>
  )
}
