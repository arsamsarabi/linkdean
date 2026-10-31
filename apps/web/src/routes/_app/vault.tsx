import { createFileRoute, redirect } from '@tanstack/react-router'

export const Route = createFileRoute('/_app/vault')({
  beforeLoad: () => {
    throw redirect({ to: '/settings', search: { tab: 'vault' } })
  },
})
