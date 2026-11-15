import { type ReactElement } from 'react'
import { List, Text } from '@mantine/core'
import { LegalLayout, LegalSection, LegalSubsection, MailLink } from './LegalLayout'
import { OPERATOR } from './operator'

export function PrivacyPage(): ReactElement {
  return (
    <LegalLayout title="Privacy Policy" noticeSubject="This Privacy Policy has">
      <LegalSection heading="1. Who is responsible for your data">
        <Text>
          The operator of the Linkdean instance you use is responsible for the personal data processed through that
          instance.
        </Text>
        <Text>For this instance, the data controller is:</Text>
        <Text fw={600}>
          {OPERATOR.legalName} (sole trader)
          <br />
          Contact: <MailLink address={OPERATOR.privacyEmail} />
        </Text>
        <Text>
          In this Privacy Policy, &ldquo;we&rdquo;, &ldquo;us&rdquo; and &ldquo;our&rdquo; mean that operator.
        </Text>
      </LegalSection>

      <LegalSection heading="2. What personal data we store">
        <Text>Depending on how you use Linkdean, we may process the following information.</Text>
        <LegalSubsection heading="Your account">
          <Text>This may include:</Text>
          <List>
            <List.Item>Your email address.</List.Item>
            <List.Item>Your display name.</List.Item>
            <List.Item>
              A cryptographic hash of your password. We do not store your password in readable form.
            </List.Item>
            <List.Item>The version of the Terms you accepted and the time you accepted them.</List.Item>
          </List>
        </LegalSubsection>
        <LegalSubsection heading="Your content">
          <Text>This may include:</Text>
          <List>
            <List.Item>Bookmarks and links.</List.Item>
            <List.Item>Notes.</List.Item>
            <List.Item>Tags.</List.Item>
            <List.Item>Lists.</List.Item>
            <List.Item>Dashboards.</List.Item>
            <List.Item>Uploaded images.</List.Item>
            <List.Item>Your avatar.</List.Item>
            <List.Item>Page titles, icons and preview images fetched for links you save.</List.Item>
          </List>
        </LegalSubsection>
        <LegalSubsection heading="Your activity and settings">
          <Text>This may include:</Text>
          <List>
            <List.Item>Your account settings.</List.Item>
            <List.Item>Login sessions.</List.Item>
            <List.Item>API keys.</List.Item>
            <List.Item>Shares, comments and ratings you create.</List.Item>
            <List.Item>Other actions necessary to operate the service.</List.Item>
          </List>
        </LegalSubsection>
        <LegalSubsection heading="Vault items">
          <Text>Vault items are encrypted using a key derived from your vault passphrase.</Text>
          <Text>We do not store your vault passphrase.</Text>
          <Text>
            You should keep your vault passphrase safe. If it cannot be recovered through the service, we may be unable
            to recover access to encrypted vault content for you.
          </Text>
        </LegalSubsection>
      </LegalSection>

      <LegalSection heading="3. Cookies and similar storage">
        <Text>We use only cookies that are necessary for the operation of the service.</Text>
        <Text>These include:</Text>
        <List>
          <List.Item>
            <code>linkdean_session</code>, which keeps you signed in. It is HttpOnly and is removed when you sign out or
            when it expires. It lasts 30 days unless you sign out sooner.
          </List.Item>
          <List.Item>
            <code>linkdean_oidc</code> and <code>linkdean_oidc_link</code>, which are short-lived cookies used during
            single sign-on where applicable.
          </List.Item>
        </List>
        <Text>We do not use advertising cookies.</Text>
        <Text>We do not use analytics or tracking cookies.</Text>
        <Text>
          The app also keeps a few interface preferences in your browser&apos;s local and session storage, such as
          whether the sidebar is collapsed, the folder you last saved to, which dashboard sections you collapsed, and
          notices you dismissed. The marketing site remembers your light or dark theme the same way. This stays on your
          device and is not sent to us.
        </Text>
        <Text>
          Because strictly necessary cookies are used to provide the service, consent may not be required for those
          cookies under applicable law. Cookie rules vary by jurisdiction.
        </Text>
      </LegalSection>

      <LegalSection heading="4. Page view statistics">
        <Text>
          An instance may use GoatCounter to measure page views without using cookies or storing identifiers in your
          browser.
        </Text>
        <Text>
          For linkdean.app, we run GoatCounter ourselves at <strong>stats.linkdean.app</strong>, on our own server, so
          no outside analytics company receives this data.
        </Text>
        <Text>When you open a page, the information sent may include:</Text>
        <List>
          <List.Item>The page being viewed.</List.Item>
          <List.Item>
            On the marketing site and documentation only, the site or page you came from. The app does not send it.
          </List.Item>
          <List.Item>Your screen size.</List.Item>
          <List.Item>Your browser&apos;s user agent.</List.Item>
          <List.Item>
            Your IP address for the purpose of processing the request and generating privacy-preserving statistics.
          </List.Item>
        </List>
        <Text>
          Within the Linkdean application, we limit page information sent for statistics. For example, page paths may be
          recorded as <code>/library</code> or <code>/bookmarks/$id</code> rather than including individual bookmark
          IDs, folder names, search terms or bookmark titles.
        </Text>
        <Text>
          GoatCounter uses technical information to generate aggregate statistics such as browser type, operating system
          and country. According to its operation, it does not retain the IP address as part of the stored statistics
          and uses a short-lived, privacy-preserving identifier to distinguish repeat visits.
        </Text>
        <Text>
          Nothing is sent for these statistics where your browser sends a supported Do Not Track or Global Privacy
          Control signal.
        </Text>
        <Text>The resulting statistics are intended to be aggregate and are not linked to your Linkdean account.</Text>
      </LegalSection>

      <LegalSection heading="5. Why we use your information">
        <Text>Depending on the circumstances, we process your personal data to:</Text>
        <List>
          <List.Item>Provide the Linkdean service you requested.</List.Item>
          <List.Item>Create and manage your account.</List.Item>
          <List.Item>Store, organize and search your bookmarks and other content.</List.Item>
          <List.Item>Provide sharing and public publishing features that you choose to use.</List.Item>
          <List.Item>Send service emails such as verification, password reset and invitations.</List.Item>
          <List.Item>Provide API and browser extension functionality.</List.Item>
          <List.Item>Keep the service secure and prevent abuse.</List.Item>
          <List.Item>Investigate suspected breaches of our Terms.</List.Item>
          <List.Item>Comply with legal obligations.</List.Item>
          <List.Item>Maintain and improve the service.</List.Item>
        </List>
        <LegalSubsection heading="Our legal bases">
          <Text>Where UK GDPR or similar data protection laws apply, we generally rely on:</Text>
          <List>
            <List.Item>
              <strong>Contract</strong>, where processing is necessary to provide the service you requested.
            </List.Item>
            <List.Item>
              <strong>Legitimate interests</strong>, where processing is reasonably necessary to secure, protect and
              improve the service, provided those interests are not overridden by your rights.
            </List.Item>
            <List.Item>
              <strong>Legal obligations</strong>, where we are required to process or retain information by law.
            </List.Item>
            <List.Item>
              <strong>Consent</strong>, where applicable law requires it and we ask for it.
            </List.Item>
          </List>
        </LegalSubsection>
      </LegalSection>

      <LegalSection heading="6. Security and abuse prevention">
        <Text>
          We use reasonable technical and organizational measures designed to protect the service and the information we
          process.
        </Text>
        <Text>No internet service can guarantee absolute security.</Text>
        <Text>
          For security and abuse prevention, we may process technical information such as IP addresses and request
          information.
        </Text>
        <Text>
          For example, rate limits may count requests by IP address in memory to help protect the service from abuse.
        </Text>
      </LegalSection>

      <LegalSection heading="7. Who else processes your information">
        <Text>We may use service providers to operate Linkdean.</Text>
        <Text>Depending on the configuration of an instance, these may include the following.</Text>
        <LegalSubsection heading="Email delivery">
          <Text>
            The SMTP provider configured for the instance receives the email address and message content necessary to
            deliver emails we send, such as verification emails, password resets and invitations.
          </Text>
        </LegalSubsection>
        <LegalSubsection heading="Cloudflare">
          <Text>
            All traffic to linkdean.app, app.linkdean.app and docs.linkdean.app passes through Cloudflare. It provides
            our DNS, encrypted connections (HTTPS), caching and protection against attacks. Cloudflare therefore
            processes your IP address, information about your browser and the requests you make, including their
            content, as they pass through its network.
          </Text>
          <Text>
            The sign-up page also loads Cloudflare Turnstile to help distinguish people from automated traffic.
          </Text>
          <Text>
            Cloudflare may set strictly necessary security cookies, such as <code>__cf_bm</code>, when it checks for
            automated traffic. It processes this information in accordance with its own privacy documentation.
          </Text>
        </LegalSubsection>
        <LegalSubsection heading="Single sign-on providers">
          <Text>
            If you choose to sign in using an external identity provider, that provider may provide us with information
            such as your email address and name.
          </Text>
          <Text>
            The identity provider&apos;s own privacy policy also applies to its processing of your information.
          </Text>
        </LegalSubsection>
        <LegalSubsection heading="Search">
          <Text>Bookmark search may use a Meilisearch server operated as part of the Linkdean instance.</Text>
          <Text>
            Where the search infrastructure is operated by the instance operator, it is not an independent third party.
          </Text>
          <Text>Vault content is not indexed for search.</Text>
        </LegalSubsection>
        <Text>
          We may also disclose information where reasonably necessary to comply with applicable law, enforce our Terms,
          protect the service or protect the rights, safety or security of others.
        </Text>
        <Text>We do not sell your personal data.</Text>
      </LegalSection>

      <LegalSection heading="8. International data transfers">
        <Text>
          Your information may be processed in countries other than the country where you live, depending on where the
          Linkdean instance and its service providers are hosted.
        </Text>
        <Text>
          Where data protection law requires safeguards for international transfers, we will use an appropriate lawful
          transfer mechanism.
        </Text>
        <Text>
          You may contact us using the details in section 13 if you want more information about the locations where your
          data is processed.
        </Text>
      </LegalSection>

      <LegalSection heading="9. Public and shared content">
        <Text>Content that you choose to make public may be visible to anyone who can access it.</Text>
        <Text>Content that you share with specific people may be visible to those people.</Text>
        <Text>
          Your email address is not displayed to other users unless you choose to disclose it or another feature
          explicitly requires it.
        </Text>
        <Text>
          Public content may be copied, saved or shared by other people. Removing it from Linkdean may not remove copies
          made outside the service.
        </Text>
        <Text>If you report content, the report may include:</Text>
        <List>
          <List.Item>Information about your account.</List.Item>
          <List.Item>The content or account being reported.</List.Item>
          <List.Item>Your reason for making the report.</List.Item>
        </List>
        <Text>Reports may be visible to the administrators or operators responsible for handling them.</Text>
      </LegalSection>

      <LegalSection heading="10. Browser extension">
        <Text>
          The Linkdean browser extension for Chrome and Firefox works with the Linkdean instance you sign it into, such
          as <code>app.linkdean.app</code> or another Linkdean instance.
        </Text>
        <Text>
          When you choose to save a page using the extension&apos;s button, menu or shortcut, the extension may send the
          following information to your Linkdean instance:
        </Text>
        <List>
          <List.Item>The page address.</List.Item>
          <List.Item>The page title.</List.Item>
          <List.Item>Text you selected to save.</List.Item>
        </List>
        <Text>
          The extension may also read your folders and tags from your Linkdean instance to help you organize bookmarks.
        </Text>
        <Text>
          The extension does not send information about pages you visit unless you use a feature that requires that
          information.
        </Text>
        <Text>
          If you enable <strong>&ldquo;Color the icon on saved pages&rdquo;</strong>, the extension checks the address
          of your current browser tab against short hashes of addresses you have previously saved.
        </Text>
        <Text>The extension downloads those hashes from your Linkdean instance.</Text>
        <Text>The addresses of pages you visit are not sent to the instance for that check.</Text>
        <Text>
          Signing in to the extension creates an API key for the extension. The key is stored in the extension&apos;s
          own browser storage.
        </Text>
        <Text>Signing out of the extension revokes that key.</Text>
        <Text>
          You can also revoke extension API keys through <strong>Settings → API keys</strong>.
        </Text>
      </LegalSection>

      <LegalSection heading="11. How long we keep information">
        <Text>
          We keep personal data only for as long as reasonably necessary for the purposes described in this Privacy
          Policy.
        </Text>
        <Text>The retention period depends on the type of information and how it is used.</Text>
        <Text>For example:</Text>
        <List>
          <List.Item>Account and content data is generally kept while your account remains active.</List.Item>
          <List.Item>
            Deleted accounts and content may remain available for a short grace period and may remain temporarily in
            backups or technical systems before being permanently removed.
          </List.Item>
          <List.Item>
            Security, abuse and legal records may be retained for longer where reasonably necessary.
          </List.Item>
          <List.Item>Information may be retained where required by law.</List.Item>
        </List>
        <Text>
          When information is no longer needed, we will delete it, anonymize it or otherwise securely dispose of it
          where reasonably practical.
        </Text>
      </LegalSection>

      <LegalSection heading="12. Your choices and rights">
        <Text>Depending on applicable law, you may have rights to:</Text>
        <List>
          <List.Item>Access your personal data.</List.Item>
          <List.Item>Correct inaccurate personal data.</List.Item>
          <List.Item>Request deletion of your personal data.</List.Item>
          <List.Item>Restrict certain processing.</List.Item>
          <List.Item>Object to certain processing.</List.Item>
          <List.Item>Receive a portable copy of certain information.</List.Item>
          <List.Item>Withdraw consent where processing is based on consent.</List.Item>
          <List.Item>Complain to a data protection authority.</List.Item>
        </List>
        <Text>
          You can export your bookmarks, change your profile and email address, and delete your account through the
          settings provided by the service.
        </Text>
        <Text>To exercise other rights, contact us using the details in section 13.</Text>
        <Text>
          If you are in the United Kingdom, you may also have the right to complain to the Information
          Commissioner&apos;s Office.
        </Text>
        <Text>Your rights may be subject to legal limitations and exceptions.</Text>
      </LegalSection>

      <LegalSection heading="13. Contact">
        <Text>For privacy questions or requests about your personal data, contact:</Text>
        <Text fw={600}>
          <MailLink address={OPERATOR.privacyEmail} />
        </Text>
        <Text>For abuse reports, contact:</Text>
        <Text fw={600}>
          <MailLink address={OPERATOR.abuseEmail} />
        </Text>
      </LegalSection>

      <LegalSection heading="14. Children">
        <Text>
          Linkdean is not intended for use by children where their use would require parental consent under applicable
          law.
        </Text>
        <Text>
          If you believe a child has provided personal information to us in circumstances where they were not permitted
          to do so, contact us and we will review the situation.
        </Text>
      </LegalSection>

      <LegalSection heading="15. Changes to this Privacy Policy">
        <Text>We may update this Privacy Policy from time to time.</Text>
        <Text>The version date at the top of this page will show when it was last updated.</Text>
        <Text>
          For material changes, we will make reasonable efforts to provide notice before the changes take effect, where
          appropriate.
        </Text>
        <Text>The latest version will always be published through the service or website.</Text>
      </LegalSection>
    </LegalLayout>
  )
}
