import { createFileRoute } from '@tanstack/react-router'
import { TrashPage } from '@/pages'

export const Route = createFileRoute('/_app/trash')({
  component: TrashPage,
})
