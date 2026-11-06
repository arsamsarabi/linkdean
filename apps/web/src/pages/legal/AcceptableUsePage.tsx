import { type ReactElement } from 'react'
import { List, Text } from '@mantine/core'
import { LegalLayout, LegalSection } from './LegalLayout'

export function AcceptableUsePage(): ReactElement {
  return (
    <LegalLayout title="Acceptable Use Policy">
      <LegalSection heading="1. Be decent">
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
      </LegalSection>
      <LegalSection heading="2. Protect the service">
        <List>
          <List.Item>No spam: mass invites, comments or shares that nobody asked for.</List.Item>
          <List.Item>
            No probing for weaknesses, scraping other users&apos; data, or getting around rate limits and quotas.
          </List.Item>
          <List.Item>No automated sign-ups. Use API keys for scripts and keep within the limits.</List.Item>
          <List.Item>Do not save links to private or internal addresses to make the server fetch them.</List.Item>
        </List>
      </LegalSection>
      <LegalSection heading="3. Enforcement">
        <Text>
          We may remove content, ban or delete accounts, and report illegal activity. Where we can, we say why. If you
          think we got it wrong, contact the administrator of this instance.
        </Text>
      </LegalSection>
      <LegalSection heading="4. Reporting">
        <Text>To report abuse, contact the administrator of this instance.</Text>
      </LegalSection>
    </LegalLayout>
  )
}
