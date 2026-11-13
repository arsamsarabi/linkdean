import { useState } from 'react'
import { notifications } from '@mantine/notifications'
import { api, type AgentSkillSource } from '@/api'

/** The file name the server picked (`rust.zip`), else a safe default. */
export function downloadName(disposition: string | null, fallback: string): string {
  const match = disposition?.match(/filename="([^"]+)"/)
  return match?.[1] ?? fallback
}

/** Downloads a folder or tag as an Agent Skill zip, with a notification on failure. */
export function useDownloadAgentSkill(): {
  download: (source: AgentSkillSource) => Promise<void>
  pending: boolean
} {
  const [pending, setPending] = useState(false)

  async function download(source: AgentSkillSource): Promise<void> {
    setPending(true)
    try {
      const res = await fetch(api.agentSkillUrl(source), { credentials: 'include' })
      if (!res.ok) {
        const body = (await res.json().catch(() => null)) as { error?: string } | null
        throw new Error(
          res.status === 404
            ? 'No bookmarks found there to make a skill from'
            : body?.error || 'Could not make the skill'
        )
      }
      const blob = await res.blob()
      const a = document.createElement('a')
      const objectUrl = URL.createObjectURL(blob)
      a.href = objectUrl
      a.download = downloadName(res.headers.get('content-disposition'), 'linkdean-skill.zip')
      a.click()
      URL.revokeObjectURL(objectUrl)
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    } finally {
      setPending(false)
    }
  }

  return { download, pending }
}
