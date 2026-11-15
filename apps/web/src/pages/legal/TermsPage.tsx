import { type ReactElement } from 'react'
import { Anchor, List, Text } from '@mantine/core'
import { LegalLayout, LegalSection, LegalSubsection } from './LegalLayout'

export function TermsPage(): ReactElement {
  return (
    <LegalLayout title="Terms of Service">
      <LegalSection heading="1. The service">
        <Text>
          Linkdean is a bookmark manager. These terms apply to the Linkdean instance you are signing up on, run by its
          operator (&ldquo;we&rdquo;). By creating an account you agree to them, including the{' '}
          <Anchor href="#acceptable-use">acceptable use rules</Anchor> in section 5, and to the{' '}
          <Anchor href="/privacy">Privacy Policy</Anchor>.
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
      <LegalSection heading="5. Acceptable use" id="acceptable-use">
        <LegalSubsection heading="5.1 Be decent">
          <Text>Do not use Linkdean to save, share or publish:</Text>
          <List>
            <List.Item>
              Content that is illegal where the instance operates, or that sexually exploits or endangers children.
            </List.Item>
            <List.Item>Harassment, threats, hate, or doxxing of other people.</List.Item>
            <List.Item>Malware, phishing pages or links meant to defraud or compromise others.</List.Item>
            <List.Item>
              Material you have no right to share, including copyrighted work you are not allowed to republish.
            </List.Item>
          </List>
        </LegalSubsection>
        <LegalSubsection heading="5.2 Protect the service">
          <List>
            <List.Item>No spam: mass invites, comments or shares that nobody asked for.</List.Item>
            <List.Item>
              No probing for weaknesses, scraping other users&apos; data, or getting around rate limits and quotas.
            </List.Item>
            <List.Item>No automated sign-ups. Use API keys for scripts and keep within the limits.</List.Item>
            <List.Item>Do not save links to private or internal addresses to make the server fetch them.</List.Item>
          </List>
        </LegalSubsection>
        <LegalSubsection heading="5.3 Enforcement">
          <Text>
            We may remove content, suspend or ban accounts that break these rules, close accounts that put the service
            or other users at risk, and report illegal activity. Where we can, we say why. If you think we got it wrong,
            contact the administrator of this instance.
          </Text>
        </LegalSubsection>
        <LegalSubsection heading="5.4 Reporting">
          <Text>
            To report abuse, use Report on public lists, profiles and comments (you need to be signed in), or contact
            the administrator of this instance at the abuse address shown in the report form once one is published.
          </Text>
        </LegalSubsection>
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
