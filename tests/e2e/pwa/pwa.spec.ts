import { test, expect } from '@playwright/test'

// iPhone-PWA tests in WebKit (Phase 19, Step 9) — Apple's engine, the production
// build, its real CSP + service worker. These verify the PWA *foundation* that is
// built today: the app boots under WebKit + CSP; the vidya-core rules/sealing/PIN
// run byte-identically in the browser via WASM; the service worker serves the shell
// offline; and persistent storage + IndexedDB work. (Full staff-flow tests — join,
// attendance, Drive sync — wait on the web-backend command porting; see
// docs/phase-notes/phase-19.md.)

test('boots under WebKit with its CSP (no white screen)', async ({ page }) => {
  const failed: string[] = []
  page.on('requestfailed', (r) => {
    // GIS/Drive requests may fail (no network/creds in the test) — ignore those; a
    // failed same-origin asset would mean CSP or the bundle broke the boot.
    const u = r.url()
    if (u.startsWith('http://localhost')) failed.push(`${u} — ${r.failure()?.errorText}`)
  })
  await page.goto('/')
  // The app rendered *something* (it reaches its status/welcome screen; app_state is
  // not on web yet, so a status screen is expected — the point is it did not crash).
  await expect(page.locator('#root')).not.toBeEmpty()
  await expect(page).toHaveTitle(/Vidya/)
  expect(failed, `same-origin requests failed:\n${failed.join('\n')}`).toEqual([])
})

test('vidya-core rules, sealing and Argon2id run in WebKit via WASM', async ({ page }) => {
  await page.goto('/')
  // Load the same wasm-pack module the app uses, in the real engine, under the app's
  // CSP ('self' import + 'wasm-unsafe-eval'), and exercise a rule, a seal round-trip
  // and a PIN hash/verify. The results must match vidya-core exactly.
  const r = await page.evaluate(async () => {
    // Runtime URL so TS treats this as a dynamic import (the module is served at
    // /wasm by scripts/serve-static.mjs, not resolvable at type-check time).
    const modUrl = new URL('/wasm/vidya_wasm.js', location.origin).href
    const mod: Record<string, (...a: unknown[]) => unknown> & { default: () => Promise<unknown> } =
      await import(/* @vite-ignore */ modUrl)
    await mod.default() // init(): fetches the co-located .wasm
    // vidya-core returns attendance % in tenths; its doctest asserts 612,58,0 -> 913.
    const percent = mod.percent_present(612, 58, 0)

    const key = new Uint8Array(32).fill(7)
    const aad = mod.bundle_aad('admin', 1)
    const nonce = new Uint8Array(12).fill(9)
    const pt = new Uint8Array([1, 2, 3, 4, 5])
    const sealed = mod.seal_with_nonce(key, aad, nonce, pt)
    const opened = mod.seal_open(key, aad, sealed)

    const salt = new Uint8Array(16).fill(3)
    const phc = mod.pin_hash_with_salt('1234', salt)

    return {
      percent,
      opened: Array.from(opened as Uint8Array).join(','),
      sealedGrew: (sealed as Uint8Array).length > pt.length, // nonce+tag overhead
      pinGood: mod.pin_verify('1234', phc),
      pinBad: mod.pin_verify('9999', phc),
      phcArgon2id: typeof phc === 'string' && phc.startsWith('$argon2id$'),
    }
  })
  expect(r.percent).toBe(913) // 91.3% — identical to vidya-core
  expect(r.opened).toBe('1,2,3,4,5') // decrypts back to the plaintext
  expect(r.sealedGrew).toBe(true)
  expect(r.pinGood).toBe(true)
  expect(r.pinBad).toBe(false)
  expect(r.phcArgon2id).toBe(true)
})

test('service worker precaches the app shell (offline-ready)', async ({ page }) => {
  await page.goto('/')
  // The SW registers on load (production build only). Wait until it is ready and
  // controlling the page.
  await page.evaluate(async () => {
    await navigator.serviceWorker.ready
  })
  await page.reload()
  await page.waitForFunction(() => !!navigator.serviceWorker.controller)

  // Everything needed to boot with no network is in the versioned cache — the exact
  // cached responses the SW's cache-first fetch handler returns when offline (the
  // shell, the manifest, the WASM, and the hashed JS/CSS). We assert the cache
  // contents directly: WebKit's setOffline emulation under Playwright short-circuits
  // fetch before the SW, so it cannot exercise the offline path reliably; the cache
  // being fully populated + the SW controlling the page is the honest proof.
  const report = await page.evaluate(async () => {
    const key = (await caches.keys()).find((k) => k.startsWith('vidya-'))
    if (!key) return null
    const cache = await caches.open(key)
    const urls = (await cache.keys()).map((r) => new URL(r.url).pathname)
    return {
      controller: !!navigator.serviceWorker.controller,
      shell: urls.includes('/index.html'),
      manifest: urls.includes('/manifest.webmanifest'),
      wasm: urls.some((u) => u.endsWith('.wasm')),
      js: urls.some((u) => u.endsWith('.js')),
      css: urls.some((u) => u.endsWith('.css')),
      count: urls.length,
    }
  })
  expect(report, 'no vidya-* cache was created').not.toBeNull()
  expect(report!.controller).toBe(true)
  expect(report!.shell).toBe(true)
  expect(report!.manifest).toBe(true)
  expect(report!.wasm).toBe(true)
  expect(report!.js).toBe(true)
  expect(report!.count).toBeGreaterThan(3)
})

test('persistent storage and IndexedDB are available', async ({ page }) => {
  await page.goto('/')
  const r = await page.evaluate(async () => {
    const persistIsFn = typeof navigator.storage?.persist === 'function'
    // persist() may return false in a headless browser; we only assert it is callable
    // and returns a boolean without throwing.
    const persisted = persistIsFn ? await navigator.storage.persist() : null
    const idbWorks = await new Promise<boolean>((resolve) => {
      const req = indexedDB.open('vidya-e2e-probe', 1)
      req.onupgradeneeded = () => req.result.createObjectStore('kv')
      req.onsuccess = () => {
        req.result.close()
        indexedDB.deleteDatabase('vidya-e2e-probe')
        resolve(true)
      }
      req.onerror = () => resolve(false)
    })
    return { persistIsFn, persistedType: typeof persisted, idbWorks }
  })
  expect(r.persistIsFn).toBe(true)
  expect(r.persistedType).toBe('boolean')
  expect(r.idbWorks).toBe(true)
})
