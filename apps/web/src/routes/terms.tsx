import { createFileRoute } from '@tanstack/react-router'
import { TermsPage } from '@/pages'

export const Route = createFileRoute('/terms')({ component: TermsPage })
