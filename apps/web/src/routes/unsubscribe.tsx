import { createFileRoute } from '@tanstack/react-router'
import { UnsubscribePage } from '@/pages'

// Signed out is fine: the link (`u`, `s`, `t`) carries its own signature. The page reads it.
export const Route = createFileRoute('/unsubscribe')({
  component: UnsubscribePage,
})
