import { createFileRoute, redirect } from '@tanstack/react-router'

/** The Acceptable Use Policy is now section 5 of the Terms; keep old links working. */
export const Route = createFileRoute('/acceptable-use')({
  beforeLoad: () => {
    throw redirect({ to: '/terms', hash: 'acceptable-use', replace: true })
  },
})
