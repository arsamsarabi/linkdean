import { useState, type ReactElement } from 'react'
import {
  Box,
  Button,
  Checkbox,
  Drawer,
  Group,
  Loader,
  LoadingOverlay,
  MultiSelect,
  Select,
  Stack,
  Switch,
  Tabs,
  TagsInput,
  Text,
  Textarea,
  TextInput,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconWorldDownload } from '@tabler/icons-react'
import { SocialThread } from '@/components/social'
import {
  useCreateBookmark,
  useDeleteBookmarkPreview,
  useMe,
  usePageDetails,
  useSyncDuringRender,
  useUpdateBookmark,
  useUploadBookmarkPreview,
  useVaults,
} from '@/hooks'
import { useDrawerClosesOnEscape, useDrawerSize } from '@/hooks/render'
import { categorySelectData, withWebScheme, writableCategories } from '@/lib'
import type { Category } from '@/schemas'
import { draftHeadsToVault, emptyDraft, fillFromPage, isFetchableUrl, type BookmarkDraft } from '@/services'
import { BookmarkPreviewField } from './BookmarkPreviewField'

export type { BookmarkDraft }

type Props = {
  opened: boolean
  /** Vaults page: a new bookmark must land in a vault. */
  vaultRequired?: boolean
  onClose: () => void
  categories: Category[]
  initial: BookmarkDraft | null
  /** A shared bookmark: add a Discussion tab with its likes, rating and comments. */
  discussion?: boolean
}

export function BookmarkDrawer({
  opened,
  onClose,
  categories,
  initial,
  vaultRequired,
  discussion = false,
}: Props): ReactElement {
  const closeOnEscape = useDrawerClosesOnEscape()
  const drawerSize = useDrawerSize()
  const [draft, setDraft] = useState<BookmarkDraft>(emptyDraft())
  const [pendingPreview, setPendingPreview] = useState<Blob | null>(null)
  const [tab, setTab] = useState<string | null>('details')
  const editing = !!draft.id
  const vaultMissing = !!vaultRequired && !editing && !draft.vault_id && draft.category_ids.length === 0
  const vaults = useVaults(opened)
  const me = useMe(true)
  const canSetPrivate = editing && !draft.is_vault && draft.owner_user_id === me.data?.user.id
  const createBookmark = useCreateBookmark()
  const updateBookmark = useUpdateBookmark()
  const uploadPreview = useUploadBookmarkPreview()
  const deletePreview = useDeleteBookmarkPreview()
  const pageDetails = usePageDetails()
  // The result of the last Import from URL: a note for an empty page, or why it failed.
  const [fetchNote, setFetchNote] = useState<{ error: boolean; text: string } | null>(null)
  const pending =
    createBookmark.isPending || updateBookmark.isPending || uploadPreview.isPending || deletePreview.isPending
  const fetchUrl = withWebScheme(draft.url)
  const canFetch = isFetchableUrl(fetchUrl)
  // Vault URLs never leave the instance, so the button is not offered for them.
  const showFetch = !draftHeadsToVault(draft, categories, vaultRequired) && draft.can_write !== false
  const fetching = pageDetails.isPending

  useSyncDuringRender([opened, initial], () => {
    if (!opened) return
    setDraft(initial ? { ...initial, tags: [...initial.tags] } : emptyDraft())
    setPendingPreview(null)
    setTab('details')
    setFetchNote(null)
  })

  async function importFromUrl(): Promise<void> {
    const url = fetchUrl
    setFetchNote(null)
    try {
      const details = await pageDetails.mutateAsync(url)
      // Typing goes on while the page loads: fill only what is still empty, and only for
      // the address that was fetched.
      setDraft((d) => (withWebScheme(d.url) === url ? fillFromPage(d, details) : d))
      if (!details.title && !details.description && details.tags.length === 0) {
        setFetchNote({ error: false, text: 'The page did not share a title, description or tags.' })
      }
    } catch (e) {
      setFetchNote({ error: true, text: e instanceof Error ? e.message : String(e) })
    }
  }

  async function save(): Promise<void> {
    const tags = draft.tags.map((t) => t.trim()).filter(Boolean)
    const body = {
      url: withWebScheme(draft.url),
      title: draft.title.trim() || undefined,
      description: draft.description.trim() || undefined,
      category_ids: draft.category_ids,
      tags,
    }
    try {
      if (draft.id) {
        await updateBookmark.mutateAsync({
          id: draft.id,
          body: {
            url: body.url,
            title: body.title || '',
            description: body.description ?? null,
            category_ids: body.category_ids,
            tags: body.tags,
            is_favorite: draft.is_favorite,
            is_unread: draft.is_unread,
            is_pinned: draft.is_pinned,
            ...(canSetPrivate ? { is_private: draft.is_private } : {}),
          },
        })
        if (pendingPreview) {
          await uploadPreview.mutateAsync({ id: draft.id, blob: pendingPreview })
        }
      } else {
        const created = await createBookmark.mutateAsync({
          url: body.url,
          title: body.title,
          description: body.description,
          category_ids: body.category_ids,
          vault_id: body.category_ids.length === 0 ? (draft.vault_id ?? null) : null,
          tags: body.tags,
          is_favorite: draft.is_favorite,
          is_unread: draft.is_unread,
          is_pinned: draft.is_pinned,
        })
        if (pendingPreview && !created.is_vault) {
          try {
            await uploadPreview.mutateAsync({ id: created.id, blob: pendingPreview })
          } catch (e) {
            notifications.show({
              color: 'yellow',
              message: `Bookmark saved; preview upload failed: ${e}`,
            })
            onClose()
            return
          }
        }
      }
      notifications.show({ message: editing ? 'Updated' : 'Saved' })
      onClose()
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  const previewAllowed = !draft.is_vault && (draft.can_write !== false || !editing)

  const showDiscussion = discussion && editing
  const form = (
    <Box pos="relative">
      <LoadingOverlay visible={pending} zIndex={100} overlayProps={{ blur: 1 }} />
      <Stack gap="md">
        <TextInput
          label="URL"
          required
          placeholder="https://"
          value={draft.url}
          onChange={(e) => {
            const url = e.currentTarget.value
            setDraft((d) => ({ ...d, url }))
            setFetchNote(null)
          }}
        />
        {showFetch ? (
          <Stack gap={4} mt={-8}>
            <Group>
              <Button
                variant="light"
                size="xs"
                leftSection={<IconWorldDownload size={14} />}
                onClick={() => void importFromUrl()}
                disabled={!canFetch || pending}
                loading={fetching}
              >
                Import from URL
              </Button>
            </Group>
            {fetchNote ? (
              <Text size="xs" c={fetchNote.error ? 'red' : 'dimmed'} role={fetchNote.error ? 'alert' : undefined}>
                {fetchNote.text}
              </Text>
            ) : null}
          </Stack>
        ) : null}
        <TextInput
          label="Title"
          rightSection={fetching && !draft.title.trim() ? <Loader size="xs" /> : null}
          value={draft.title}
          onChange={(e) => {
            const title = e.currentTarget.value
            setDraft((d) => ({ ...d, title }))
          }}
        />
        <Textarea
          label="Description"
          rightSection={fetching && !draft.description.trim() ? <Loader size="xs" /> : null}
          minRows={2}
          autosize
          value={draft.description}
          onChange={(e) => {
            const description = e.currentTarget.value
            setDraft((d) => ({ ...d, description }))
          }}
        />
        {canSetPrivate ? (
          <Switch
            label="Only me"
            description="Keep this off your profile, even in a visible category."
            checked={draft.is_private}
            onChange={(e) => {
              const is_private = e.currentTarget.checked
              setDraft((d) => ({ ...d, is_private }))
            }}
          />
        ) : null}
        {previewAllowed ? (
          <BookmarkPreviewField
            bookmarkId={draft.id}
            hasPreview={draft.has_preview}
            previewImageUrl={draft.preview_image_url}
            updatedAt={draft.updated_at}
            pendingBlob={pendingPreview}
            onPendingChange={setPendingPreview}
            onUploaded={(next) =>
              setDraft((d) => ({
                ...d,
                has_preview: next.has_preview,
                preview_image_url: next.preview_image_url,
                updated_at: next.updated_at,
              }))
            }
            onUpload={async (id, blob) => uploadPreview.mutateAsync({ id, blob })}
            onDelete={async (id) => deletePreview.mutateAsync(id)}
            disabled={pending}
          />
        ) : null}
        <MultiSelect
          label="Categories"
          description="Can pick several. Nested path shown in label. Read-only shared folders are omitted."
          data={categorySelectData(writableCategories(categories))}
          value={draft.category_ids}
          onChange={(category_ids) => setDraft((d) => ({ ...d, category_ids }))}
          searchable
          clearable
        />
        {!editing && draft.category_ids.length === 0 ? (
          <Select
            label="Vault"
            placeholder={vaultRequired ? 'Choose a vault' : 'Not in a vault'}
            required={vaultRequired}
            clearable={!vaultRequired}
            data={(vaults.data?.vaults ?? []).filter((v) => v.unlocked).map((v) => ({ value: v.id, label: v.name }))}
            value={draft.vault_id ?? null}
            onChange={(vault_id) => setDraft((d) => ({ ...d, vault_id }))}
          />
        ) : null}
        <TagsInput
          label="Tags"
          description="Type a tag and press Enter"
          value={draft.tags}
          onChange={(tags) => setDraft((d) => ({ ...d, tags }))}
          clearable
        />
        <Checkbox
          label="Reading list (unread)"
          checked={draft.is_unread}
          onChange={(e) => {
            const is_unread = e.currentTarget.checked
            setDraft((d) => ({ ...d, is_unread }))
          }}
        />
        <Checkbox
          label="Favorite"
          checked={draft.is_favorite}
          onChange={(e) => {
            const is_favorite = e.currentTarget.checked
            setDraft((d) => ({ ...d, is_favorite }))
          }}
        />
        <Checkbox
          label="Pin to top of library"
          checked={draft.is_pinned}
          onChange={(e) => {
            const is_pinned = e.currentTarget.checked
            setDraft((d) => ({ ...d, is_pinned }))
          }}
        />
        <Button mt="sm" onClick={() => void save()} disabled={!draft.url.trim() || vaultMissing} loading={pending}>
          {editing ? 'Save changes' : 'Add bookmark'}
        </Button>
      </Stack>
    </Box>
  )

  return (
    <Drawer
      closeOnEscape={closeOnEscape}
      opened={opened}
      onClose={onClose}
      title={editing ? 'Edit bookmark' : 'Add bookmark'}
      position="right"
      size={drawerSize}
      padding="lg"
    >
      {showDiscussion && draft.id ? (
        <Tabs value={tab} onChange={setTab}>
          <Tabs.List mb="md">
            <Tabs.Tab value="details">Details</Tabs.Tab>
            <Tabs.Tab value="discussion">Discussion</Tabs.Tab>
          </Tabs.List>
          <Tabs.Panel value="details">{form}</Tabs.Panel>
          <Tabs.Panel value="discussion">
            <SocialThread subjectType="bookmark" subjectId={draft.id} enabled={opened && tab === 'discussion'} />
          </Tabs.Panel>
        </Tabs>
      ) : (
        form
      )}
    </Drawer>
  )
}
