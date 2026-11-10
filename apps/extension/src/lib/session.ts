import { storage } from 'wxt/utils/storage'

/** Who the extension is signed in as. The token is an API key the person can revoke. */
export type Session = {
  instance: string
  token: string
  keyId: string
  email: string
}

export const sessionItem = storage.defineItem<Session | null>('local:session', { fallback: null })
/** The last instance signed in to, offered again after signing out. */
export const lastInstanceItem = storage.defineItem<string | null>('local:lastInstance', { fallback: null })
/** The folder picked on the last save, offered first next time. */
export const lastFolderItem = storage.defineItem<string | null>('local:lastFolder', { fallback: null })
