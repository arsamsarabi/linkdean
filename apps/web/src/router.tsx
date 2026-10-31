import {
  Outlet,
  createRootRoute,
  createRoute,
  redirect,
} from '@tanstack/react-router'
import { AppShell } from './pages/app-shell'
import { LoginPage } from './pages/login'
import { LibraryPage } from './pages/library'
import { OnboardingPage } from './pages/onboarding'
import { ProfilePage } from './pages/profile'
import { SettingsPage } from './pages/settings'
import { RegisterPage } from './pages/register'
import { getToken } from './lib/auth/authToken'
import { validateLibrarySearch } from './lib/library/librarySearch'

const rootRoute = createRootRoute({
  component: () => <Outlet />,
})

const loginRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/login',
  component: LoginPage,
})

const registerRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/register',
  component: RegisterPage,
})

const onboardingRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/onboarding',
  component: OnboardingPage,
})

/** Pathless auth layout — avoids duplicate `/` with index child */
const appRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: '_app',
  component: AppShell,
  beforeLoad: () => {
    if (!getToken()) throw redirect({ to: '/login' })
  },
})

const libraryIndexRoute = createRoute({
  getParentRoute: () => appRoute,
  path: '/',
  beforeLoad: () => {
    throw redirect({ to: '/library' })
  },
})

const libraryRoute = createRoute({
  getParentRoute: () => appRoute,
  path: '/library',
  component: LibraryPage,
  validateSearch: (search: Record<string, unknown>) => validateLibrarySearch(search),
})

const importRoute = createRoute({
  getParentRoute: () => appRoute,
  path: '/import',
  beforeLoad: () => {
    throw redirect({ to: '/settings', search: { tab: 'import' } })
  },
})

const vaultRoute = createRoute({
  getParentRoute: () => appRoute,
  path: '/vault',
  beforeLoad: () => {
    throw redirect({ to: '/settings', search: { tab: 'vault' } })
  },
})

const profileRoute = createRoute({
  getParentRoute: () => appRoute,
  path: '/profile',
  component: ProfilePage,
})

const settingsRoute = createRoute({
  getParentRoute: () => appRoute,
  path: '/settings',
  component: SettingsPage,
  validateSearch: (search: Record<string, unknown>): { tab?: string } => ({
    tab: typeof search.tab === 'string' ? search.tab : undefined,
  }),
})

export const routeTree = rootRoute.addChildren([
  loginRoute,
  registerRoute,
  onboardingRoute,
  appRoute.addChildren([
    libraryIndexRoute,
    libraryRoute,
    importRoute,
    vaultRoute,
    profileRoute,
    settingsRoute,
  ]),
])
