import { createFileRoute, redirect } from '@tanstack/react-router'
import { api, ApiError } from '@/api'
import { isSignedIn } from '@/lib'
import { AppShell } from '@/pages'

export const Route = createFileRoute('/_app')({
  beforeLoad: async ({ context }) => {
    if (isSignedIn()) return
    // OIDC sign-in sets only the session cookie, so ask the API whether it holds a session.
    try {
      await context.queryClient.ensureQueryData({ queryKey: ['me'], queryFn: api.me })
    } catch (e) {
      if (e instanceof ApiError && e.status === 401) throw redirect({ to: '/login' })
      // Bans and outages are handled by the shell and the API client.
    }
  },
  component: AppShell,
})
