import { defineConfig } from 'wxt'

// One codebase for Chrome (and Edge) and Firefox, Manifest V3 for both.
export default defineConfig({
  srcDir: 'src',
  // Firefox defaults to Manifest V2 in WXT; V3 keeps the optional site permission and matches Chrome.
  manifestVersion: 3,
  modules: ['@wxt-dev/module-react'],
  manifest: ({ browser }) => ({
    name: 'Linkdean',
    description: 'Save the page you are on to your Linkdean library, on linkdean.app or your own instance.',
    homepage_url: 'https://linkdean.app',
    // activeTab: the current tab's address and title, only when the toolbar button, menu or
    // shortcut is used. scripting: read the selected text in that tab. identity: the sign-in
    // window. The instance itself is asked for at sign-in, so no site is reachable before that.
    permissions: ['activeTab', 'contextMenus', 'scripting', 'storage', 'identity'],
    optional_host_permissions: ['https://*/*', 'http://localhost/*', 'http://127.0.0.1/*'],
    action: { default_title: 'Save to Linkdean' },
    commands: {
      _execute_action: {
        suggested_key: { default: 'Alt+Shift+S' },
        description: 'Save this page to Linkdean',
      },
    },
    ...(browser === 'firefox'
      ? {
          browser_specific_settings: {
            gecko: {
              id: 'extension@linkdean.app',
              strict_min_version: '128.0',
              // What leaves the browser, and only to the instance the person signs in to: the
              // addresses of pages they save, and the text they select as a description.
              data_collection_permissions: { required: ['browsingActivity', 'websiteContent'] },
            },
          },
        }
      : {}),
  }),
})
