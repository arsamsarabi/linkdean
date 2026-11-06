import type { QueryClient } from '@tanstack/react-query'
import { createRootRouteWithContext, Outlet } from '@tanstack/react-router'
import { BetaBanner } from '@/components'

export type RouterContext = { queryClient: QueryClient }

export const Route = createRootRouteWithContext<RouterContext>()({
  component: () => (
    <>
      <BetaBanner />
      <Outlet />
    </>
  ),
})
