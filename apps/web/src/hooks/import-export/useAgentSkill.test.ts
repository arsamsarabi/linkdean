import { describe, expect, it } from 'vitest'
import { api } from '@/api'
import { downloadName } from './useAgentSkill'

describe('agent skill download', () => {
  it('uses the file name the server picked', () => {
    expect(downloadName('attachment; filename="rust-web.zip"', 'x.zip')).toBe('rust-web.zip')
    expect(downloadName(null, 'x.zip')).toBe('x.zip')
  })

  it('asks for a folder or a tag', () => {
    expect(api.agentSkillUrl({ folder: 'f1' })).toBe('/api/agent-skill?folder=f1&format=zip')
    expect(api.agentSkillUrl({ tag: 'c++ & rust' }, 'md')).toBe('/api/agent-skill?tag=c%2B%2B+%26+rust&format=md')
  })
})
