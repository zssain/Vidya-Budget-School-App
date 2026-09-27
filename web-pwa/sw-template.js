// Hand-written service worker for the Vidya iPhone PWA (Phase 19, Step 6). No
// Workbox. At build time (web-pwa/vite.config.ts) the two placeholders below are
// replaced with a content-hash version and the precache list (the app shell + the
// hashed JS/CSS/WASM/font assets), and the result is written to dist/sw.js.
//
//   * install  — precache the shell + assets into a versioned cache.
//   * activate — delete caches from older versions; claim clients.
//   * fetch    — Google sign-in/Drive are NETWORK-ONLY (auth + live data, never
//                cached); same-origin requests are cache-first with a network
//                fallback that also fills the cache; an offline navigation falls
//                back to the cached app shell.
//   * message  — the page posts SKIP_WAITING when the user taps "Reload".
/* eslint-disable */
const VERSION = '__VERSION__'
const CACHE = 'vidya-' + VERSION
const PRECACHE = __PRECACHE__

self.addEventListener('install', (event) => {
  event.waitUntil(
    (async () => {
      const cache = await caches.open(CACHE)
      await cache.addAll(PRECACHE)
      // No auto-skipWaiting — the page shows "A new version is ready — Reload".
    })(),
  )
})

self.addEventListener('activate', (event) => {
  event.waitUntil(
    (async () => {
      const names = await caches.keys()
      await Promise.all(names.filter((n) => n !== CACHE).map((n) => caches.delete(n)))
      await self.clients.claim()
    })(),
  )
})

function isGoogle(url) {
  return (
    url.hostname === 'accounts.google.com' ||
    url.hostname.endsWith('googleapis.com') ||
    url.hostname.endsWith('google.com') ||
    url.hostname.endsWith('gstatic.com')
  )
}

self.addEventListener('fetch', (event) => {
  const req = event.request
  if (req.method !== 'GET') return
  const url = new URL(req.url)
  // Google APIs (sign-in, Drive) must always hit the network — never cache auth or
  // school change data. The DONE-MEANS network check depends on this.
  if (isGoogle(url)) return
  if (url.origin !== self.location.origin) return
  event.respondWith(
    (async () => {
      const cache = await caches.open(CACHE)
      const cached = await cache.match(req)
      if (cached) return cached
      try {
        const res = await fetch(req)
        if (res.ok && res.type === 'basic') cache.put(req, res.clone())
        return res
      } catch (err) {
        if (req.mode === 'navigate') {
          const shell = await cache.match('/index.html')
          if (shell) return shell
        }
        throw err
      }
    })(),
  )
})

self.addEventListener('message', (event) => {
  if (event.data === 'SKIP_WAITING') self.skipWaiting()
})
