import React from 'react'
import { MantineProvider } from '@mantine/core'
import { Notifications } from '@mantine/notifications'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createRouter, RouterProvider } from '@tanstack/react-router'
import ReactDOM from 'react-dom/client'
import { routeTree } from '@/routeTree.gen'
import { linkdeanTheme } from '@/styles'
import '@mantine/core/styles.css'
import '@mantine/notifications/styles.css'
import '@/styles/fonts.css'
import '@/styles/styles.css'

const queryClient = new QueryClient()
const router = createRouter({ routeTree })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <MantineProvider theme={linkdeanTheme} defaultColorScheme="auto">
        <Notifications />
        <RouterProvider router={router} />
      </MantineProvider>
    </QueryClientProvider>
  </React.StrictMode>
)
