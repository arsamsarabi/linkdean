import { type ReactElement } from 'react'
import { List, Text } from '@mantine/core'
import { LegalLayout, LegalSection } from './LegalLayout'

export function PrivacyPage(): ReactElement {
  return (
    <LegalLayout title="Privacy Policy">
      <LegalSection heading="1. What we store">
        <Text>When you use Linkdean we store:</Text>
        <List>
          <List.Item>
            Your account: email address, display name, a hash of your password (never the password itself), and the
            Terms version you accepted with the time you accepted it.
          </List.Item>
          <List.Item>
            Your content: bookmarks, notes, tags, lists, dashboards, uploaded images and your avatar, plus page titles,
            icons and preview images fetched for the links you save.
          </List.Item>
          <List.Item>
            Your settings, sessions, API keys and activity such as shares, comments and ratings you make.
          </List.Item>
          <List.Item>
            Vault items are encrypted with a key derived from your vault passphrase, which we never store.
          </List.Item>
        </List>
      </LegalSection>
      <LegalSection heading="2. Cookies">
        <Text>We use only strictly necessary cookies, so there is no cookie consent banner:</Text>
        <List>
          <List.Item>
            <code>linkdean_session</code>: keeps you signed in. HttpOnly, and removed when you sign out or it expires.
          </List.Item>
          <List.Item>
            <code>linkdean_oidc</code> and <code>linkdean_oidc_link</code>: short-lived, set only while you sign in with
            single sign-on.
          </List.Item>
        </List>
        <Text>
          We do not use analytics, advertising or tracking cookies. The beta notice remembers that you dismissed it in
          your browser&apos;s session storage, which never leaves your device.
        </Text>
      </LegalSection>
      <LegalSection heading="3. Why we use it">
        <Text>
          To run the service you asked for: sign you in, store and search your bookmarks, send the emails you trigger
          (verification, password reset, invites) and keep the service secure, for example with rate limits that count
          requests by IP address in memory.
        </Text>
      </LegalSection>
      <LegalSection heading="4. Who else handles it">
        <List>
          <List.Item>
            Email delivery: the SMTP provider configured for this instance receives the address and message of each
            email we send.
          </List.Item>
          <List.Item>
            Cloudflare Turnstile: if the instance has it turned on, the sign-up form loads it and Cloudflare sees your
            browser and IP address to tell people from bots.
          </List.Item>
          <List.Item>
            Single sign-on: if you sign in through an identity provider, that provider tells us your email and name.
          </List.Item>
          <List.Item>
            Search: bookmark search runs on a Meilisearch server operated with the instance; it is not a third party.
            Vault content is never indexed.
          </List.Item>
        </List>
        <Text>We do not sell your data.</Text>
      </LegalSection>
      <LegalSection heading="5. Public and shared content">
        <Text>
          Lists you make public, and what you share with other people, can be seen by them. Your email address is not
          shown to people who were not given it. To flag something that should not be there, use Report on public lists,
          profiles and comments, or email the operator&apos;s abuse address when it is published in the report form. The
          report, your account and your reason are visible to the instance administrators.
        </Text>
      </LegalSection>
      <LegalSection heading="6. Your choices">
        <Text>
          You can export your bookmarks, change your profile and email, and delete your account from your settings.
          Deleting removes your account and content after a short grace period. For anything else, ask the administrator
          of this instance.
        </Text>
      </LegalSection>
      <LegalSection heading="7. Changes">
        <Text>If this policy changes, the version date above changes with it.</Text>
      </LegalSection>
    </LegalLayout>
  )
}
