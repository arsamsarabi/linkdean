import { createFileRoute } from '@tanstack/react-router'
import { DuplicatesPage } from '@/pages'

export const Route = createFileRoute('/_app/duplicates')({
  component: DuplicatesPage,
})
