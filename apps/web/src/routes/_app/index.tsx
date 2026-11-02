import { createFileRoute } from '@tanstack/react-router'
import { DashboardPage } from '@/pages'

type DashboardSearch = {
  /** Open this dashboard instead of the pinned one (links from the Shared page). */
  board?: string
}

export const Route = createFileRoute('/_app/')({
  validateSearch: (search: Record<string, unknown>): DashboardSearch => ({
    board: typeof search.board === 'string' && search.board ? search.board : undefined,
  }),
  component: DashboardPage,
})
