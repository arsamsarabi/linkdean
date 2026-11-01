import { createFileRoute } from '@tanstack/react-router'
import { RegisterPage } from '@/pages'

export type RegisterSearch = {
  invite?: string
  share?: string
}

export const Route = createFileRoute('/register')({
  validateSearch: (search: Record<string, unknown>): RegisterSearch => ({
    invite: typeof search.invite === 'string' && search.invite.trim() ? search.invite.trim() : undefined,
    share: typeof search.share === 'string' && search.share.trim() ? search.share.trim() : undefined,
  }),
  component: RegisterPage,
})
