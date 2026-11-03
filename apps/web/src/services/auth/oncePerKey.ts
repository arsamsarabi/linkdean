const runs = new Map<string, Promise<unknown>>()

/**
 * Runs a one-time link action once per key and shares the result, so a StrictMode remount or a
 * second render cannot spend the same token twice.
 */
export function oncePerKey<T>(key: string, run: () => Promise<T>): Promise<T> {
  let p = runs.get(key) as Promise<T> | undefined
  if (!p) {
    p = run()
    runs.set(key, p)
  }
  return p
}
