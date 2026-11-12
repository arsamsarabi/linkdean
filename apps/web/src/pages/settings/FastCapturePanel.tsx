import { useEffect, useRef, type ReactElement } from 'react'
import { Anchor, Button, Code, CopyButton, Group, List, Paper, Stack, Text, Title } from '@mantine/core'
import { IconBookmarkPlus, IconBrandChrome } from '@tabler/icons-react'
import { bookmarkletSource } from '@/lib'

const DOCS = 'https://docs.linkdean.app'
/** The extension's Chrome Web Store listing; Edge installs from it too. */
const CHROME_STORE_URL = 'https://chromewebstore.google.com/detail/ielbdodidimpcolkjhkjdaifpfibolhf'

/** Settings → Fast capture: the browser extension, and the bookmarklet that saves the page you are on. */
export function FastCapturePanel(): ReactElement {
  const source = bookmarkletSource(window.location.origin)
  const link = useRef<HTMLAnchorElement>(null)

  // React refuses `javascript:` in href, so the bookmarklet is set on the element directly.
  useEffect(() => {
    link.current?.setAttribute('href', source)
  }, [source])

  return (
    <Stack gap="lg" maw={640}>
      <Paper p="xl" withBorder>
        <Stack gap="md">
          <Stack gap={4}>
            <Title order={4}>Browser extension</Title>
            <Text size="sm" c="dimmed">
              Save the page you are on from the toolbar, the right-click menu or Alt+Shift+S, and see at a glance which
              pages you have already saved. Works in Chrome and Edge; the Firefox add-on is in review.
            </Text>
          </Stack>
          <Group>
            <Button
              component="a"
              href={CHROME_STORE_URL}
              target="_blank"
              rel="noreferrer"
              leftSection={<IconBrandChrome size={16} />}
            >
              Add to Chrome
            </Button>
            <Anchor
              href={`${DOCS}/docs/guides/fast-capture#browser-extension-chrome-edge-firefox`}
              target="_blank"
              rel="noreferrer"
              size="sm"
            >
              How it works
            </Anchor>
          </Group>
        </Stack>
      </Paper>
      <Paper p="xl" withBorder>
        <Stack gap="md">
          <Stack gap={4}>
            <Title order={4}>Save from any page</Title>
            <Text size="sm" c="dimmed">
              A bookmarklet is a bookmark that runs a small script. Click it on any page to open Linkdean in a small
              window with the page&apos;s address and title filled in. Text you have selected becomes the description.
            </Text>
          </Stack>
          <Group>
            <Button
              component="a"
              ref={link}
              draggable
              leftSection={<IconBookmarkPlus size={16} />}
              onClick={(e) => e.preventDefault()}
              title="Drag me to your bookmarks bar"
            >
              Save to Linkdean
            </Button>
            <CopyButton value={source} timeout={1500}>
              {({ copied, copy }) => (
                <Button variant="default" onClick={copy}>
                  {copied ? 'Copied' : 'Copy code'}
                </Button>
              )}
            </CopyButton>
          </Group>
          <List size="sm" spacing={4}>
            <List.Item>
              <b>Chrome, Edge, Firefox:</b> show the bookmarks bar, then drag <b>Save to Linkdean</b> onto it.
            </List.Item>
            <List.Item>
              <b>Safari, or a phone:</b> bookmark any page, edit that bookmark, and replace its address with the copied
              code.
            </List.Item>
          </List>
          <Text size="xs" c="dimmed">
            It uses the session you are signed in with here, so it needs no API key. If you are signed out, sign in in
            the window it opens and the save carries on. A page that blocks popups opens Linkdean in the same tab
            instead. The code points at <Code>{window.location.origin}</Code>.
          </Text>
        </Stack>
      </Paper>
      <Paper p="xl" withBorder>
        <Stack gap="sm">
          <Title order={4}>Share from your phone</Title>
          <Text size="sm">
            <b>Android:</b> install Linkdean from Chrome (<b>⋮ → Add to Home screen</b>). It then appears in the share
            sheet: share a link to <b>Linkdean</b> to save it. Desktop Chrome and Edge do the same once installed.
          </Text>
          <Text size="sm">
            <b>iPhone and iPad:</b> iOS does not let web apps receive shares, so use a Shortcut instead.{' '}
            <Anchor
              href={`${DOCS}/docs/guides/fast-capture#iphone-and-ipad-a-shortcut`}
              target="_blank"
              rel="noreferrer"
            >
              How to set it up
            </Anchor>
          </Text>
        </Stack>
      </Paper>
    </Stack>
  )
}
