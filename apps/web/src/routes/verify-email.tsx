import { createFileRoute } from '@tanstack/react-router'
import { VerifyEmailPage } from '@/pages'

// The token arrives in the fragment (`#token=…`), read by the page, never as a search param.
// Without a token the page is the hard gate's screen for an unverified account.
export const Route = createFileRoute('/verify-email')({
  component: VerifyEmailPage,
})
