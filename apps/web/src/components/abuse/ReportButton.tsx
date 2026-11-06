import { useState, type ReactElement } from 'react'
import { Button, Tooltip } from '@mantine/core'
import { IconFlag } from '@tabler/icons-react'
import type { AbuseTargetType } from '@/schemas'
import { ReportDrawer } from './ReportDrawer'

type Props = {
  targetType: AbuseTargetType
  targetId: string
  subject: string
}

/** Opens the report drawer. The caller only shows it to signed-in users. */
export function ReportButton({ targetType, targetId, subject }: Props): ReactElement {
  const [opened, setOpened] = useState(false)
  return (
    <>
      <Tooltip label="Report abuse">
        <Button
          variant="subtle"
          color="gray"
          size="compact-sm"
          leftSection={<IconFlag size={14} stroke={1.5} />}
          onClick={() => setOpened(true)}
        >
          Report
        </Button>
      </Tooltip>
      <ReportDrawer
        opened={opened}
        onClose={() => setOpened(false)}
        targetType={targetType}
        targetId={targetId}
        subject={subject}
      />
    </>
  )
}
