import { createFileRoute } from '@tanstack/react-router'
import { BannedPage } from '@/pages'

export const Route = createFileRoute('/banned')({
  component: BannedPage,
})
