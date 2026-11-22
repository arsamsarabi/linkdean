import { useEffect, type ReactElement } from 'react'
import { Menu, Text, UnstyledButton } from '@mantine/core'
import { IconBuilding, IconCheck, IconChevronDown, IconUser } from '@tabler/icons-react'
import { useOrganizations, useOrganizationTeams } from '@/hooks'
import { useShellUi } from '@/stores'
import classes from './AppShell.module.css'

/**
 * Personal vs organization workspace. Shown when the account belongs to at least one org.
 * Archived teams are omitted; the default team is selected when entering an org.
 */
export function WorkspaceSwitcher(): ReactElement | null {
  const orgs = useOrganizations()
  const workspace = useShellUi((s) => s.workspace)
  const setPersonal = useShellUi((s) => s.setPersonalWorkspace)
  const setOrg = useShellUi((s) => s.setOrganizationWorkspace)
  const setTeam = useShellUi((s) => s.setWorkspaceTeam)

  const liveOrgs = (orgs.data ?? []).filter((o) => !o.deleted_at)
  const activeOrgId = workspace.kind === 'organization' ? workspace.organizationId : null
  const teams = useOrganizationTeams(activeOrgId ?? undefined)

  useEffect(() => {
    if (workspace.kind !== 'organization') return
    const list = (teams.data ?? []).filter((t) => !t.archived_at && !t.deleted_at)
    if (!list.length) return
    if (workspace.teamId && list.some((t) => t.id === workspace.teamId)) return
    const def = list.find((t) => t.is_default) ?? list[0]
    setTeam(def.id)
  }, [workspace, teams.data, setTeam])

  if (!liveOrgs.length) return null

  const label =
    workspace.kind === 'personal'
      ? 'Personal'
      : (liveOrgs.find((o) => o.id === workspace.organizationId)?.name ?? 'Organization')

  return (
    <Menu withinPortal position="bottom-start" width={260}>
      <Menu.Target>
        <UnstyledButton className={classes.workspaceSwitcher} aria-label="Workspace">
          {workspace.kind === 'personal' ? (
            <IconUser size={16} stroke={1.5} />
          ) : (
            <IconBuilding size={16} stroke={1.5} />
          )}
          <Text size="sm" fw={600} lineClamp={1} style={{ flex: 1 }}>
            {label}
          </Text>
          <IconChevronDown size={14} stroke={1.5} />
        </UnstyledButton>
      </Menu.Target>
      <Menu.Dropdown>
        <Menu.Label>Workspace</Menu.Label>
        <Menu.Item
          leftSection={<IconUser size={16} stroke={1.5} />}
          rightSection={workspace.kind === 'personal' ? <IconCheck size={14} /> : null}
          onClick={() => setPersonal()}
        >
          Personal
        </Menu.Item>
        <Menu.Divider />
        <Menu.Label>Organizations</Menu.Label>
        {liveOrgs.map((o) => (
          <Menu.Item
            key={o.id}
            leftSection={<IconBuilding size={16} stroke={1.5} />}
            rightSection={
              workspace.kind === 'organization' && workspace.organizationId === o.id ? <IconCheck size={14} /> : null
            }
            onClick={() => setOrg(o.id, null)}
          >
            {o.name}
          </Menu.Item>
        ))}
        {activeOrgId && ['owner', 'admin'].includes(liveOrgs.find((o) => o.id === activeOrgId)?.role ?? '') ? (
          <>
            <Menu.Divider />
            <Menu.Item component="a" href={`/organizations/${activeOrgId}`}>
              Organization settings
            </Menu.Item>
          </>
        ) : null}
        {workspace.kind === 'organization' && (teams.data ?? []).filter((t) => !t.archived_at).length > 1 ? (
          <>
            <Menu.Divider />
            <Menu.Label>Teams</Menu.Label>
            {(teams.data ?? [])
              .filter((t) => !t.archived_at && !t.deleted_at)
              .map((t) => (
                <Menu.Item
                  key={t.id}
                  rightSection={workspace.teamId === t.id ? <IconCheck size={14} /> : null}
                  onClick={() => setTeam(t.id)}
                >
                  {t.name}
                  {t.is_default ? ' (default)' : ''}
                </Menu.Item>
              ))}
          </>
        ) : null}
      </Menu.Dropdown>
    </Menu>
  )
}
