// Registers the service worker and surfaces the update flow (Phase 19, Step 6). Only
// in production builds (the dev server has no sw.js). When a new version has
// installed and is waiting, a minimal banner offers "Reload"; tapping it tells the
// waiting worker to activate, and the resulting controllerchange reloads the page.
// Styled with the app's design tokens (loaded from app.css before this runs).

function showUpdateBanner(onReload: () => void): void {
  if (document.getElementById('vidya-update-banner')) return
  const bar = document.createElement('div')
  bar.id = 'vidya-update-banner'
  bar.setAttribute('role', 'status')
  bar.style.cssText = [
    'position:fixed',
    'left:16px',
    'right:16px',
    'bottom:calc(16px + env(safe-area-inset-bottom))',
    'z-index:2147483647',
    'background:var(--navy)',
    'color:var(--on-navy-strong)',
    'border-radius:10px',
    'padding:12px 16px',
    'display:flex',
    'align-items:center',
    'justify-content:space-between',
    'gap:12px',
    "font:500 14px 'Geist', system-ui, sans-serif",
    'box-shadow:0 8px 24px rgba(11,26,51,0.3)',
  ].join(';')

  const msg = document.createElement('span')
  msg.textContent = 'A new version of Vidya is ready.'

  const btn = document.createElement('button')
  btn.type = 'button'
  btn.textContent = 'Reload'
  btn.style.cssText = [
    'background:var(--gold)',
    'color:var(--navy)',
    'border:0',
    'border-radius:6px',
    'padding:8px 14px',
    "font:600 14px 'Geist', system-ui, sans-serif",
  ].join(';')
  btn.addEventListener('click', () => {
    bar.remove()
    onReload()
  })

  bar.append(msg, btn)
  document.body.appendChild(bar)
}

export function registerServiceWorker(): void {
  if (!import.meta.env.PROD || !('serviceWorker' in navigator)) return
  window.addEventListener('load', () => {
    navigator.serviceWorker
      .register('/sw.js')
      .then((reg) => {
        reg.addEventListener('updatefound', () => {
          const next = reg.installing
          if (!next) return
          next.addEventListener('statechange', () => {
            // A new worker installed while an old one controls the page → update ready.
            if (next.state === 'installed' && navigator.serviceWorker.controller) {
              showUpdateBanner(() => next.postMessage('SKIP_WAITING'))
            }
          })
        })
      })
      .catch(() => {
        /* SW registration is best-effort; the app still works online. */
      })

    let refreshing = false
    navigator.serviceWorker.addEventListener('controllerchange', () => {
      if (refreshing) return
      refreshing = true
      location.reload()
    })
  })
}
