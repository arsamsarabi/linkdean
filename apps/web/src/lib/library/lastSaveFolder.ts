/** The folder last picked when saving through quick save, offered first next time. */
const lastSaveFolderKey = 'linkdean_last_save_folder'

export function lastSaveFolder(): string | null {
  try {
    return localStorage.getItem(lastSaveFolderKey) || null
  } catch {
    return null
  }
}

export function rememberSaveFolder(categoryId: string | null): void {
  try {
    if (categoryId) localStorage.setItem(lastSaveFolderKey, categoryId)
    else localStorage.removeItem(lastSaveFolderKey)
  } catch {
    // Storage blocked: the next save starts with no folder.
  }
}
