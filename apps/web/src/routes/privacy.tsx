import { createFileRoute } from '@tanstack/react-router'
import { PrivacyPage } from '@/pages'

export const Route = createFileRoute('/privacy')({ component: PrivacyPage })
