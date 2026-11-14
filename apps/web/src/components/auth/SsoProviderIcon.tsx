import { type ReactElement } from 'react'
import { IconBrandGithub, IconBrandGoogle, IconBrandWindows, IconLogin } from '@tabler/icons-react'

const iconSm = { size: 16, stroke: 1.5 } as const

/** The icon for a sign-in provider's button; generic OIDC gets a plain sign-in arrow. */
export function SsoProviderIcon({ id }: { id: string }): ReactElement {
  switch (id) {
    case 'google':
      return <IconBrandGoogle {...iconSm} />
    case 'microsoft':
      return <IconBrandWindows {...iconSm} />
    case 'github':
      return <IconBrandGithub {...iconSm} />
    default:
      return <IconLogin {...iconSm} />
  }
}
