import { describe, expect, it } from 'vitest'
import { inviteeShareAccess } from './shareAccess'

describe('inviteeShareAccess', () => {
  it('null for owner or missing ids', () => {
    expect(inviteeShareAccess('a', true, 'a')).toBeNull()
    expect(inviteeShareAccess('a', false, undefined)).toBeNull()
    expect(inviteeShareAccess(undefined, true, 'b')).toBeNull()
  })

  it('RO vs RW for invitee', () => {
    expect(inviteeShareAccess('owner', false, 'invitee')).toBe('read')
    expect(inviteeShareAccess('owner', true, 'invitee')).toBe('write')
  })
})
