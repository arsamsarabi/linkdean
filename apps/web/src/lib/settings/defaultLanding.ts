export type DefaultLanding = '/' | '/library'

/** Map stored setting (incl. legacy values) to a real app route. */
export function resolveDefaultLanding(value: string | null | undefined): DefaultLanding {
  return value === '/library' ? '/library' : '/'
}
