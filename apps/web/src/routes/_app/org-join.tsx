import { createFileRoute } from '@tanstack/react-router'
import { OrganizationJoinPage } from '@/pages/organizations/OrganizationJoinPage'

export type OrgJoinSearch = {
  code?: string
}

export const Route = createFileRoute('/_app/org-join')({
  validateSearch: (search: Record<string, unknown>): OrgJoinSearch => ({
    code: typeof search.code === 'string' && search.code.trim() ? search.code.trim() : undefined,
  }),
  component: OrganizationJoinPage,
})
