import { createFileRoute, redirect } from '@tanstack/react-router'
import { getToken } from '@/lib'
import { AppShell } from '@/pages'

export const Route = createFileRoute('/_app')({
  beforeLoad: () => {
    if (!getToken()) throw redirect({ to: '/login' })
  },
  component: AppShell,
})
