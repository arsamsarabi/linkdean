import type { ReactElement } from 'react'
import { ActionIcon, Group, Stack, Text, Tooltip, UnstyledButton } from '@mantine/core'
import { IconFolderFilled, IconX } from '@tabler/icons-react'
import { CATEGORY_ICON_OPTIONS, type CategoryIconKey } from '@/lib'
import styles from './CategoryIconPicker.module.css'

type Props = {
  value: CategoryIconKey | null
  onChange: (next: CategoryIconKey | null) => void
  label?: string
}

export function CategoryIconPicker({
  value,
  onChange,
  label = 'Icon',
}: Props): ReactElement {
  return (
    <Stack gap="xs">
      <Group justify="space-between" align="center">
        <Text size="sm" fw={500}>
          {label}
        </Text>
        {value ? (
          <ActionIcon
            variant="subtle"
            color="gray"
            size="sm"
            aria-label="Clear icon"
            onClick={() => onChange(null)}
          >
            <IconX size={14} stroke={1.5} />
          </ActionIcon>
        ) : null}
      </Group>
      <div className={styles.grid} role="listbox" aria-label={label}>
        <Tooltip label="Default folder">
          <UnstyledButton
            type="button"
            className={`${styles.cell}${value == null ? ` ${styles.selected}` : ''}`}
            aria-selected={value == null}
            onClick={() => onChange(null)}
          >
            <IconFolderFilled size={20} stroke={1.5} aria-hidden />
          </UnstyledButton>
        </Tooltip>
        {CATEGORY_ICON_OPTIONS.map(({ key, label: tip, Icon }) => (
          <Tooltip key={key} label={tip}>
            <UnstyledButton
              type="button"
              className={`${styles.cell}${value === key ? ` ${styles.selected}` : ''}`}
              aria-label={tip}
              aria-selected={value === key}
              onClick={() => onChange(key)}
            >
              <Icon size={20} stroke={1.5} aria-hidden />
            </UnstyledButton>
          </Tooltip>
        ))}
      </div>
    </Stack>
  )
}
