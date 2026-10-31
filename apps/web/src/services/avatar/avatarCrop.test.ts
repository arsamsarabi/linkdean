import { afterEach, describe, expect, it, vi } from 'vitest'
import { getCroppedBlob } from './avatarCrop'

describe('getCroppedBlob', () => {
  afterEach(() => {
    vi.restoreAllMocks()
    vi.unstubAllGlobals()
  })

  it('draws crop into jpeg blob', async () => {
    const drawImage = vi.fn()
    const toBlob = vi.fn((cb: BlobCallback) => {
      cb(new Blob(['x'], { type: 'image/jpeg' }))
    })
    vi.spyOn(document, 'createElement').mockImplementation(((tag: string) => {
      if (tag === 'canvas') {
        return {
          width: 0,
          height: 0,
          getContext: () => ({ drawImage }),
          toBlob,
        } as unknown as HTMLCanvasElement
      }
      return document.createElement(tag)
    }) as typeof document.createElement)

    vi.stubGlobal(
      'Image',
      class {
        private listeners: Record<string, Array<() => void>> = {}
        addEventListener(type: string, cb: () => void): void {
          ;(this.listeners[type] ||= []).push(cb)
        }
        set src(_v: string) {
          queueMicrotask(() => {
            for (const cb of this.listeners.load || []) cb()
          })
        }
      }
    )

    const blob = await getCroppedBlob('blob:test', { x: 0, y: 0, width: 50, height: 50 })
    expect(blob).toBeInstanceOf(Blob)
    expect(drawImage).toHaveBeenCalled()
    expect(toBlob).toHaveBeenCalled()
  })
})
