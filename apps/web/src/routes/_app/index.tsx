import { createFileRoute } from '@tanstack/react-router'
import { DashboardPage } from '@/pages'

export const Route = createFileRoute('/_app/')({
  component: DashboardPage,
})
