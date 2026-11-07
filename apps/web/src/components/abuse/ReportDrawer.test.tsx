import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest'
import { ReportDrawer } from './ReportDrawer'

const state = vi.hoisted(() => ({ create: vi.fn(), contact: null as string | null }))

vi.mock('@/hooks', () => ({
  useCreateReport: () => ({ mutateAsync: state.create, isPending: false }),
  useReportConfig: () => ({ data: { abuse_contact_email: state.contact } }),
}))
vi.mock('@/hooks/render', () => ({
  useDrawerClosesOnEscape: () => true,
  useDrawerSize: (desktop: string | number = 'md') => desktop,
}))
vi.mock('@mantine/notifications', () => ({ notifications: { show: vi.fn() } }))

beforeAll(() => {
  // The details field autosizes, and listens for web fonts; jsdom has neither.
  Object.defineProperty(document, 'fonts', {
    configurable: true,
    value: { addEventListener: () => undefined, removeEventListener: () => undefined },
  })
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe(): void {}
      unobserve(): void {}
      disconnect(): void {}
    }
  )
})

function renderDrawer(): () => void {
  const onClose = vi.fn()
  render(
    <MantineProvider env="test">
      <ReportDrawer opened onClose={onClose} targetType="comment" targetId="c1" subject="comment" />
    </MantineProvider>
  )
  return onClose
}

describe('ReportDrawer', () => {
  afterEach(() => {
    cleanup()
    state.create.mockReset()
    state.contact = null
  })

  it('needs a reason before it can send', async () => {
    renderDrawer()
    const send = (await screen.findByRole('button', { name: 'Send report' })) as HTMLButtonElement
    expect(send.disabled).toBe(true)
  })

  it('sends the reason and trimmed details, then closes', async () => {
    state.create.mockResolvedValue({ ok: true, id: 'r1' })
    const onClose = renderDrawer()
    await userEvent.click(await screen.findByRole('radio', { name: 'Spam or scam' }))
    await userEvent.type(screen.getByRole('textbox', { name: /Details/ }), '  buy now  ')
    await userEvent.click(screen.getByRole('button', { name: 'Send report' }))
    expect(state.create).toHaveBeenCalledWith({
      target_type: 'comment',
      target_id: 'c1',
      reason: 'spam',
      details: 'buy now',
    })
    expect(onClose).toHaveBeenCalled()
  })

  it('shows the server error and stays open', async () => {
    state.create.mockRejectedValue(new Error('too many requests'))
    const onClose = renderDrawer()
    await userEvent.click(await screen.findByRole('radio', { name: 'Impersonation' }))
    await userEvent.click(screen.getByRole('button', { name: 'Send report' }))
    expect((await screen.findByRole('alert')).textContent).toContain('too many requests')
    expect(onClose).not.toHaveBeenCalled()
  })

  it('offers the operator contact when one is set', async () => {
    state.contact = 'abuse@example.com'
    renderDrawer()
    const link = await screen.findByRole('link', { name: 'abuse@example.com' })
    expect(link.getAttribute('href')).toBe('mailto:abuse@example.com')
  })
})
