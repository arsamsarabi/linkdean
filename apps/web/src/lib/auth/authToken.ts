export const tokenStorageKey = 'linkdean_token'

// Known issue: The session token sits in localStorage, which undoes the cookie's `HttpOnly` if a
// script injection is ever found. None is known today. Fix: rely on the cookie in the browser.
export function getToken(): string | null {
  return localStorage.getItem(tokenStorageKey)
}

export function setToken(token: string | null): void {
  if (token) localStorage.setItem(tokenStorageKey, token)
  else localStorage.removeItem(tokenStorageKey)
}
