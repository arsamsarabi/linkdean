import '@mantine/core/styles.css'
import { StrictMode } from 'react'
import { createTheme, MantineProvider } from '@mantine/core'
import { createRoot } from 'react-dom/client'
import { App } from './App'

const theme = createTheme({ primaryColor: 'violet' })

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <MantineProvider theme={theme} defaultColorScheme="auto">
      <App />
    </MantineProvider>
  </StrictMode>
)
