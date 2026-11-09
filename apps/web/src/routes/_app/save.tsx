import { createFileRoute } from '@tanstack/react-router'
import { SavePage, type SaveSearch } from '@/pages'

function text(v: unknown): string | undefined {
  return typeof v === 'string' && v.trim() ? v.trim() : undefined
}

export const Route = createFileRoute('/_app/save')({
  validateSearch: (search: Record<string, unknown>): SaveSearch => ({
    url: text(search.url),
    title: text(search.title),
    text: text(search.text),
    popup: search.popup === 1 || search.popup === '1' || search.popup === true ? true : undefined,
  }),
  component: SavePage,
})
