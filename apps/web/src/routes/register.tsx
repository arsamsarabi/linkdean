import { createFileRoute } from '@tanstack/react-router'
import { RegisterPage } from '@/pages'

export const Route = createFileRoute('/register')({
  component: RegisterPage,
})
