import { useEffect, useState, type ReactElement } from 'react'
import {
  Button,
  Group,
  NumberInput,
  PasswordInput,
  Select,
  Stack,
  Switch,
  Table,
  Tabs,
  Text,
  TextInput,
  ThemeIcon,
  Title,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconBuilding, IconTrash } from '@tabler/icons-react'
import { useParams } from '@tanstack/react-router'
import { QueryErrorAlert } from '@/components'
import {
  useApproveDomainJoinRequest,
  useArchiveTeam,
  useCreateOrganizationDomain,
  useCreateOrganizationInvite,
  useCreateOrganizationInviteLink,
  useCreateOrganizationScimToken,
  useDeleteOrganization,
  useDeleteOrganizationDomain,
  useDeleteOrganizationSso,
  useDeleteTeam,
  useDenyDomainJoinRequest,
  useOrganization,
  useOrganizationDomainJoinRequests,
  useOrganizationDomains,
  useOrganizationInviteLinks,
  useOrganizationInvites,
  useOrganizationMembers,
  useOrganizationScimTokens,
  useOrganizationSso,
  useOrganizationTeams,
  usePatchOrganizationDomain,
  usePatchOrganizationInviteLinks,
  useRestoreOrganization,
  useRevokeOrganizationInvite,
  useRevokeOrganizationInviteLink,
  useRevokeOrganizationScimToken,
  useUnarchiveTeam,
  useUpsertOrganizationSso,
  useVerifyOrganizationDomain,
} from '@/hooks'
import { OrganizationAuditPanel } from './OrganizationAuditPanel'

function joinUrl(code: string): string {
  const base = typeof window !== 'undefined' ? window.location.origin : ''
  return `${base}/org-join?code=${encodeURIComponent(code)}`
}

export function OrganizationSettingsPage(): ReactElement {
  const { id } = useParams({ from: '/_app/organizations/$id' })
  const org = useOrganization(id)
  const members = useOrganizationMembers(id)
  const teams = useOrganizationTeams(id)
  const emailInvites = useOrganizationInvites(id)
  const createEmailInvite = useCreateOrganizationInvite(id)
  const revokeEmailInvite = useRevokeOrganizationInvite(id)
  const linksEnabled = !!org.data?.invite_links_enabled
  const inviteLinks = useOrganizationInviteLinks(id, linksEnabled)
  const delOrg = useDeleteOrganization(id)
  const restoreOrg = useRestoreOrganization(id)
  const archiveTeam = useArchiveTeam(id)
  const unarchiveTeam = useUnarchiveTeam(id)
  const deleteTeam = useDeleteTeam(id)
  const patchLinks = usePatchOrganizationInviteLinks(id)
  const createLink = useCreateOrganizationInviteLink(id)
  const revokeLink = useRevokeOrganizationInviteLink(id)
  const domains = useOrganizationDomains(id)
  const createDomain = useCreateOrganizationDomain(id)
  const deleteDomain = useDeleteOrganizationDomain(id)
  const verifyDomain = useVerifyOrganizationDomain(id)
  const patchDomain = usePatchOrganizationDomain(id)
  const joinRequests = useOrganizationDomainJoinRequests(id)
  const approveJoin = useApproveDomainJoinRequest(id)
  const denyJoin = useDenyDomainJoinRequest(id)
  const sso = useOrganizationSso(id)
  const upsertSso = useUpsertOrganizationSso(id)
  const deleteSso = useDeleteOrganizationSso(id)
  const scimTokens = useOrganizationScimTokens(id)
  const createScimToken = useCreateOrganizationScimToken(id)
  const revokeScimToken = useRevokeOrganizationScimToken(id)
  const [confirmName, setConfirmName] = useState('')
  const [freshScimToken, setFreshScimToken] = useState<string | null>(null)
  const [tab, setTab] = useState<string | null>('members')
  const [inviteEmail, setInviteEmail] = useState('')
  const [inviteRole, setInviteRole] = useState<'admin' | 'member' | 'guest'>('member')
  const [inviteGuestTeamId, setInviteGuestTeamId] = useState<string | null>(null)
  const [linkRole, setLinkRole] = useState<'admin' | 'member' | 'guest'>('member')
  const [expiresDays, setExpiresDays] = useState<number | string>(7)
  const [maxUses, setMaxUses] = useState<number | string>('')
  const [freshCode, setFreshCode] = useState<string | null>(null)
  const [guestTeamId, setGuestTeamId] = useState<string | null>(null)
  const [newDomain, setNewDomain] = useState('')
  const [ssoIssuer, setSsoIssuer] = useState('')
  const [ssoClientId, setSsoClientId] = useState('')
  const [ssoSecret, setSsoSecret] = useState('')
  const [ssoDisplayName, setSsoDisplayName] = useState('Company SSO')
  const [ssoEnabled, setSsoEnabled] = useState(false)
  const [ssoLoadedKey, setSsoLoadedKey] = useState<string | null>(null)

  const canAdmin = org.data?.role === 'owner' || org.data?.role === 'admin'
  const pendingDelete = !!org.data?.deleted_at
  const liveTeams = (teams.data ?? []).filter((t) => !t.archived_at && !t.deleted_at)
  const hasVerifiedDomain = (domains.data ?? []).some((d) => !!d.verified_at)

  useEffect(() => {
    if (!sso.isSuccess) return
    const key = sso.data?.updated_at ?? 'none'
    if (ssoLoadedKey === key) return
    setSsoIssuer(sso.data?.issuer_url ?? '')
    setSsoClientId(sso.data?.client_id ?? '')
    setSsoDisplayName(sso.data?.display_name ?? 'Company SSO')
    setSsoEnabled(sso.data?.enabled ?? false)
    setSsoSecret('')
    setSsoLoadedKey(key)
  }, [sso.isSuccess, sso.data, ssoLoadedKey])

  return (
    <Stack gap="xl">
      <Group gap="md">
        <ThemeIcon size={40} radius="md" variant="light" color="teal">
          <IconBuilding size={22} stroke={1.5} />
        </ThemeIcon>
        <div>
          <Title order={2}>{org.data?.name ?? 'Organization'}</Title>
          <Text size="sm" c="dimmed">
            Organization settings
          </Text>
        </div>
      </Group>

      {org.isError ? <QueryErrorAlert error={org.error} onRetry={() => void org.refetch()} /> : null}

      {pendingDelete ? (
        <Stack gap="sm">
          <Text>This organization is scheduled for deletion. Owners can restore it within 30 days.</Text>
          {org.data?.role === 'owner' ? (
            <Button
              loading={restoreOrg.isPending}
              onClick={() =>
                restoreOrg.mutate(undefined, {
                  onSuccess: () => notifications.show({ message: 'Organization restored' }),
                  onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                })
              }
            >
              Restore organization
            </Button>
          ) : null}
        </Stack>
      ) : null}

      {canAdmin && !pendingDelete ? (
        <Tabs value={tab} onChange={setTab}>
          <Tabs.List>
            <Tabs.Tab value="members">Members</Tabs.Tab>
            <Tabs.Tab value="teams">Teams</Tabs.Tab>
            <Tabs.Tab value="invite-links">Invite links</Tabs.Tab>
            <Tabs.Tab value="domains">Domains</Tabs.Tab>
            <Tabs.Tab value="sso">SSO</Tabs.Tab>
            <Tabs.Tab value="audit">Audit log</Tabs.Tab>
            <Tabs.Tab value="danger">Danger zone</Tabs.Tab>
          </Tabs.List>

          <Tabs.Panel value="members" pt="md">
            <Stack gap="lg" maw={720}>
              <Stack gap="sm">
                <Title order={4}>Invite by email</Title>
                <Text size="sm" c="dimmed">
                  Sends an email when mail is configured. The invitee accepts from Notifications after signing in with
                  that address.
                </Text>
                <Group align="flex-end" grow>
                  <TextInput
                    label="Email"
                    placeholder="teammate@example.com"
                    value={inviteEmail}
                    onChange={(e) => setInviteEmail(e.currentTarget.value)}
                  />
                  <Select
                    label="Role"
                    data={[
                      { value: 'member', label: 'Member' },
                      { value: 'admin', label: 'Admin' },
                      { value: 'guest', label: 'Guest' },
                    ]}
                    value={inviteRole}
                    onChange={(v) => setInviteRole((v as 'admin' | 'member' | 'guest') ?? 'member')}
                  />
                </Group>
                {inviteRole === 'guest' ? (
                  <Select
                    label="Team for guests"
                    placeholder="Pick a team"
                    data={liveTeams.map((t) => ({ value: t.id, label: t.name }))}
                    value={inviteGuestTeamId}
                    onChange={setInviteGuestTeamId}
                  />
                ) : null}
                <Button
                  loading={createEmailInvite.isPending}
                  disabled={!inviteEmail.trim() || (inviteRole === 'guest' && !inviteGuestTeamId)}
                  onClick={() =>
                    createEmailInvite.mutate(
                      {
                        email: inviteEmail.trim(),
                        role: inviteRole,
                        teams:
                          inviteRole === 'guest' && inviteGuestTeamId ? [{ team_id: inviteGuestTeamId }] : undefined,
                      },
                      {
                        onSuccess: () => {
                          setInviteEmail('')
                          setInviteGuestTeamId(null)
                          notifications.show({ message: 'Invite sent' })
                        },
                        onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                      }
                    )
                  }
                >
                  Send invite
                </Button>
              </Stack>

              <Stack gap="sm">
                <Title order={4}>Pending invites</Title>
                {emailInvites.isError ? (
                  <QueryErrorAlert error={emailInvites.error} onRetry={() => void emailInvites.refetch()} />
                ) : (emailInvites.data ?? []).filter((i) => !i.rejected_at).length === 0 ? (
                  <Text size="sm" c="dimmed">
                    No pending invites.
                  </Text>
                ) : (
                  <Table striped highlightOnHover>
                    <Table.Thead>
                      <Table.Tr>
                        <Table.Th>Email</Table.Th>
                        <Table.Th>Role</Table.Th>
                        <Table.Th />
                      </Table.Tr>
                    </Table.Thead>
                    <Table.Tbody>
                      {(emailInvites.data ?? [])
                        .filter((i) => !i.rejected_at)
                        .map((i) => (
                          <Table.Tr key={i.id}>
                            <Table.Td>{i.invitee_email}</Table.Td>
                            <Table.Td>{i.role}</Table.Td>
                            <Table.Td>
                              <Button
                                size="xs"
                                color="red"
                                variant="light"
                                loading={revokeEmailInvite.isPending}
                                onClick={() =>
                                  revokeEmailInvite.mutate(i.id, {
                                    onSuccess: () => notifications.show({ message: 'Invite revoked' }),
                                    onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                                  })
                                }
                              >
                                Revoke
                              </Button>
                            </Table.Td>
                          </Table.Tr>
                        ))}
                    </Table.Tbody>
                  </Table>
                )}
              </Stack>

              <Stack gap="sm">
                <Title order={4}>Members</Title>
                {members.isError ? (
                  <QueryErrorAlert error={members.error} onRetry={() => void members.refetch()} />
                ) : (
                  <Table striped highlightOnHover>
                    <Table.Thead>
                      <Table.Tr>
                        <Table.Th>Email</Table.Th>
                        <Table.Th>Role</Table.Th>
                      </Table.Tr>
                    </Table.Thead>
                    <Table.Tbody>
                      {(members.data ?? []).map((m) => (
                        <Table.Tr key={m.user_id}>
                          <Table.Td>{m.display_name ? `${m.display_name} (${m.email})` : m.email}</Table.Td>
                          <Table.Td>{m.role}</Table.Td>
                        </Table.Tr>
                      ))}
                    </Table.Tbody>
                  </Table>
                )}
              </Stack>
            </Stack>
          </Tabs.Panel>

          <Tabs.Panel value="teams" pt="md">
            <Stack gap="sm">
              {(teams.data ?? []).map((t) => (
                <Group key={t.id} justify="space-between">
                  <Text>
                    {t.name}
                    {t.is_default ? ' (default)' : ''}
                    {t.archived_at ? ' — archived' : ''}
                  </Text>
                  <Group gap="xs">
                    {t.archived_at ? (
                      <Button
                        size="xs"
                        variant="light"
                        loading={unarchiveTeam.isPending}
                        onClick={() =>
                          unarchiveTeam.mutate(t.id, {
                            onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                          })
                        }
                      >
                        Restore
                      </Button>
                    ) : (
                      <Button
                        size="xs"
                        variant="light"
                        loading={archiveTeam.isPending}
                        onClick={() =>
                          archiveTeam.mutate(t.id, {
                            onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                          })
                        }
                      >
                        Archive
                      </Button>
                    )}
                    {!t.is_default ? (
                      <Button
                        size="xs"
                        color="red"
                        variant="light"
                        leftSection={<IconTrash size={14} />}
                        loading={deleteTeam.isPending}
                        onClick={() =>
                          deleteTeam.mutate(t.id, {
                            onSuccess: () => notifications.show({ message: 'Team deleted' }),
                            onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                          })
                        }
                      >
                        Delete
                      </Button>
                    ) : null}
                  </Group>
                </Group>
              ))}
            </Stack>
          </Tabs.Panel>

          <Tabs.Panel value="invite-links" pt="md">
            <Stack gap="md" maw={640}>
              <Switch
                label="Allow invite links"
                description="Off by default. When on, admins can create shareable join links."
                checked={linksEnabled}
                disabled={patchLinks.isPending}
                onChange={(e) =>
                  patchLinks.mutate(e.currentTarget.checked, {
                    onError: (err) => notifications.show({ color: 'red', message: String(err) }),
                  })
                }
              />

              {linksEnabled ? (
                <>
                  <Group align="flex-end" grow>
                    <Select
                      label="Role"
                      data={[
                        { value: 'member', label: 'Member' },
                        { value: 'admin', label: 'Admin' },
                        { value: 'guest', label: 'Guest' },
                      ]}
                      value={linkRole}
                      onChange={(v) => setLinkRole((v as 'admin' | 'member' | 'guest') ?? 'member')}
                    />
                    <NumberInput
                      label="Expires (days)"
                      min={1}
                      max={90}
                      value={expiresDays}
                      onChange={setExpiresDays}
                    />
                    <NumberInput
                      label="Max uses"
                      placeholder="Unlimited"
                      min={1}
                      value={maxUses}
                      onChange={setMaxUses}
                    />
                  </Group>
                  {linkRole === 'guest' ? (
                    <Select
                      label="Team for guests"
                      placeholder="Pick a team"
                      data={liveTeams.map((t) => ({ value: t.id, label: t.name }))}
                      value={guestTeamId}
                      onChange={setGuestTeamId}
                    />
                  ) : null}
                  <Button
                    loading={createLink.isPending}
                    disabled={linkRole === 'guest' && !guestTeamId}
                    onClick={() =>
                      createLink.mutate(
                        {
                          role: linkRole,
                          expires_in_days: typeof expiresDays === 'number' ? expiresDays : 7,
                          max_uses: typeof maxUses === 'number' ? maxUses : null,
                          teams: linkRole === 'guest' && guestTeamId ? [{ team_id: guestTeamId }] : undefined,
                        },
                        {
                          onSuccess: (res) => {
                            setFreshCode(res.code)
                            notifications.show({ message: 'Invite link created — copy it now' })
                          },
                          onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                        }
                      )
                    }
                  >
                    Create invite link
                  </Button>
                  {freshCode ? (
                    <Stack gap="xs">
                      <Text size="sm">New link (shown once):</Text>
                      <TextInput readOnly value={joinUrl(freshCode)} />
                      <Button
                        variant="light"
                        onClick={() => {
                          void navigator.clipboard.writeText(joinUrl(freshCode))
                          notifications.show({ message: 'Copied' })
                        }}
                      >
                        Copy URL
                      </Button>
                    </Stack>
                  ) : null}

                  {inviteLinks.isError ? (
                    <QueryErrorAlert error={inviteLinks.error} onRetry={() => void inviteLinks.refetch()} />
                  ) : (
                    <Table striped highlightOnHover>
                      <Table.Thead>
                        <Table.Tr>
                          <Table.Th>Prefix</Table.Th>
                          <Table.Th>Role</Table.Th>
                          <Table.Th>Uses</Table.Th>
                          <Table.Th>Status</Table.Th>
                          <Table.Th />
                        </Table.Tr>
                      </Table.Thead>
                      <Table.Tbody>
                        {(inviteLinks.data ?? []).map((link) => (
                          <Table.Tr key={link.id}>
                            <Table.Td>{link.code_prefix}…</Table.Td>
                            <Table.Td>{link.role}</Table.Td>
                            <Table.Td>
                              {link.use_count}
                              {link.max_uses != null ? ` / ${link.max_uses}` : ''}
                            </Table.Td>
                            <Table.Td>{link.status}</Table.Td>
                            <Table.Td>
                              {link.revoked_at ? null : (
                                <Button
                                  size="xs"
                                  color="red"
                                  variant="light"
                                  loading={revokeLink.isPending}
                                  onClick={() =>
                                    revokeLink.mutate(link.id, {
                                      onSuccess: () => notifications.show({ message: 'Invite link revoked' }),
                                      onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                                    })
                                  }
                                >
                                  Revoke
                                </Button>
                              )}
                            </Table.Td>
                          </Table.Tr>
                        ))}
                      </Table.Tbody>
                    </Table>
                  )}
                </>
              ) : null}
            </Stack>
          </Tabs.Panel>

          <Tabs.Panel value="domains" pt="md">
            <Stack gap="md" maw={720}>
              <Text size="sm" c="dimmed">
                Prove you control a domain with a DNS TXT record. Matching verified emails can auto-join or request to
                join.
              </Text>
              <Group align="flex-end" grow>
                <TextInput
                  label="Domain"
                  placeholder="acme.com"
                  value={newDomain}
                  onChange={(e) => setNewDomain(e.currentTarget.value)}
                />
                <Button
                  loading={createDomain.isPending}
                  onClick={() =>
                    createDomain.mutate(newDomain.trim(), {
                      onSuccess: () => {
                        setNewDomain('')
                        notifications.show({ message: 'Domain added — publish the TXT record' })
                      },
                      onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                    })
                  }
                >
                  Add domain
                </Button>
              </Group>

              {domains.isError ? (
                <QueryErrorAlert error={domains.error} onRetry={() => void domains.refetch()} />
              ) : (
                <Stack gap="lg">
                  {(domains.data ?? []).map((d) => (
                    <Stack key={d.id} gap="xs">
                      <Group justify="space-between">
                        <Text fw={600}>
                          {d.domain} {d.verified_at ? '(verified)' : '(pending)'}
                        </Text>
                        <Group gap="xs">
                          {!d.verified_at ? (
                            <Button
                              size="xs"
                              loading={verifyDomain.isPending}
                              onClick={() =>
                                verifyDomain.mutate(d.id, {
                                  onSuccess: () => notifications.show({ message: 'Domain verified' }),
                                  onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                                })
                              }
                            >
                              Verify DNS
                            </Button>
                          ) : null}
                          <Button
                            size="xs"
                            color="red"
                            variant="light"
                            loading={deleteDomain.isPending}
                            onClick={() =>
                              deleteDomain.mutate(d.id, {
                                onSuccess: () => notifications.show({ message: 'Domain removed' }),
                                onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                              })
                            }
                          >
                            Remove
                          </Button>
                        </Group>
                      </Group>
                      <TextInput label="TXT name" readOnly value={d.txt_name} />
                      <TextInput label="TXT value" readOnly value={d.txt_value} />
                      <Select
                        label="Join mode"
                        data={[
                          { value: 'auto_join', label: 'Auto-join matching emails' },
                          { value: 'request', label: 'Require admin approval' },
                        ]}
                        value={d.join_mode}
                        onChange={(v) => {
                          if (!v) return
                          patchDomain.mutate(
                            { domainId: d.id, join_mode: v as 'auto_join' | 'request' },
                            {
                              onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                            }
                          )
                        }}
                      />
                    </Stack>
                  ))}
                </Stack>
              )}

              <Title order={4}>Join requests</Title>
              {joinRequests.isError ? (
                <QueryErrorAlert error={joinRequests.error} onRetry={() => void joinRequests.refetch()} />
              ) : (
                <Table striped highlightOnHover>
                  <Table.Thead>
                    <Table.Tr>
                      <Table.Th>Email</Table.Th>
                      <Table.Th>Domain</Table.Th>
                      <Table.Th />
                    </Table.Tr>
                  </Table.Thead>
                  <Table.Tbody>
                    {(joinRequests.data ?? []).map((r) => (
                      <Table.Tr key={r.id}>
                        <Table.Td>{r.display_name ? `${r.display_name} (${r.email})` : r.email}</Table.Td>
                        <Table.Td>{r.domain}</Table.Td>
                        <Table.Td>
                          <Group gap="xs" justify="flex-end">
                            <Button
                              size="xs"
                              loading={approveJoin.isPending}
                              onClick={() =>
                                approveJoin.mutate(r.id, {
                                  onSuccess: () => notifications.show({ message: 'Request approved' }),
                                  onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                                })
                              }
                            >
                              Approve
                            </Button>
                            <Button
                              size="xs"
                              color="red"
                              variant="light"
                              loading={denyJoin.isPending}
                              onClick={() =>
                                denyJoin.mutate(r.id, {
                                  onSuccess: () => notifications.show({ message: 'Request denied' }),
                                  onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                                })
                              }
                            >
                              Deny
                            </Button>
                          </Group>
                        </Table.Td>
                      </Table.Tr>
                    ))}
                  </Table.Tbody>
                </Table>
              )}
            </Stack>
          </Tabs.Panel>

          <Tabs.Panel value="sso" pt="md">
            <Stack gap="md" maw={720}>
              <Text size="sm" c="dimmed">
                Connect one OpenID Connect IdP so members with a verified domain can sign in and join this organization.
                Register the redirect URI below at your IdP.
              </Text>
              {sso.isError ? (
                <QueryErrorAlert error={sso.error} onRetry={() => void sso.refetch()} />
              ) : (
                <>
                  <TextInput
                    label="Redirect URI"
                    readOnly
                    value={
                      sso.data?.redirect_uri ??
                      `${typeof window !== 'undefined' ? window.location.origin : ''}/api/auth/org-oidc/callback`
                    }
                  />
                  <TextInput
                    label="Issuer URL"
                    placeholder="https://idp.example/realms/acme"
                    value={ssoIssuer}
                    onChange={(e) => setSsoIssuer(e.currentTarget.value)}
                  />
                  <TextInput
                    label="Client ID"
                    value={ssoClientId}
                    onChange={(e) => setSsoClientId(e.currentTarget.value)}
                  />
                  <PasswordInput
                    label="Client secret"
                    description={
                      sso.data ? 'Leave blank to keep the current secret.' : 'Required when configuring SSO.'
                    }
                    value={ssoSecret}
                    onChange={(e) => setSsoSecret(e.currentTarget.value)}
                  />
                  <TextInput
                    label="Button label"
                    value={ssoDisplayName}
                    onChange={(e) => setSsoDisplayName(e.currentTarget.value)}
                  />
                  <Switch
                    label="Enable organization SSO"
                    description={
                      hasVerifiedDomain
                        ? 'Members discover this IdP from a matching work email.'
                        : 'Verify at least one domain before enabling.'
                    }
                    checked={ssoEnabled}
                    disabled={!hasVerifiedDomain && !ssoEnabled}
                    onChange={(e) => setSsoEnabled(e.currentTarget.checked)}
                  />
                  <Group>
                    <Button
                      loading={upsertSso.isPending}
                      onClick={() =>
                        upsertSso.mutate(
                          {
                            issuer_url: ssoIssuer.trim(),
                            client_id: ssoClientId.trim(),
                            client_secret: ssoSecret.trim() || undefined,
                            display_name: ssoDisplayName.trim() || 'Company SSO',
                            enabled: ssoEnabled,
                          },
                          {
                            onSuccess: () => {
                              setSsoSecret('')
                              notifications.show({ message: 'SSO settings saved' })
                            },
                            onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                          }
                        )
                      }
                    >
                      Save SSO
                    </Button>
                    {sso.data ? (
                      <Button
                        color="red"
                        variant="light"
                        loading={deleteSso.isPending}
                        onClick={() =>
                          deleteSso.mutate(undefined, {
                            onSuccess: () => {
                              setSsoLoadedKey(null)
                              notifications.show({ message: 'SSO configuration removed' })
                            },
                            onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                          })
                        }
                      >
                        Remove SSO
                      </Button>
                    ) : null}
                  </Group>

                  <Title order={4}>SCIM</Title>
                  <Text size="sm" c="dimmed">
                    Provision members from your IdP. Base URL:{' '}
                    {`${typeof window !== 'undefined' ? window.location.origin : ''}/api/scim/v2`}. Creating a new token
                    revokes the previous one.
                  </Text>
                  {freshScimToken ? <TextInput label="New token (copy now)" readOnly value={freshScimToken} /> : null}
                  <Button
                    loading={createScimToken.isPending}
                    onClick={() =>
                      createScimToken.mutate('SCIM', {
                        onSuccess: (res) => {
                          setFreshScimToken(res.token)
                          notifications.show({ message: 'SCIM token created — copy it now' })
                        },
                        onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                      })
                    }
                  >
                    Create SCIM token
                  </Button>
                  {scimTokens.isError ? (
                    <QueryErrorAlert error={scimTokens.error} onRetry={() => void scimTokens.refetch()} />
                  ) : (
                    <Table striped highlightOnHover>
                      <Table.Thead>
                        <Table.Tr>
                          <Table.Th>Prefix</Table.Th>
                          <Table.Th>Name</Table.Th>
                          <Table.Th />
                        </Table.Tr>
                      </Table.Thead>
                      <Table.Tbody>
                        {(scimTokens.data ?? []).map((t) => (
                          <Table.Tr key={t.id}>
                            <Table.Td>{t.prefix}</Table.Td>
                            <Table.Td>{t.name}</Table.Td>
                            <Table.Td>
                              <Button
                                size="xs"
                                color="red"
                                variant="light"
                                loading={revokeScimToken.isPending}
                                onClick={() =>
                                  revokeScimToken.mutate(t.id, {
                                    onSuccess: () => {
                                      setFreshScimToken(null)
                                      notifications.show({ message: 'SCIM token revoked' })
                                    },
                                    onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                                  })
                                }
                              >
                                Revoke
                              </Button>
                            </Table.Td>
                          </Table.Tr>
                        ))}
                      </Table.Tbody>
                    </Table>
                  )}
                </>
              )}
            </Stack>
          </Tabs.Panel>

          <Tabs.Panel value="audit" pt="md">
            <OrganizationAuditPanel organizationId={id} />
          </Tabs.Panel>

          <Tabs.Panel value="danger" pt="md">
            {org.data?.role === 'owner' ? (
              <Stack gap="sm" maw={420}>
                <Text size="sm">Type the organization name to schedule deletion. You can restore within 30 days.</Text>
                <TextInput
                  label="Confirm name"
                  value={confirmName}
                  onChange={(e) => setConfirmName(e.currentTarget.value)}
                />
                <Button
                  color="red"
                  disabled={confirmName.trim() !== (org.data?.name ?? '')}
                  loading={delOrg.isPending}
                  onClick={() =>
                    delOrg.mutate(confirmName, {
                      onSuccess: () => notifications.show({ message: 'Organization scheduled for deletion' }),
                      onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                    })
                  }
                >
                  Delete organization
                </Button>
              </Stack>
            ) : (
              <Text size="sm" c="dimmed">
                Only owners can delete the organization.
              </Text>
            )}
          </Tabs.Panel>
        </Tabs>
      ) : null}
    </Stack>
  )
}
