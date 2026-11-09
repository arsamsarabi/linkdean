import { createFileRoute } from '@tanstack/react-router'
import { SavePage } from '@/pages'
import type { SharedPage } from '@/services'

function text(v: unknown): string | undefined {
  return typeof v === 'string' && v.trim() ? v.trim() : undefined
}

export const Route = createFileRoute('/_app/save')({
  validateSearch: (search: Record<string, unknown>): SharedPage => ({
    url: text(search.url),
    title: text(search.title),
    text: text(search.text),
  }),
  component: SavePage,
})
