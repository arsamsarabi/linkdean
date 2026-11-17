import { type ReactElement, type ReactNode } from 'react'
import { Anchor, Checkbox } from '@mantine/core'
import type { LegalLinks } from '@/schemas'

type Props = {
  legal: LegalLinks
  checked: boolean
  onChange: (checked: boolean) => void
  /** Extra line under the label, e.g. why a returning user is asked too. */
  description?: ReactNode
}

/** Required sign-up checkbox, on a server with terms. Links open in a new tab so a half-filled form survives. */
export function TermsAgreement({ legal, checked, onChange, description }: Props): ReactElement | null {
  if (!legal.terms_version || !legal.terms_url) return null
  return (
    <Checkbox
      checked={checked}
      onChange={(e) => onChange(e.currentTarget.checked)}
      description={description}
      label={
        <>
          I agree to the{' '}
          <Anchor href={legal.terms_url} target="_blank" rel="noopener noreferrer" inherit>
            Terms of Service
          </Anchor>
          {legal.privacy_url ? (
            <>
              {' '}
              and{' '}
              <Anchor href={legal.privacy_url} target="_blank" rel="noopener noreferrer" inherit>
                Privacy Policy
              </Anchor>
            </>
          ) : null}
        </>
      }
    />
  )
}
