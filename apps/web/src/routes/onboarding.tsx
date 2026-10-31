import { createFileRoute } from '@tanstack/react-router'
import { OnboardingPage } from '@/pages'

export const Route = createFileRoute('/onboarding')({
  component: OnboardingPage,
})
