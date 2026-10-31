import { useEffect, useRef, useState, type ReactElement } from 'react'
import { useNavigate } from '@tanstack/react-router'
import {
  Button,
  Center,
  FileButton,
  FileInput,
  Group,
  Loader,
  Paper,
  PasswordInput,
  Select,
  Stack,
  Stepper,
  Text,
  TextInput,
  Title,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import {
  IconLockPlus,
  IconPhoto,
  IconPlayerSkipForward,
  IconRocket,
  IconTrash,
  IconUpload,
  IconUserPlus,
} from '@tabler/icons-react'
import { getToken } from '@/lib'
import { AvatarCropModal } from '@/components'
import { useBootstrap, useCompleteOnboarding, useRegister, useMe, useImportBookmarks, useUploadAvatar, useVaultSetup } from '@/hooks'
import { validatePasswordPair } from '@/services'
import motion from '@/styles/motion.module.css'

const iconSm = { size: 16, stroke: 1.5 } as const

const VAULT_PW_MESSAGES = {
  tooShort: 'Vault passphrase min 8 chars',
  mismatch: 'Passphrases mismatch',
} as const

export function OnboardingPage(): ReactElement {
  const nav = useNavigate()
  const [step, setStep] = useState(0)
  const [busy, setBusy] = useState(false)
  const [ready, setReady] = useState(false)

  const [displayName, setDisplayName] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [password2, setPassword2] = useState('')
  const [cropFile, setCropFile] = useState<File | null>(null)
  const [cropOpen, setCropOpen] = useState(false)
  const [pendingAvatar, setPendingAvatar] = useState<Blob | null>(null)
  const resetRef = useRef<() => void>(null)

  const [vaultPass, setVaultPass] = useState('')
  const [vaultPass2, setVaultPass2] = useState('')

  const [format, setFormat] = useState<string | null>('netscape')
  const [file, setFile] = useState<File | null>(null)

  const bootstrap = useBootstrap()
  const me = useMe(!!getToken())
  const register = useRegister()
  const uploadAvatar = useUploadAvatar()
  const vaultSetup = useVaultSetup()
  const importBookmarks = useImportBookmarks()
  const completeOnboarding = useCompleteOnboarding()

  useEffect(() => {
    if (bootstrap.isLoading) return
    if (bootstrap.isError) {
      void nav({ to: '/login' })
      return
    }
    if (!bootstrap.data) return

    if (bootstrap.data.needs_bootstrap) {
      setStep(0)
      setReady(true)
      return
    }

    const token = getToken()
    if (!token) {
      void nav({ to: '/login' })
      return
    }

    if (me.isLoading) return
    if (me.isError) {
      void nav({ to: '/login' })
      return
    }
    if (!me.data) return

    if (me.data.user.onboarding_completed) {
      void nav({ to: '/library' })
      return
    }
    setStep(2)
    setReady(true)
  }, [bootstrap.isLoading, bootstrap.isError, bootstrap.data, me.isLoading, me.isError, me.data, nav])

  async function createAdmin(): Promise<void> {
    const check = validatePasswordPair(password, password2)
    if (!check.ok) {
      notifications.show({ color: 'red', message: check.message })
      return
    }
    setBusy(true)
    try {
      const res = await register.mutateAsync({
        email,
        password,
        display_name: displayName || undefined,
      })
      if (pendingAvatar) {
        try {
          await uploadAvatar.mutateAsync(pendingAvatar)
        } catch (e) {
          notifications.show({
            color: 'yellow',
            message: `Account created; avatar failed: ${e}`,
          })
        }
      }
      notifications.show({
        message: res.user.is_admin ? 'Admin account created' : 'Account created',
      })
      setStep(2)
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    } finally {
      setBusy(false)
    }
  }

  async function setupVault(): Promise<void> {
    const check = validatePasswordPair(vaultPass, vaultPass2, VAULT_PW_MESSAGES)
    if (!check.ok) {
      notifications.show({ color: 'red', message: check.message })
      return
    }
    setBusy(true)
    try {
      await vaultSetup.mutateAsync(vaultPass)
      notifications.show({ message: 'Vault ready' })
      setStep(3)
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    } finally {
      setBusy(false)
    }
  }

  async function doImportOrSkip(skip: boolean): Promise<void> {
    setBusy(true)
    try {
      if (!skip && file && format) {
        const content = await file.text()
        const res = await importBookmarks.mutateAsync({ format, content })
        notifications.show({ message: `Imported ${res.imported}/${res.total}` })
      }
      await completeOnboarding.mutateAsync()
      void nav({ to: '/library' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    } finally {
      setBusy(false)
    }
  }

  if (!ready) {
    return (
      <Center mih="70vh">
        <Loader color="violet" type="dots" />
      </Center>
    )
  }

  return (
    <Stack maw={560} mx="auto" mt={48} px="md" pb={48} gap="lg" className={motion.fadeIn}>
      <Stack gap={6}>
        <Title order={1} c="violet">
          Welcome to Linkdean
        </Title>
        <Text c="dimmed">
          First account becomes <strong>admin</strong>. Nested cats, Meilisearch, vault for sensitive
          links.
        </Text>
      </Stack>

      <Stepper active={step} allowNextStepsSelect={false}>
        <Stepper.Step label="Welcome" description="What this is">
          <Paper p="xl" mt="md" withBorder>
            <Stack>
              <Text>
                Self-hosted bookmarks. You own the data. Import from browser / Pocket / Raindrop next.
              </Text>
              <Text size="sm" c="dimmed">
                Next: create your admin account.
              </Text>
              <Button leftSection={<IconRocket {...iconSm} />} onClick={() => setStep(1)}>
                Get started
              </Button>
            </Stack>
          </Paper>
        </Stepper.Step>

        <Stepper.Step label="Admin" description="Your account">
          <Paper p="xl" mt="md" withBorder>
            <Stack>
              <TextInput
                label="Display name"
                placeholder="Ada"
                value={displayName}
                onChange={(e) => setDisplayName(e.currentTarget.value)}
              />
              <TextInput
                label="Email"
                required
                value={email}
                onChange={(e) => setEmail(e.currentTarget.value)}
              />
              <PasswordInput
                label="Password"
                required
                description="Min 8 characters"
                value={password}
                onChange={(e) => setPassword(e.currentTarget.value)}
              />
              <PasswordInput
                label="Confirm password"
                required
                value={password2}
                onChange={(e) => setPassword2(e.currentTarget.value)}
              />
              <Group>
                <FileButton
                  resetRef={resetRef}
                  accept="image/jpeg,image/png,image/webp,image/gif"
                  onChange={(f) => {
                    if (f) {
                      setCropFile(f)
                      setCropOpen(true)
                    }
                  }}
                >
                  {(props) => (
                    <Button {...props} variant="light" leftSection={<IconPhoto {...iconSm} />}>
                      {pendingAvatar ? 'Avatar ready — change' : 'Optional avatar'}
                    </Button>
                  )}
                </FileButton>
                {pendingAvatar ? (
                  <Button
                    variant="subtle"
                    color="red"
                    leftSection={<IconTrash {...iconSm} />}
                    onClick={() => {
                      setPendingAvatar(null)
                      resetRef.current?.()
                    }}
                  >
                    Clear
                  </Button>
                ) : null}
              </Group>
              <Button
                leftSection={<IconUserPlus {...iconSm} />}
                loading={busy}
                onClick={() => void createAdmin()}
                disabled={!email || !password}
              >
                Create admin account
              </Button>
            </Stack>
          </Paper>
        </Stepper.Step>

        <Stepper.Step label="Vault" description="Optional">
          <Paper p="xl" mt="md" withBorder>
            <Stack>
              <Text>
                Vault categories encrypt titles/URLs at rest. Skip now — set up later under Vault.
              </Text>
              <PasswordInput
                label="Vault passphrase"
                value={vaultPass}
                onChange={(e) => setVaultPass(e.currentTarget.value)}
              />
              <PasswordInput
                label="Confirm passphrase"
                value={vaultPass2}
                onChange={(e) => setVaultPass2(e.currentTarget.value)}
              />
              <Group>
                <Button
                  leftSection={<IconLockPlus {...iconSm} />}
                  loading={busy}
                  onClick={() => void setupVault()}
                >
                  Enable vault
                </Button>
                <Button
                  variant="subtle"
                  leftSection={<IconPlayerSkipForward {...iconSm} />}
                  onClick={() => setStep(3)}
                >
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
                onChange={setFormat}
                data={[
                  { value: 'netscape', label: 'Netscape HTML (Chrome/Firefox)' },
                  { value: 'json', label: 'Linkdean JSON' },
                  { value: 'pocket', label: 'Pocket CSV' },
                  { value: 'raindrop', label: 'Raindrop CSV' },
                ]}
              />
              <FileInput label="File" value={file} onChange={setFile} clearable />
              <Group>
                <Button
                  leftSection={<IconUpload {...iconSm} />}
                  loading={busy}
                  disabled={!file}
                  onClick={() => void doImportOrSkip(false)}
                >
                  Import & finish
                </Button>
                <Button
                  variant="light"
                  leftSection={<IconPlayerSkipForward {...iconSm} />}
                  loading={busy}
                  onClick={() => void doImportOrSkip(true)}
                >
                  Skip & finish
                </Button>
              </Group>
            </Stack>
          </Paper>
        </Stepper.Step>
      </Stepper>

      <AvatarCropModal
        opened={cropOpen}
        file={cropFile}
        onClose={() => {
          setCropOpen(false)
          setCropFile(null)
          resetRef.current?.()
        }}
        onCropped={(blob) => {
          setPendingAvatar(blob)
          notifications.show({ message: 'Avatar cropped — uploads with account' })
        }}
      />
    </Stack>
  )
}
