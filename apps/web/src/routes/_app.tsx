import { createFileRoute, redirect } from '@tanstack/react-router'
import { api, ApiError } from '@/api'
import { isSignedIn, rememberReturnTo, takeReturnTo } from '@/lib'
import { AppShell } from '@/pages'

export const Route = createFileRoute('/_app')({
  beforeLoad: async ({ context, location }) => {
    if (!isSignedIn()) {
      // OIDC sign-in sets only the session cookie, so ask the API whether it holds a session.
      try {
        await context.queryClient.ensureQueryData({ queryKey: ['me'], queryFn: api.me })
      } catch (e) {
        if (e instanceof ApiError && e.status === 401) {
          rememberReturnTo(location.href)
          throw redirect({ to: '/login' })
        }
        // Bans and outages are handled by the shell and the API client.
      }
    }
    // Signed in, by password or single sign-on: finish the quick save that sent them to sign in.
    const back = takeReturnTo()
    if (back && back !== location.href) throw redirect({ href: back })
  },
  component: AppShell,
})
