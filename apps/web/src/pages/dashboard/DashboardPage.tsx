import { type ReactElement } from 'react'
import { Stack, Text } from '@mantine/core'
import { ConfirmModal, QueryErrorAlert, ShareDashboardDrawer, SocialDrawer } from '@/components'
import { useDashboardPage } from '@/hooks'
import { DashboardEditLayout } from './DashboardEditLayout'
import { DashboardPageHeader } from './DashboardPageHeader'
import { DashboardViewLayout } from './DashboardViewLayout'

export function DashboardPage(): ReactElement {
  const d = useDashboardPage()

  if (d.list.isError) {
    return <QueryErrorAlert error={d.list.error} onRetry={() => void d.list.refetch()} title="Dashboards failed" />
  }

  if (d.dashboard.isError) {
    return (
      <QueryErrorAlert error={d.dashboard.error} onRetry={() => void d.dashboard.refetch()} title="Dashboard failed" />
    )
  }

  if (d.list.isLoading || !d.activeId || d.dashboard.isLoading || !d.view) {
    return (
      <Text size="sm" c="dimmed">
        Loading dashboard…
      </Text>
    )
  }

  const view = d.view
  const draft = d.editDraft

  return (
    <Stack gap="lg">
      <DashboardPageHeader
        view={view}
        draft={draft}
        activeId={d.activeId}
        switcherData={d.switcherData}
        isOwner={d.isOwner}
        isPinned={d.isPinned}
        canEditLayout={!!d.canEditLayout}
        socialEnabled={d.socialEnabled}
        inviteeAccess={d.inviteeAccess}
        createPending={d.create.isPending}
        pinPending={d.patch.isPending}
        savePending={d.save.isPending}
        viewerName={d.viewerName}
        filter={d.filter}
        onFilterChange={d.setFilter}
        onDraftTitleChange={d.setDraftTitle}
        onSelectDashboard={d.selectDashboard}
        onCreate={() => void d.onCreate()}
        onOpenSocial={() => d.setSocialOpen(true)}
        onOpenShare={() => d.setShareOpen(true)}
        onPin={() => void d.onPin()}
        onOpenDelete={() => d.setDeleteOpen(true)}
        onCancelEdit={d.cancelEdit}
        onSave={() => void d.onSave()}
        onStartEdit={d.startEdit}
      />

      {draft ? (
        <DashboardEditLayout
          draft={draft}
          bookmarkOptions={d.bookmarkOptions}
          bookmarkById={d.bookmarkById}
          folderOptions={d.folderOptions}
          tagOptions={d.tagOptions}
          canAddSelfFilling={d.canAddSelfFilling}
          viewerUserId={d.viewerUserId}
          onUpdateTitle={d.updateSectionTitle}
          onUpdateBookmarks={d.updateSectionBookmarks}
          onUpdateSection={d.updateSection}
          onMoveSection={d.moveSectionBy}
          onRemoveSection={d.removeSection}
          onAddSection={d.addSection}
        />
      ) : (
        <DashboardViewLayout
          view={view}
          empty={d.empty}
          canEditLayout={!!d.canEditLayout}
          viewerUserId={d.viewerUserId}
          folderName={d.folderName}
          filter={d.filter}
          bookmarkOptions={d.bookmarkOptions}
          quickPinPending={d.save.isPending}
          onQuickPin={d.quickPin}
          onStartEdit={d.startEdit}
        />
      )}

      <ConfirmModal
        opened={d.deleteOpen}
        onClose={() => d.setDeleteOpen(false)}
        title="Delete dashboard?"
        body={`Delete “${view.title}”? Layout cannot be undone.`}
        confirmLabel="Delete"
        confirmColor="red"
        loading={d.remove.isPending}
        onConfirm={() => void d.onDelete()}
      />

      <ShareDashboardDrawer
        dashboardId={d.isOwner ? d.activeId || null : null}
        dashboardTitle={view.title}
        opened={d.shareOpen}
        onClose={() => d.setShareOpen(false)}
        selfFillingSections={d.selfFillingSections}
      />
      {d.socialEnabled ? (
        <SocialDrawer
          opened={d.socialOpen}
          onClose={() => d.setSocialOpen(false)}
          subjectType="dashboard"
          subjectId={d.activeId}
          title={view.title}
        />
      ) : null}
    </Stack>
  )
}
