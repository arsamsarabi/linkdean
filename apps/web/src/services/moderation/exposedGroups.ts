import type { ExposedContent, ModerateItem } from '@/schemas'

export type ExposedRow = {
  id: string
  label: string
  detail?: string
  /** Share rows carry their own type. */
  subjectType?: string
}

export type GroupAction = {
  label: string
  action: string
  subjectType?: string
  /** Set when the change needs a confirm step. */
  confirm?: string
}

export type ExposedGroup = {
  key: string
  title: string
  rows: ExposedRow[]
  actions: GroupAction[]
}

const TRASH_CONFIRM = 'Moves the selected items to the owner’s Trash. The owner can restore them.'

/** Groups with something in them, each with what an admin can do. */
export function exposedGroups(content: ExposedContent, userId: string): ExposedGroup[] {
  const groups: ExposedGroup[] = [
    {
      key: 'public_links',
      title: 'Public links',
      rows: content.public_links.map((c) => ({ id: c.id, label: c.name, detail: `/l/${c.public_slug ?? ''}` })),
      actions: [{ label: 'Turn off', action: 'public_link_disabled' }],
    },
    {
      key: 'categories',
      title: 'Visible categories',
      rows: content.categories.map((c) => ({ id: c.id, label: c.name, detail: `Visible to ${c.visibility}` })),
      actions: [
        { label: 'Set private', action: 'category_private' },
        { label: 'Move to Trash', action: 'removed_to_trash', subjectType: 'category', confirm: TRASH_CONFIRM },
      ],
    },
    {
      key: 'bookmarks',
      title: 'Bookmarks in exposed categories',
      rows: content.bookmarks.map((b) => ({ id: b.id, label: b.title || b.url, detail: b.url })),
      actions: [
        { label: 'Set private', action: 'bookmark_private' },
        { label: 'Move to Trash', action: 'removed_to_trash', subjectType: 'bookmark', confirm: TRASH_CONFIRM },
      ],
    },
    {
      key: 'profile',
      title: 'Profile',
      rows: content.profile_visibility
        ? [{ id: userId, label: 'Profile', detail: `Visible to ${content.profile_visibility}` }]
        : [],
      actions: [{ label: 'Hide', action: 'profile_hidden' }],
    },
    {
      key: 'shares',
      title: 'Shares',
      rows: content.shares.map((s) => ({
        id: s.id,
        label: s.title || 'Pending invite',
        detail: `${s.invitee} · ${s.access}`,
        subjectType: s.kind,
      })),
      actions: [{ label: 'Revoke', action: 'share_revoked' }],
    },
    {
      key: 'comments',
      title: 'Comments',
      rows: content.comments.map((c) => ({ id: c.id, label: c.body, detail: `On a ${c.subject_type}` })),
      actions: [
        {
          label: 'Delete',
          action: 'comment_deleted',
          confirm: 'Deleted comments cannot be restored. The text stays in the history below.',
        },
      ],
    },
  ]
  return groups.filter((g) => g.rows.length > 0)
}

export function moderateItems(group: ExposedGroup, action: GroupAction, selected: ReadonlySet<string>): ModerateItem[] {
  return group.rows
    .filter((r) => selected.has(r.id))
    .map((r) => ({
      action: action.action,
      subject_type: r.subjectType ?? action.subjectType,
      subject_id: r.id,
    }))
}

export function moderationLabel(action: string): string {
  return action.replaceAll('_', ' ')
}
