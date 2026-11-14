import { useEffect, useRef, useState, type ReactElement } from 'react'
import { ActionIcon, Button, Loader, Text, Tooltip } from '@mantine/core'
import { useHotkeys } from '@mantine/hooks'
import {
  IconArrowsMaximize,
  IconArrowsMinimize,
  IconExternalLink,
  IconMinus,
  IconPictureInPicture,
  IconWindowMaximize,
  IconX,
} from '@tabler/icons-react'
import { usePeekUi, type PeekMode, type PeekTarget } from '@/stores'
import styles from './PeekPanel.module.css'

/** No allow-same-origin / allow-top-navigation: peeked pages cannot touch the session. */
export const PEEK_IFRAME_SANDBOX = 'allow-scripts allow-forms allow-popups allow-popups-to-escape-sandbox'

const EMBED_TIMEOUT_MS = 4000

export function PeekPanel(): ReactElement | null {
  const target = usePeekUi((s) => s.target)
  const mode = usePeekUi((s) => s.mode)
  const close = usePeekUi((s) => s.close)

  useHotkeys([['Escape', () => close()]])

  if (!target) return null

  return <PeekPanelChrome key={`${target.bookmarkId}:${target.url}`} target={target} mode={mode} />
}

type ChromeProps = {
  target: PeekTarget
  mode: PeekMode
}

function PeekPanelChrome({ target, mode }: ChromeProps): ReactElement {
  const close = usePeekUi((s) => s.close)
  const maximize = usePeekUi((s) => s.maximize)
  const minimize = usePeekUi((s) => s.minimize)
  const restore = usePeekUi((s) => s.restore)
  const iframeRef = useRef<HTMLIFrameElement>(null)
  const embedTimerRef = useRef<number | null>(null)
  const [frameBlocked, setFrameBlocked] = useState(false)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    embedTimerRef.current = window.setTimeout(() => {
      embedTimerRef.current = null
      setLoading(false)
      setFrameBlocked(true)
    }, EMBED_TIMEOUT_MS)
    return () => {
      if (embedTimerRef.current != null) {
        window.clearTimeout(embedTimerRef.current)
        embedTimerRef.current = null
      }
    }
  }, [])

  function clearEmbedTimer(): void {
    if (embedTimerRef.current != null) {
      window.clearTimeout(embedTimerRef.current)
      embedTimerRef.current = null
    }
  }

  function onFrameLoad(): void {
    const frame = iframeRef.current
    if (!frame) return
    try {
      const href = frame.contentWindow?.location.href
      // about:blank after a load usually means the browser refused the embed.
      if (!href || href === 'about:blank') {
        clearEmbedTimer()
        setFrameBlocked(true)
        setLoading(false)
        return
      }
    } catch {
      // Cross-origin content loaded: embed succeeded.
    }
    clearEmbedTimer()
    setFrameBlocked(false)
    setLoading(false)
  }

  function openInNewTab(): void {
    window.open(target.url, '_blank', 'noopener,noreferrer')
  }

  function detach(): void {
    const w = Math.min(1100, window.screen.availWidth)
    const h = Math.min(800, window.screen.availHeight)
    window.open(target.url, '_blank', `noopener,noreferrer,width=${w},height=${h}`)
    close()
  }

  const panelClass =
    mode === 'maximized' ? styles.panelMaximized : mode === 'minimized' ? styles.panelMinimized : styles.panelOpen

  const title = target.title.trim() || 'Untitled'

  return (
    <div
      className={`${styles.panel} ${panelClass}`}
      role="dialog"
      aria-modal={mode === 'maximized'}
      aria-label={`Peek: ${title}`}
    >
      <div className={styles.header}>
        <Text className={styles.title} lineClamp={1} title={title}>
          {title}
        </Text>
        <div className={styles.controls}>
          {mode === 'minimized' ? (
            <Tooltip label="Restore">
              <ActionIcon variant="subtle" color="gray" aria-label="Restore peek" onClick={restore}>
                <IconWindowMaximize size={16} stroke={1.5} />
              </ActionIcon>
            </Tooltip>
          ) : (
            <Tooltip label="Minimize">
              <ActionIcon variant="subtle" color="gray" aria-label="Minimize peek" onClick={minimize}>
                <IconMinus size={16} stroke={1.5} />
              </ActionIcon>
            </Tooltip>
          )}
          {mode === 'maximized' ? (
            <Tooltip label="Restore size">
              <ActionIcon variant="subtle" color="gray" aria-label="Restore peek size" onClick={restore}>
                <IconArrowsMinimize size={16} stroke={1.5} />
              </ActionIcon>
            </Tooltip>
          ) : (
            <Tooltip label="Maximize">
              <ActionIcon variant="subtle" color="gray" aria-label="Maximize peek" onClick={maximize}>
                <IconArrowsMaximize size={16} stroke={1.5} />
              </ActionIcon>
            </Tooltip>
          )}
          <Tooltip label="Detach">
            <ActionIcon variant="subtle" color="gray" aria-label="Detach peek" onClick={detach}>
              <IconPictureInPicture size={16} stroke={1.5} />
            </ActionIcon>
          </Tooltip>
          <Tooltip label="Open in new tab">
            <ActionIcon variant="subtle" color="gray" aria-label="Open in new tab" onClick={openInNewTab}>
              <IconExternalLink size={16} stroke={1.5} />
            </ActionIcon>
          </Tooltip>
          <Tooltip label="Close">
            <ActionIcon variant="subtle" color="gray" aria-label="Close peek" onClick={close}>
              <IconX size={16} stroke={1.5} />
            </ActionIcon>
          </Tooltip>
        </div>
      </div>

      {mode === 'minimized' ? null : (
        <div className={styles.body}>
          {frameBlocked ? (
            <div className={styles.fallback}>
              {target.previewUrl ? (
                <div className={styles.fallbackPreview}>
                  <img src={target.previewUrl} alt="" />
                </div>
              ) : null}
              <Text fw={600}>{title}</Text>
              {target.description ? (
                <Text size="sm" c="dimmed" lineClamp={4}>
                  {target.description}
                </Text>
              ) : (
                <Text size="sm" c="dimmed">
                  This site cannot be shown inside Linkdean.
                </Text>
              )}
              <Button
                component="a"
                href={target.url}
                target="_blank"
                rel="noreferrer"
                leftSection={<IconExternalLink size={14} stroke={1.5} />}
              >
                Open in new tab
              </Button>
            </div>
          ) : (
            <>
              <iframe
                ref={iframeRef}
                className={styles.frame}
                title={title}
                src={target.url}
                sandbox={PEEK_IFRAME_SANDBOX}
                referrerPolicy="no-referrer"
                onLoad={onFrameLoad}
              />
              {loading ? (
                <div className={styles.loading}>
                  <Loader type="dots" color="violet" />
                </div>
              ) : null}
            </>
          )}
        </div>
      )}
    </div>
  )
}
