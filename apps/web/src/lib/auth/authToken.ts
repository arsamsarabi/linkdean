export const tokenStorageKey = 'linkdean_token'

export function getToken(): string | null {
  return localStorage.getItem(tokenStorageKey)
}

export function setToken(token: string | null): void {
  if (token) localStorage.setItem(tokenStorageKey, token)
  else localStorage.removeItem(tokenStorageKey)
}
