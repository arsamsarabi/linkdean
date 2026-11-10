import type { QueryClient } from '@tanstack/react-query'
import { createRootRouteWithContext, Outlet } from '@tanstack/react-router'
import { AppScroll, BetaBanner } from '@/components'
import { NotFoundPage } from '@/pages'

export type RouterContext = { queryClient: QueryClient }

export const Route = createRootRouteWithContext<RouterContext>()({
  component: () => (
    <AppScroll>
      <BetaBanner />
      <Outlet />
    </AppScroll>
  ),
  notFoundComponent: NotFoundPage,
})
