import { describe, expect, it } from 'vitest'
import { folderOptions, savableUrl, type Category } from './api'

const cat = (id: string, name: string, extra: Partial<Category> = {}): Category => ({
  id,
  name,
  parent_id: null,
  is_vault: false,
  access: 'owner',
  ...extra,
})

describe('folder choices', () => {
  it('shows full paths, sorted, without vaults or read-only folders', () => {
    const options = folderOptions([
      cat('b', 'Reading'),
      cat('a', 'Rust', { parent_id: 'c' }),
      cat('c', 'Dev'),
      cat('v', 'Secret', { is_vault: true }),
      cat('r', 'Shared with me', { access: 'read' }),
      cat('w', 'Team', { access: 'write' }),
    ])
    expect(options.map((o) => o.label)).toEqual(['Dev', 'Dev / Rust', 'Reading', 'Team'])
  })

  it('saves only web pages', () => {
    expect(savableUrl('https://example.com')).toBe(true)
    expect(savableUrl('chrome://extensions')).toBe(false)
    expect(savableUrl(undefined)).toBe(false)
  })
})
