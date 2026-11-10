// Screenshot stub: stands in for the browser APIs and the Linkdean API with sample data.
const s = new URLSearchParams(location.search).get('s') || 'save'
const page = {
  id: 1,
  url: 'https://fieldnotes.example/designing-calm-interfaces',
  title: 'Designing calm interfaces · Field Notes',
}
const store = { lastFolder: 'c3' }
if (s !== 'signin')
  store.session = { instance: 'https://app.linkdean.app', token: 'x', keyId: 'k', email: 'sam@example.com' }
const ev = { addListener() {}, removeListener() {}, hasListener: () => false }
const area = {
  get: async (k) => {
    const keys = k == null ? Object.keys(store) : Array.isArray(k) ? k : typeof k === 'string' ? [k] : Object.keys(k)
    const o = {}
    for (const x of keys) if (x in store) o[x] = store[x]
    return o
  },
  set: async (o) => void Object.assign(store, o),
  remove: async (k) => void [].concat(k).forEach((x) => delete store[x]),
  onChanged: ev,
}
globalThis.chrome = {
  runtime: { id: 'linkdean', getURL: (p) => p, sendMessage: async () => ({ ok: true }), onMessage: ev },
  storage: { local: area, session: area, sync: area, onChanged: ev },
  tabs: { query: async () => [page] },
  scripting: {
    executeScript: async () => [
      { result: 'Calm software asks for attention only when it matters, and gives it back the moment it is done.' },
    ],
  },
  permissions: { request: async () => true },
}
window.close = () => {}
const json = (b) => new Response(JSON.stringify(b), { headers: { 'content-type': 'application/json' } })
window.fetch = async (input) => {
  const u = new URL(String(input))
  if (u.pathname.endsWith('/bookmarks/lookup'))
    return json({ bookmark: s === 'saved' ? { id: 'b1', url: page.url, title: 'Designing calm interfaces' } : null })
  if (u.pathname.endsWith('/categories'))
    return json([
      { id: 'c1', name: 'Reading list', is_vault: false },
      { id: 'c2', name: 'Design', is_vault: false },
      { id: 'c3', name: 'Interfaces', parent_id: 'c2', is_vault: false },
      { id: 'c4', name: 'Work', is_vault: false },
    ])
  if (u.pathname.endsWith('/tags')) return json([{ name: 'design' }, { name: 'ux' }, { name: 'reading' }])
  return json({})
}
