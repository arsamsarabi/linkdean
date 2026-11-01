import { useEffect, useRef, useState, type ReactElement } from 'react'
import { Center, Loader, Stack } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useNavigate, useSearch } from '@tanstack/react-router'
import { api } from '@/api'
import { AvatarCropModal, QueryErrorAlert } from '@/components'
import { useBootstrap, useRegister, useSyncDuringRender, useUploadAvatar } from '@/hooks'
import { resolveDefaultLanding } from '@/lib'
import { validatePasswordPair } from '@/services'
import { RegisterForm } from './RegisterForm'
import { RegisterInviteGate } from './RegisterInviteGate'

export function RegisterPage(): ReactElement {
  const nav = useNavigate()
  const search = useSearch({ strict: false }) as { invite?: string; share?: string }
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [password2, setPassword2] = useState('')
  const [displayName, setDisplayName] = useState('')
  const [inviteCode, setInviteCode] = useState(search.invite ?? '')
  const [cropFile, setCropFile] = useState<File | null>(null)
  const [cropOpen, setCropOpen] = useState(false)
  const [pendingAvatar, setPendingAvatar] = useState<Blob | null>(null)
  const resetRef = useRef<() => void>(null)
  const bootstrap = useBootstrap()
  const register = useRegister()
  const uploadAvatar = useUploadAvatar()

  useSyncDuringRender([search.invite], () => {
    if (search.invite) setInviteCode(search.invite)
  })

  useEffect(() => {
    if (bootstrap.data?.needs_bootstrap) {
      void nav({ to: '/onboarding' })
    }
  }, [bootstrap.data, nav])

  async function submit(): Promise<void> {
    const check = validatePasswordPair(password, password2)
    if (!check.ok) {
      notifications.show({ color: 'red', message: check.message })
      return
    }
    try {
      const res = await register.mutateAsync({
        email,
        password,
        display_name: displayName || undefined,
        invite_code: inviteCode.trim() || undefined,
        share_token: search.share,
      })
      if (pendingAvatar) {
        try {
          await uploadAvatar.mutateAsync(pendingAvatar)
        } catch (e) {
          notifications.show({ color: 'yellow', message: `Account created; avatar failed: ${e}` })
        }
      }
      if (!res.user.onboarding_completed) {
        void nav({ to: '/onboarding' })
        return
      }
      const { settings } = await api.getSettings()
      void nav({ to: resolveDefaultLanding(settings.default_landing) })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  if (bootstrap.isLoading) {
    return (
      <Center mih="70vh">
        <Loader color="violet" type="dots" />
      </Center>
    )
  }

  if (bootstrap.isError) {
    return (
      <Stack maw={420} mx="auto" mt={80} px="md">
        <QueryErrorAlert
          error={bootstrap.error}
          onRetry={() => void bootstrap.refetch()}
          title="Failed to check registration status"
        />
      </Stack>
    )
  }

  const closed = !bootstrap.data?.needs_bootstrap && bootstrap.data?.registration_open === false
  const hasInvite = inviteCode.trim().length > 0

  if (closed && !hasInvite) {
    return (
      <RegisterInviteGate
        inviteCode={inviteCode}
        onInviteCode={setInviteCode}
        onContinue={() => {
          void nav({ to: '/register', search: { invite: inviteCode.trim(), share: search.share } })
        }}
        onLogin={() => void nav({ to: '/login' })}
      />
    )
  }

  return (
    <>
      <RegisterForm
        closed={closed}
        hasInvite={hasInvite}
        hasShareInvite={!!search.share}
        inviteCode={inviteCode}
        displayName={displayName}
        email={email}
        password={password}
        password2={password2}
        pendingAvatar={pendingAvatar}
        registerPending={register.isPending}
        resetRef={resetRef}
        onInviteCode={setInviteCode}
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
        onSubmit={() => void submit()}
        onLogin={() => void nav({ to: '/login' })}
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
          notifications.show({ message: 'Avatar cropped — will upload on sign up' })
        }}
      />
    </>
  )
}
