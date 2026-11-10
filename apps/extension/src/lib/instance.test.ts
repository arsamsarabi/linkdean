import { describe, expect, it } from 'vitest'
import { normalizeInstance, originPattern } from './instance'

describe('instance address', () => {
  it('turns what people type into an origin', () => {
    expect(normalizeInstance('app.linkdean.app')).toBe('https://app.linkdean.app')
    expect(normalizeInstance(' https://links.example.com/library?x=1 ')).toBe('https://links.example.com')
    expect(normalizeInstance('http://localhost:5173')).toBe('http://localhost:5173')
  })

  it('refuses plain http off this machine, credentials and nonsense', () => {
    expect(() => normalizeInstance('http://links.example.com')).toThrow(/https/)
    expect(() => normalizeInstance('https://u:p@links.example.com')).toThrow(/password/)
    expect(() => normalizeInstance('')).toThrow()
    expect(() => normalizeInstance('ftp://links.example.com')).toThrow(/https/)
  })

  it('asks for the host without a port', () => {
    expect(originPattern('http://localhost:5173')).toBe('http://localhost/*')
    expect(originPattern('https://app.linkdean.app')).toBe('https://app.linkdean.app/*')
  })
})
