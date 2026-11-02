import { createFileRoute } from '@tanstack/react-router'
import { SharedPage } from '@/pages'

export const Route = createFileRoute('/_app/shared')({
  component: SharedPage,
})
