import { useEffect, useRef, useState, type ReactElement } from 'react'
import { Center, Loader, Stack, Text, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useNavigate } from '@tanstack/react-router'
import { AvatarCropModal, RecoveryKeyModal } from '@/components'
import {
  useBootstrap,
  useCompleteOnboarding,
  useCreateVault,
  useImportBookmarks,
  useMe,
  useRegister,
  useUploadAvatar,
} from '@/hooks'
import { getToken, resolveDefaultLanding } from '@/lib'
import { MIN_SECRET_LENGTH, validatePasswordPair } from '@/services'
import motion from '@/styles/motion.module.css'
import { OnboardingSteps } from './OnboardingSteps'

const VAULT_PW_MESSAGES = {
  tooShort: `Vault passphrase min ${MIN_SECRET_LENGTH} chars`,
  mismatch: 'Passphrases mismatch',
} as const

export function OnboardingPage(): ReactElement {
  const nav = useNavigate()
  const [step, setStep] = useState(0)
  const [busy, setBusy] = useState(false)
  const [recoveryKey, setRecoveryKey] = useState<string | null>(null)
  const [ready, setReady] = useState(false)
  const [firstAccount, setFirstAccount] = useState(false)

  const [displayName, setDisplayName] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [password2, setPassword2] = useState('')
  const [bootstrapToken, setBootstrapToken] = useState('')
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
  const createVault = useCreateVault()
  const importBookmarks = useImportBookmarks()
  const completeOnboarding = useCompleteOnboarding()

  useEffect(() => {
    // Only picks the starting step: later query updates must not move a user mid-flow.
    if (ready) return
    if (bootstrap.isLoading) return
    if (bootstrap.isError) {
      void nav({ to: '/login' })
      return
    }
    if (!bootstrap.data) return

    if (bootstrap.data.needs_bootstrap) {
      // oxlint-disable-next-line react/set-state-in-effect -- picks the starting step once async data lands
      setStep(0)
      setFirstAccount(true)
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
      void nav({ to: resolveDefaultLanding(me.data.settings.default_landing) })
      return
    }
    setStep(2)
    setReady(true)
  }, [ready, bootstrap.isLoading, bootstrap.isError, bootstrap.data, me.isLoading, me.isError, me.data, nav])

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
        bootstrap_token: bootstrapToken.trim() || undefined,
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
      const created = await createVault.mutateAsync({ name: 'Vault', passphrase: vaultPass })
      notifications.show({ message: 'Vault ready' })
      setRecoveryKey(created.recovery_key)
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
      void nav({ to: '/' })
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
    <Stack maw={760} mx="auto" mt={48} px="md" pb={48} gap="lg" className={motion.fadeIn}>
      <Stack gap={6}>
        <Title order={1} c="violet">
          Welcome to Linkdean
        </Title>
        {firstAccount ? (
          <Text c="dimmed">
            First account becomes <strong>admin</strong>. Nested cats, Meilisearch, vault for sensitive links.
          </Text>
        ) : (
          <Text c="dimmed">Two optional steps and you're in.</Text>
        )}
      </Stack>

      {recoveryKey ? (
        <RecoveryKeyModal
          recoveryKey={recoveryKey}
          onDone={() => {
            setRecoveryKey(null)
            setStep(3)
          }}
        />
      ) : null}

      <OnboardingSteps
        firstAccount={firstAccount}
        needsBootstrapToken={!!bootstrap.data?.bootstrap_token_required}
        bootstrapToken={bootstrapToken}
        onBootstrapToken={setBootstrapToken}
        step={step}
        busy={busy}
        displayName={displayName}
        email={email}
        password={password}
        password2={password2}
        pendingAvatar={pendingAvatar}
        resetRef={resetRef}
        vaultPass={vaultPass}
        vaultPass2={vaultPass2}
        format={format}
        file={file}
        onStep={setStep}
        onDisplayName={setDisplayName}
        onEmail={setEmail}
        onPassword={setPassword}
        onPassword2={setPassword2}
        onPickAvatar={(f) => {
          if (f) {
            setCropFile(f)
            setCropOpen(true)
          }
        }}
        onClearAvatar={() => {
          setPendingAvatar(null)
          resetRef.current?.()
        }}
        onCreateAdmin={() => void createAdmin()}
        onVaultPass={setVaultPass}
        onVaultPass2={setVaultPass2}
        onSetupVault={() => void setupVault()}
        onFormat={setFormat}
        onFile={setFile}
        onImportOrSkip={(skip) => void doImportOrSkip(skip)}
      />

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
