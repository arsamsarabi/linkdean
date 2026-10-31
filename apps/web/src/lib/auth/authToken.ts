const tokenKey = 'linkdean_token'

export function getToken(): string | null {
  return localStorage.getItem(tokenKey)
}

export function setToken(token: string | null): void {
  if (token) localStorage.setItem(tokenKey, token)
  else localStorage.removeItem(tokenKey)
}
