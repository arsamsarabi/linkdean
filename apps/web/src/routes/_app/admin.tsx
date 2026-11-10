import { createFileRoute } from '@tanstack/react-router'
import { AdminPage } from '@/pages'

export const Route = createFileRoute('/_app/admin')({
  component: AdminPage,
})
