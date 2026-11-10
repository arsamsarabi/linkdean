import { createFileRoute } from '@tanstack/react-router'
import { ConnectExtensionPage, type ConnectExtensionSearch } from '@/pages'

function text(v: unknown): string | undefined {
  return typeof v === 'string' && v.trim() ? v.trim() : undefined
}

export const Route = createFileRoute('/_app/connect/extension')({
  validateSearch: (search: Record<string, unknown>): ConnectExtensionSearch => ({
    extension_origin: text(search.extension_origin),
    code_challenge: text(search.code_challenge),
    code_challenge_method: text(search.code_challenge_method),
    state: text(search.state),
  }),
  component: ConnectExtensionPage,
})
