import { useEffect, useState, type ReactElement } from 'react'
import { Alert, Button, Group, Loader, Select, Stack, TagsInput, Text, Textarea, TextInput } from '@mantine/core'
import { browser } from 'wxt/browser'
import {
  categories,
  createBookmark,
  folderOptions,
  tags as listTags,
  lookup,
  savableUrl,
  SignedOutError,
  type Bookmark,
} from '@/lib/api'
import { rememberSaved } from '@/lib/savedPages'
import { lastFolderItem, type Session } from '@/lib/session'

type Page = { url: string; title: string; selection: string }

/** The current tab's address, title and selected text. Needs the activeTab grant of the click. */
async function currentPage(): Promise<Page | null> {
  const [tab] = await browser.tabs.query({ active: true, currentWindow: true })
  if (!tab || !savableUrl(tab.url)) return null
  let selection = ''
  if (tab.id !== undefined) {
    try {
      const [result] = await browser.scripting.executeScript({
        target: { tabId: tab.id },
        func: () =>
          String(window.getSelection?.() ?? '')
            .trim()
            .slice(0, 1000),
      })
      selection = typeof result?.result === 'string' ? result.result : ''
    } catch {
      // Pages such as the browser's own or the web store refuse scripts: no selection then.
    }
  }
  return { url: tab.url, title: tab.title ?? '', selection }
}

export function SaveView({ session }: { session: Session }): ReactElement {
  const [page, setPage] = useState<Page | null | undefined>(undefined)
  const [existing, setExisting] = useState<Bookmark | null>(null)
  const [folders, setFolders] = useState<{ value: string; label: string }[]>([])
  const [tagNames, setTagNames] = useState<string[]>([])
  const [title, setTitle] = useState('')
  const [description, setDescription] = useState('')
  const [folder, setFolder] = useState<string | null>(null)
  const [tags, setTags] = useState<string[]>([])
  const [busy, setBusy] = useState(false)
  const [saved, setSaved] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let live = true
    void (async () => {
      const p = await currentPage()
      if (!live) return
      setPage(p)
      if (!p) return
      setTitle(p.title)
      setDescription(p.selection)
      try {
        const [found, cats, names, last] = await Promise.all([
          lookup(session, p.url),
          categories(session),
          listTags(session),
          lastFolderItem.getValue(),
        ])
        if (!live) return
        const options = folderOptions(cats)
        setExisting(found)
        setFolders(options)
        setTagNames(names)
        setFolder(last && options.some((o) => o.value === last) ? last : null)
      } catch (e) {
        if (live && !(e instanceof SignedOutError)) setError(e instanceof Error ? e.message : String(e))
      }
    })()
    return () => {
      live = false
    }
  }, [session])

  async function save(unsorted: boolean): Promise<void> {
    if (!page) return
    setBusy(true)
    setError(null)
    try {
      await createBookmark(session, {
        url: page.url,
        title: title.trim() || undefined,
        description: description.trim() || undefined,
        category_ids: !unsorted && folder ? [folder] : [],
        tags: tags.map((t) => t.trim()).filter(Boolean),
      })
      if (!unsorted) await lastFolderItem.setValue(folder)
      await rememberSaved(page.url)
      setSaved(true)
      setTimeout(() => window.close(), 900)
    } catch (e) {
      if (!(e instanceof SignedOutError)) setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  const openUrl = page ? `${session.instance}/save?url=${encodeURIComponent(page.url)}` : session.instance

  if (page === undefined) {
    return (
      <Group justify="center" p="lg">
        <Loader size="sm" />
      </Group>
    )
  }
  if (page === null) {
    return <Text size="sm">Linkdean saves web pages (http and https). This tab cannot be saved.</Text>
  }
  if (saved) {
    return (
      <Text fw={500} ta="center" py="lg">
        Saved to Linkdean
      </Text>
    )
  }
  if (existing) {
    return (
      <Stack gap="sm">
        <Alert color="blue" title="Already in your library">
          <Text size="sm">{existing.title || page.title || page.url}</Text>
        </Alert>
        <Button component="a" href={openUrl} target="_blank">
          Open in Linkdean
        </Button>
      </Stack>
    )
  }
  return (
    <Stack gap="sm">
      <TextInput label="Title" value={title} onChange={(e) => setTitle(e.currentTarget.value)} />
      <Textarea
        label="Description"
        autosize
        minRows={2}
        maxRows={5}
        value={description}
        onChange={(e) => setDescription(e.currentTarget.value)}
      />
      <Select
        label="Folder"
        placeholder="Unsorted"
        data={folders}
        value={folder}
        onChange={setFolder}
        searchable
        clearable
        comboboxProps={{ withinPortal: false }}
      />
      <TagsInput
        label="Tags"
        data={tagNames}
        value={tags}
        onChange={setTags}
        clearable
        comboboxProps={{ withinPortal: false }}
      />
      {error ? (
        <Alert color="red" role="alert">
          {error}
        </Alert>
      ) : null}
      <Group grow>
        <Button onClick={() => void save(false)} loading={busy}>
          Save
        </Button>
        <Button variant="default" onClick={() => void save(true)} disabled={busy}>
          Save to Unsorted
        </Button>
      </Group>
    </Stack>
  )
}
