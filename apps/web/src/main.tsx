// Mantine's styles must load before any component CSS module, or its resets win ties.
import '@mantine/core/styles.css'
import '@mantine/notifications/styles.css'
import '@/styles/fonts.css'
import '@/styles/styles.css'
import React from 'react'
import { MantineProvider } from '@mantine/core'
import { Notifications } from '@mantine/notifications'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createRouter, RouterProvider } from '@tanstack/react-router'
import ReactDOM from 'react-dom/client'
import { registerSW } from 'virtual:pwa-register'
import { dropLegacyToken } from '@/lib'
import { routeTree } from '@/routeTree.gen'
import { linkdeanCssVariables, linkdeanTheme } from '@/styles'

// Older versions kept the session token in localStorage; the cookie is enough now.
dropLegacyToken()

const queryClient = new QueryClient()
const router = createRouter({ routeTree, context: { queryClient } })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

registerSW({ immediate: true })

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <MantineProvider theme={linkdeanTheme} cssVariablesResolver={linkdeanCssVariables} defaultColorScheme="auto">
        <Notifications />
        <RouterProvider router={router} />
      </MantineProvider>
    </QueryClientProvider>
  </React.StrictMode>
)
