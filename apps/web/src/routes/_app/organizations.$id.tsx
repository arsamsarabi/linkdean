import { createFileRoute } from '@tanstack/react-router'
import { OrganizationSettingsPage } from '@/pages/organizations/OrganizationSettingsPage'

export const Route = createFileRoute('/_app/organizations/$id')({
  component: OrganizationSettingsPage,
})
