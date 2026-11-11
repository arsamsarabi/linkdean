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
      <LegalSection heading="2a. Page view statistics">
        <Text>
          An instance can count page views with GoatCounter, without cookies and without storing anything in your
          browser. linkdean.app does, on its own GoatCounter at stats.linkdean.app, for the app, this site and the docs.
          Each page you open sends:
        </Text>
        <List>
          <List.Item>
            In the app, the name of the page only, such as &quot;/library&quot; or &quot;/bookmarks/$id&quot;: never
            bookmark ids, folder names, search terms or titles. On the marketing site and docs, the page&apos;s address
            and the site you came from.
          </List.Item>
          <List.Item>Your screen size.</List.Item>
          <List.Item>
            Your browser&apos;s user agent and IP address, which GoatCounter uses to work out the browser, operating
            system and country and to tell repeat visits apart with a hash that changes every day. It does not store the
            IP address.
          </List.Item>
        </List>
        <Text>
          Nothing is sent if your browser sends Do Not Track or Global Privacy Control. The statistics are counts only
          and are not linked to your account.
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
      <LegalSection heading="6. Browser extension">
        <Text>
          The Linkdean extension for Chrome and Firefox works only with the Linkdean you sign it in to, such as
          app.linkdean.app or your own instance. It sends that instance the address, title and the text you selected of
          pages you choose to save, when you use its button, menu or shortcut, and reads your folders and tags to offer
          them. It does not send the pages you visit otherwise, and it sends nothing to anyone else.
        </Text>
        <Text>
          If you turn on &quot;Color the icon on saved pages&quot;, the extension reads the address of each tab and
          checks it in your browser against short hashes of the addresses you saved, which it downloads from your
          instance. The addresses of the pages you visit are not sent.
        </Text>
        <Text>
          Signing in creates an API key for the extension, kept in the extension&apos;s own storage in your browser.
          Signing out of the extension revokes it, and you can revoke it under Settings → API keys.
        </Text>
      </LegalSection>
      <LegalSection heading="7. Your choices">
        <Text>
          You can export your bookmarks, change your profile and email, and delete your account from your settings.
          Deleting removes your account and content after a short grace period. For anything else, ask the administrator
          of this instance.
        </Text>
      </LegalSection>
      <LegalSection heading="8. Changes">
        <Text>If this policy changes, the version date above changes with it.</Text>
      </LegalSection>
    </LegalLayout>
  )
}
