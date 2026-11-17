import { createFileRoute } from '@tanstack/react-router'
import { ArchivePage } from '@/pages'

export const Route = createFileRoute('/_app/archive/$id')({
  validateSearch: (search: Record<string, unknown>): { format?: string } => ({
    format: typeof search.format === 'string' ? search.format : undefined,
  }),
  component: ArchivePage,
})
