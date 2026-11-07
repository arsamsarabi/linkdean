import { type ReactElement, type ReactNode } from 'react'
import { Anchor, Checkbox } from '@mantine/core'

type Props = {
  checked: boolean
  onChange: (checked: boolean) => void
  /** Extra line under the label, e.g. why a returning user is asked too. */
  description?: ReactNode
}

/** Required sign-up checkbox. Links open in a new tab so a half-filled form survives. */
export function TermsAgreement({ checked, onChange, description }: Props): ReactElement {
  return (
    <Checkbox
      checked={checked}
      onChange={(e) => onChange(e.currentTarget.checked)}
      description={description}
      label={
        <>
          I agree to the{' '}
          <Anchor href="/terms" target="_blank" rel="noopener noreferrer" inherit>
            Terms of Service
          </Anchor>
          ,{' '}
          <Anchor href="/privacy" target="_blank" rel="noopener noreferrer" inherit>
            Privacy Policy
          </Anchor>{' '}
          and{' '}
          <Anchor href="/acceptable-use" target="_blank" rel="noopener noreferrer" inherit>
            Acceptable Use Policy
          </Anchor>
        </>
      }
    />
  )
}
