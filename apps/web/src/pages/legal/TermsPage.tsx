import { type ReactElement } from 'react'
import { Anchor, Text } from '@mantine/core'
import { LegalLayout, LegalSection } from './LegalLayout'

export function TermsPage(): ReactElement {
  return (
    <LegalLayout title="Terms of Service">
      <LegalSection heading="1. The service">
        <Text>
          Linkdean is a bookmark manager. These terms apply to the Linkdean instance you are signing up on, run by its
          operator (&ldquo;we&rdquo;). By creating an account you agree to them, to the{' '}
          <Anchor href="/privacy">Privacy Policy</Anchor> and to the{' '}
          <Anchor href="/acceptable-use">Acceptable Use Policy</Anchor>.
        </Text>
      </LegalSection>
      <LegalSection heading="2. Public beta">
        <Text>
          Linkdean is in public beta. It may change, break or go offline without notice, and we make no promise of
          uptime. Export your bookmarks regularly from the import and export settings; a backup you made yourself is the
          only one you can count on.
        </Text>
      </LegalSection>
      <LegalSection heading="3. Your account">
        <Text>
          Give a real email address and keep your password and any API keys secret. You are responsible for what happens
          under your account. Tell us if you think someone else has access to it.
        </Text>
      </LegalSection>
      <LegalSection heading="4. Your content">
        <Text>
          The bookmarks, notes, lists and images you save stay yours. You give us only the permission needed to store
          them, show them to you, and show them to people you choose to share them with or to the public when you make a
          list public. You confirm you have the right to save and share what you add.
        </Text>
      </LegalSection>
      <LegalSection heading="5. What you may not do">
        <Text>
          Follow the <Anchor href="/acceptable-use">Acceptable Use Policy</Anchor>. We may remove content, suspend or
          ban accounts that break it, or close accounts that put the service or other users at risk.
        </Text>
      </LegalSection>
      <LegalSection heading="6. Ending your account">
        <Text>
          You can delete your account from your settings; it is removed after a short grace period during which you can
          cancel. We may end or suspend an account for breaking these terms.
        </Text>
      </LegalSection>
      <LegalSection heading="7. No warranty and limited liability">
        <Text>
          The service is provided &ldquo;as is&rdquo; during the beta, without warranties of any kind. To the extent the
          law allows, we are not liable for lost data, lost profit or indirect damages arising from your use of it.
        </Text>
      </LegalSection>
      <LegalSection heading="8. Changes">
        <Text>
          We may update these terms. The version date above changes when we do. Continuing to use the service after a
          change means you accept the new version.
        </Text>
      </LegalSection>
      <LegalSection heading="9. Contact">
        <Text>
          Questions go to the administrator of this instance. To report abuse, use Report on public lists, profiles and
          comments, or email the operator&apos;s abuse address when it is published in the report form.
        </Text>
      </LegalSection>
    </LegalLayout>
  )
}
