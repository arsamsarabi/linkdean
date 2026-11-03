import { createFileRoute } from '@tanstack/react-router'
import { ResetPasswordPage } from '@/pages'

// The token arrives in the fragment (`#token=…`), read by the page, never as a search param.
export const Route = createFileRoute('/reset-password')({
  component: ResetPasswordPage,
})
