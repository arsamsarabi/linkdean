import { createFileRoute } from '@tanstack/react-router'
import { BrokenLinksPage } from '@/pages'

export const Route = createFileRoute('/_app/broken-links')({
  component: BrokenLinksPage,
})
