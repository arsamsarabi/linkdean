import type { ReactElement } from 'react'
import { IconHome } from '@tabler/icons-react'
import { useNavigate } from '@tanstack/react-router'
import { EmptyState, MASCOT } from '@/components'

const iconSm = { size: 16, stroke: 1.5 } as const

export function NotFoundPage(): ReactElement {
  const nav = useNavigate()
  return (
    <EmptyState
      illustration={{ src: MASCOT.thinking }}
      title="Page not found"
      description="That address is not in this library."
      action={{
        label: 'Go home',
        onClick: () => void nav({ to: '/' }),
        leftSection: <IconHome {...iconSm} />,
      }}
    />
  )
}
