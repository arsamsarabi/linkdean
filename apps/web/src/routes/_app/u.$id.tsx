import { createFileRoute } from '@tanstack/react-router'
import { UserProfilePage } from '@/pages'

export const Route = createFileRoute('/_app/u/$id')({
  component: UserProfilePage,
})
