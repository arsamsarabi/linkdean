import { type ReactElement } from 'react'
import { Anchor, Badge, Breadcrumbs, Button, Group, Paper, SimpleGrid, Stack, Text, Title } from '@mantine/core'
import {
  IconArrowLeft,
  IconBooks,
  IconCopy,
  IconFolderPlus,
  IconLock,
  IconMessageCircle,
  IconPencil,
  IconShare,
  IconTrash,
} from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { FolderTile, ShareBadge } from '@/components'
import { canWriteCategory, CategoryIcon } from '@/lib'
import type { Category } from '@/schemas'

const iconSm = { size: 16, stroke: 1.5 } as const

export type CategoryDetailActions = {
  onEdit: () => void
  onNewSubcategory: () => void
  onShare: () => void
  onDuplicate: () => void
  onMoveToVault: () => void
  onSocial: () => void
  onDelete: () => void
  duplicatePending: boolean
}

type Props = {
  category: Category
  ancestors: Category[]
  subcategories: Category[]
  childCounts: Map<string, number>
  onSelect: (id: string | null) => void
  /** Phones only: back to the tree. */
  onBack?: () => void
  actions: CategoryDetailActions
}

const VISIBILITY_LABEL: Record<Category['visibility'], string> = {
  private: 'Private',
  followers: 'Followers',
  members: 'Members',
}

function countLabel(n: number): string {
  return n === 0 ? 'No subcategories' : `${n} subcategor${n === 1 ? 'y' : 'ies'}`
}

/** One category: where it sits, what it is, everything you can do with it, and its subcategories. */
export function CategoryDetail({
  category,
  ancestors,
  subcategories,
  childCounts,
  onSelect,
  onBack,
  actions,
}: Props): ReactElement {
  const isOwner = category.access === 'owner'
  const canWrite = canWriteCategory(category)
  const shared = category.access === 'read' || category.access === 'write' ? category.access : null
  const socialEnabled = category.shared_out || !!shared

  return (
    <Stack gap="lg">
      {onBack ? (
        <Button
          variant="subtle"
          size="compact-sm"
          leftSection={<IconArrowLeft {...iconSm} />}
          onClick={onBack}
          style={{ alignSelf: 'flex-start' }}
        >
          All categories
        </Button>
      ) : null}

      <Paper withBorder p="xl">
        <Stack gap="md">
          <Breadcrumbs separator="/" separatorMargin={6}>
            <Anchor component="button" type="button" size="sm" onClick={() => onSelect(null)}>
              Categories
            </Anchor>
            {ancestors.map((a) => (
              <Anchor key={a.id} component="button" type="button" size="sm" onClick={() => onSelect(a.id)}>
                {a.name}
              </Anchor>
            ))}
          </Breadcrumbs>

          <Group gap="md" wrap="nowrap" align="flex-start">
            <CategoryIcon
              icon={category.icon}
              size={44}
              stroke={1.5}
              color="var(--mantine-color-violet-filled)"
              aria-hidden
            />
            <Stack gap={6} miw={0}>
              <Title order={3} style={{ wordBreak: 'break-word' }}>
                {category.name}
              </Title>
              <Group gap={6}>
                <Badge size="sm" variant="light" color="gray" tt="none">
                  {VISIBILITY_LABEL[category.visibility]}
                </Badge>
                {shared ? <ShareBadge access={shared} ownerLabel={category.owner_display_name} size="sm" /> : null}
                {isOwner && category.shared_out ? (
                  <ShareBadge outgoing sharedWith={category.shared_with} size="sm" />
                ) : null}
                {category.is_vault ? (
                  <Badge size="sm" variant="light" color="grape" leftSection={<IconLock size={12} stroke={1.5} />}>
                    Vault
                  </Badge>
                ) : null}
                {category.is_hidden ? (
                  <Badge size="sm" variant="outline" color="gray">
                    Hidden
                  </Badge>
                ) : null}
                {category.deleted_at ? (
                  <Badge size="sm" variant="outline" color="red" tt="none">
                    Pending delete
                  </Badge>
                ) : null}
              </Group>
              <Text size="sm" c="dimmed">
                {countLabel(subcategories.length)}
                {category.last_bookmark_at
                  ? ` · last bookmark ${new Date(category.last_bookmark_at).toLocaleDateString()}`
                  : ''}
              </Text>
            </Stack>
          </Group>

          <Group gap="xs">
            <Button
              renderRoot={(props) => <Link {...props} to="/library" search={{ folder: category.id }} />}
              variant="light"
              leftSection={<IconBooks {...iconSm} />}
            >
              Open in library
            </Button>
            {canWrite ? (
              <Button variant="default" leftSection={<IconPencil {...iconSm} />} onClick={actions.onEdit}>
                Edit
              </Button>
            ) : null}
            {canWrite ? (
              <Button variant="default" leftSection={<IconFolderPlus {...iconSm} />} onClick={actions.onNewSubcategory}>
                New subcategory
              </Button>
            ) : null}
            {isOwner && !category.is_vault ? (
              <Button variant="default" leftSection={<IconShare {...iconSm} />} onClick={actions.onShare}>
                Share
              </Button>
            ) : null}
            {!category.is_vault ? (
              <Button
                variant="default"
                leftSection={<IconCopy {...iconSm} />}
                loading={actions.duplicatePending}
                onClick={actions.onDuplicate}
              >
                Duplicate
              </Button>
            ) : null}
            {isOwner && !category.is_vault ? (
              <Button variant="default" leftSection={<IconLock {...iconSm} />} onClick={actions.onMoveToVault}>
                Move to vault
              </Button>
            ) : null}
            {socialEnabled ? (
              <Button variant="default" leftSection={<IconMessageCircle {...iconSm} />} onClick={actions.onSocial}>
                Likes & comments
              </Button>
            ) : null}
            {isOwner ? (
              <Button variant="subtle" color="red" leftSection={<IconTrash {...iconSm} />} onClick={actions.onDelete}>
                Delete
              </Button>
            ) : null}
          </Group>
        </Stack>
      </Paper>

      {subcategories.length > 0 ? (
        <Stack gap="sm">
          <Text size="xs" fw={600} c="dimmed" tt="uppercase">
            Subcategories
          </Text>
          <SimpleGrid cols={{ base: 2, sm: 3, lg: 4 }} spacing="md">
            {subcategories.map((c) => (
              <FolderTile
                key={c.id}
                id={c.id}
                name={c.name}
                countLabel={countLabel(childCounts.get(c.id) ?? 0)}
                onOpen={onSelect}
                isVault={c.is_vault}
                isHidden={c.is_hidden}
                icon={c.icon}
                access={c.access}
                sharedOut={c.shared_out}
                sharedWith={c.shared_with}
                ownerDisplayName={c.owner_display_name}
                deletedAt={c.deleted_at}
              />
            ))}
          </SimpleGrid>
        </Stack>
      ) : null}
    </Stack>
  )
}
