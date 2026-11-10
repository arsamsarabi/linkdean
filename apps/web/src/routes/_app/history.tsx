import { createFileRoute } from '@tanstack/react-router'
import { HistoryPage } from '@/pages'

export const Route = createFileRoute('/_app/history')({
  component: HistoryPage,
})
