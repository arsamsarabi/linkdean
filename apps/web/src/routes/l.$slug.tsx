import { createFileRoute } from '@tanstack/react-router'
import { PublicListPage } from '@/pages/public-list'

export const Route = createFileRoute('/l/$slug')({
  component: PublicListPage,
})
