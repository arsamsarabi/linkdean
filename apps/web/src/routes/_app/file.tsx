import { createFileRoute } from '@tanstack/react-router'
import { FilingQueuePage } from '@/pages'

export const Route = createFileRoute('/_app/file')({
  component: FilingQueuePage,
})
