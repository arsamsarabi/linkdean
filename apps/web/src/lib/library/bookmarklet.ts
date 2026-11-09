/**
 * The bookmarklet for Settings → Fast capture. Run on any page, it opens `/save` on this
 * instance in a small window with the page's address, title and selected text. A blocked
 * popup falls back to opening `/save` in the same tab.
 *
 * Kept as plain ES5 in one line: it runs inside other sites' pages, in whatever browser.
 */
export function bookmarkletSource(origin: string): string {
  const base = JSON.stringify(`${origin.replace(/\/+$/, '')}/save?`)
  const code =
    '(function(){' +
    'var s=String(window.getSelection?window.getSelection():"").trim().slice(0,1000);' +
    'var q="url="+encodeURIComponent(location.href)+"&title="+encodeURIComponent(document.title)' +
    '+(s?"&text="+encodeURIComponent(s):"");' +
    `var b=${base};` +
    'if(!window.open(b+"popup=1&"+q,"linkdean_save","width=560,height=760,resizable=yes,scrollbars=yes"))' +
    'location.href=b+q;' +
    '})()'
  return `javascript:${code}`
}
