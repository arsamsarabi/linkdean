import { type ReactElement } from 'react'
import { List, Text } from '@mantine/core'
import { LegalLayout, LegalSection, LegalSubsection } from './LegalLayout'

export function TermsPage(): ReactElement {
  return (
    <LegalLayout title="Terms of Service" noticeSubject="These Terms have">
      <LegalSection heading="1. Who we are and these Terms">
        <Text>These Terms apply to your use of the Linkdean instance on which you create or use an account.</Text>
        <Text>The operator of this instance is:</Text>
        <Text fw={600}>
          Arsam Sarabi (sole trader)
          <br />
          [Trading name, if different]
          <br />
          Contact: [contact email address]
        </Text>
        <Text>
          In these Terms, &ldquo;we&rdquo;, &ldquo;us&rdquo; and &ldquo;our&rdquo; mean the operator of this Linkdean
          instance.
        </Text>
        <Text>
          By creating an account, accessing or using Linkdean, you agree to these Terms and our Privacy Policy.
        </Text>
        <Text>If you do not agree to them, do not use the service.</Text>
      </LegalSection>

      <LegalSection heading="2. The service">
        <Text>Linkdean is a service for saving, organizing and sharing bookmarks and related content.</Text>
        <Text>
          The service and its features may change over time. We may add, remove, modify or discontinue features,
          particularly while Linkdean is in beta.
        </Text>
        <Text>We do not guarantee that every feature will remain available.</Text>
      </LegalSection>

      <LegalSection heading="3. Public beta and availability">
        <Text>Linkdean is in public beta.</Text>
        <Text>
          The service may change, contain errors, become unavailable or be discontinued without notice. We do not
          promise any particular level of availability, uptime or data retention.
        </Text>
        <Text>
          You should regularly export any bookmarks or other content that is important to you and keep your own backups.
        </Text>
        <Text>
          We may take the service offline for maintenance, security, technical reasons or other operational reasons.
        </Text>
        <Text>Nothing in this section removes rights that cannot legally be excluded.</Text>
      </LegalSection>

      <LegalSection heading="4. Your account">
        <Text>You must provide accurate account information and keep it reasonably up to date.</Text>
        <Text>You are responsible for:</Text>
        <List>
          <List.Item>Keeping your password, API keys and other account credentials confidential.</List.Item>
          <List.Item>
            Activity carried out through your account, unless you have told us about unauthorized access without
            unreasonable delay.
          </List.Item>
          <List.Item>Ensuring that your use of the service complies with these Terms and applicable law.</List.Item>
        </List>
        <Text>You must tell us promptly if you believe someone has gained unauthorized access to your account.</Text>
        <Text>
          We may suspend or restrict an account where reasonably necessary to protect the service, its users or others.
        </Text>
      </LegalSection>

      <LegalSection heading="5. Your content">
        <Text>You keep ownership of the content you add to Linkdean.</Text>
        <Text>This includes, where applicable, your bookmarks, notes, tags, lists, images and other content.</Text>
        <Text>
          You grant us a limited, non-exclusive, worldwide license to host, store, process, reproduce and display your
          content only to the extent reasonably necessary to:
        </Text>
        <List>
          <List.Item>Operate and provide the service.</List.Item>
          <List.Item>Display your content to you.</List.Item>
          <List.Item>Share your content with people you choose.</List.Item>
          <List.Item>Make content publicly available when you choose to publish it.</List.Item>
          <List.Item>Maintain, secure and improve the service.</List.Item>
        </List>
        <Text>
          This license ends when your content is deleted, except where we need to retain it for a reasonable period for
          technical, backup, legal or security reasons.
        </Text>
        <Text>
          You confirm that you have the necessary rights and permissions to save, upload, publish or share the content
          you add to Linkdean.
        </Text>
      </LegalSection>

      <LegalSection heading="6. Acceptable use" id="acceptable-use">
        <LegalSubsection heading="6.1 Illegal or harmful content">
          <Text>You must not use Linkdean to save, upload, publish or share:</Text>
          <List>
            <List.Item>Content that is illegal under applicable law.</List.Item>
            <List.Item>Material that sexually exploits or endangers children.</List.Item>
            <List.Item>Content used to harass, threaten, intimidate or unlawfully target another person.</List.Item>
            <List.Item>Hate content where prohibited by applicable law.</List.Item>
            <List.Item>
              Personal information about another person that you do not have the right to disclose, including doxxing.
            </List.Item>
            <List.Item>Malware, phishing pages or links intended to defraud, deceive or compromise others.</List.Item>
            <List.Item>
              Content that infringes another person&apos;s intellectual property or other rights where you do not have
              permission or another lawful right to use or share it.
            </List.Item>
          </List>
        </LegalSubsection>
        <LegalSubsection heading="6.2 Protecting the service">
          <Text>You must not:</Text>
          <List>
            <List.Item>Send spam or unwanted mass invitations, comments or shares.</List.Item>
            <List.Item>Probe, scan or test the service for vulnerabilities without our permission.</List.Item>
            <List.Item>Attempt to access another person&apos;s account or data without authorization.</List.Item>
            <List.Item>Scrape or collect other users&apos; data without permission.</List.Item>
            <List.Item>Circumvent security measures, rate limits, quotas or access controls.</List.Item>
            <List.Item>Create accounts automatically without permission.</List.Item>
            <List.Item>Use the service in a way that places an unreasonable load on its infrastructure.</List.Item>
            <List.Item>
              Use saved links or other features to make our servers access private, internal or otherwise restricted
              network addresses without permission.
            </List.Item>
            <List.Item>Interfere with the operation or security of the service.</List.Item>
          </List>
          <Text>If we provide an API, you must use it in accordance with any applicable limits and documentation.</Text>
        </LegalSubsection>
        <LegalSubsection heading="6.3 Enforcement">
          <Text>We may investigate suspected breaches of these Terms.</Text>
          <Text>Where reasonably necessary, we may:</Text>
          <List>
            <List.Item>Remove or restrict access to content.</List.Item>
            <List.Item>Suspend or terminate accounts.</List.Item>
            <List.Item>Restrict features or API access.</List.Item>
            <List.Item>Take other reasonable action to protect the service, its users or others.</List.Item>
            <List.Item>Report suspected illegal activity to relevant authorities where appropriate.</List.Item>
          </List>
          <Text>Where reasonably possible and appropriate, we will explain the reason for our action.</Text>
          <Text>If you believe we made a mistake, you may contact us using the details in section 11.</Text>
        </LegalSubsection>
      </LegalSection>

      <LegalSection heading="7. Reporting abuse">
        <Text>You may report content or behavior that you believe breaches these Terms or applicable law.</Text>
        <Text>
          Where available, use the <strong>Report</strong> feature on public lists, profiles or comments.
        </Text>
        <Text>You may also contact us at:</Text>
        <Text fw={600}>[Abuse contact email address]</Text>
        <Text>
          Reports may be reviewed by the administrators or operators of this Linkdean instance. We may process
          information provided in a report, including information about the reporter, the reported content and the
          reason for the report.
        </Text>
      </LegalSection>

      <LegalSection heading="8. Ending your account">
        <Text>You may delete your account through the settings provided by the service.</Text>
        <Text>
          Account deletion may include a short grace period during which you can cancel the deletion. After the
          applicable deletion process is complete, we will delete or anonymize your account and content in accordance
          with our Privacy Policy, except where we need to retain information for legitimate technical, security or
          legal reasons.
        </Text>
        <Text>
          We may suspend or terminate your account if you materially breach these Terms or where reasonably necessary to
          protect the service, its users or others.
        </Text>
        <Text>Where appropriate, we may give you notice before taking action.</Text>
      </LegalSection>

      <LegalSection heading="9. Intellectual property">
        <Text>
          The Linkdean software, branding, design and other material provided by us are protected by applicable
          intellectual property laws.
        </Text>
        <Text>
          Except for the limited right to use the service in accordance with these Terms, we do not give you ownership
          of our intellectual property.
        </Text>
        <Text>
          You must not copy, modify, distribute, sell or attempt to exploit the service except where permitted by
          applicable law or with our permission.
        </Text>
        <Text>This section does not apply to content that belongs to you.</Text>
      </LegalSection>

      <LegalSection heading="10. Disclaimers and limitation of liability">
        <Text>The service is provided on an &ldquo;as is&rdquo; and &ldquo;as available&rdquo; basis.</Text>
        <Text>To the maximum extent permitted by applicable law, we do not guarantee that the service will:</Text>
        <List>
          <List.Item>Be uninterrupted or error-free.</List.Item>
          <List.Item>Always be secure or available.</List.Item>
          <List.Item>Preserve all data.</List.Item>
          <List.Item>Meet your particular requirements.</List.Item>
          <List.Item>Be compatible with every device, browser or service.</List.Item>
        </List>
        <Text>You use the service at your own risk.</Text>
        <Text>To the maximum extent permitted by law, we are not liable for:</Text>
        <List>
          <List.Item>Loss of data.</List.Item>
          <List.Item>Loss of profits, revenue or business opportunity.</List.Item>
          <List.Item>Loss of goodwill.</List.Item>
          <List.Item>Indirect or consequential loss.</List.Item>
        </List>
        <Text>
          Nothing in these Terms excludes or limits liability where doing so would be unlawful, including liability that
          cannot legally be excluded or limited.
        </Text>
        <Text>
          If applicable law allows us to limit our total liability to you, our total liability arising from or relating
          to the service will not exceed the greater of:
        </Text>
        <List>
          <List.Item>
            The amount you paid us for the service during the 12 months before the event giving rise to the claim, or
          </List.Item>
          <List.Item>
            <strong>£100 GBP</strong>.
          </List.Item>
        </List>
        <Text>This limitation does not apply where applicable law does not allow it.</Text>
      </LegalSection>

      <LegalSection heading="11. Contact">
        <Text>For general questions about Linkdean or these Terms, contact:</Text>
        <Text fw={600}>[Contact email address]</Text>
        <Text>For abuse reports, contact:</Text>
        <Text fw={600}>[Abuse contact email address]</Text>
      </LegalSection>

      <LegalSection heading="12. Changes to these Terms">
        <Text>We may update these Terms from time to time.</Text>
        <Text>The version date at the top of this page will show when they were last updated.</Text>
        <Text>
          For material changes, we will make reasonable efforts to provide notice before the changes take effect, for
          example through the service or by email where appropriate.
        </Text>
        <Text>If you continue to use Linkdean after updated Terms take effect, you agree to the updated Terms.</Text>
        <Text>
          If you do not agree to the updated Terms, you should stop using the service and may delete your account.
        </Text>
      </LegalSection>

      <LegalSection heading="13. Governing law">
        <Text>
          These Terms are governed by the laws of <strong>[England and Wales / applicable jurisdiction]</strong>.
        </Text>
        <Text>
          The courts of <strong>[England and Wales / applicable jurisdiction]</strong> will have jurisdiction over
          disputes arising from these Terms, except where applicable consumer protection law gives you additional rights
          or requires another jurisdiction.
        </Text>
      </LegalSection>

      <LegalSection heading="14. General">
        <Text>
          If any part of these Terms is found to be invalid or unenforceable, the remaining parts will continue to
          apply.
        </Text>
        <Text>
          If we do not immediately enforce a provision of these Terms, that does not mean we have waived our right to
          enforce it later.
        </Text>
        <Text>
          These Terms do not create a partnership, employment relationship, agency relationship or joint venture between
          you and us.
        </Text>
      </LegalSection>
    </LegalLayout>
  )
}
