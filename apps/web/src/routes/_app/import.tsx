import { createFileRoute, redirect } from '@tanstack/react-router'

export const Route = createFileRoute('/_app/import')({
  beforeLoad: () => {
    throw redirect({ to: '/settings', search: { tab: 'import' } })
  },
})
