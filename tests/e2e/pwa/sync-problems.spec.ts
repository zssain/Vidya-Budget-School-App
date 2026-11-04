import { test, expect, type Page, type Route } from '@playwright/test'
import type { E2ESeed } from '../../../web-pwa/e2e-seed'

// iPhone PWA, refused changes (repair D1/B1) — the "Dismiss" flow end to end in
// WebKit against the test-only seeding build (web-pwa/dist-e2e; see
// playwright.pwa.config.ts). The app is seeded through its own code (join kv +
// secrets, createPin, the real phone write commands, the real ack application) into
// a JOINED, PIN-protected teacher with ONE refused change (a homework note the school
// PC refused: FORBIDDEN) and ONE unrelated pending change (a staff check-in).
//
// Google is fully stubbed: the GIS script is a fake that hands out a token, and the
// Drive REST v3 endpoints are an in-memory fake Drive in this test. Every bundle the
// app uploads is opened with the device's session key (same WASM the app uses), so we
// can prove exactly which ops were sent. Any other non-localhost request fails the test.

const PIN = '2468'

// ---- fake Google (GIS + Drive REST v3) ------------------------------------------

interface FakeNode {
  id: string
  name: string
  parent: string
  folder: boolean
  bytes: Buffer
}

class FakeGoogle {
  nodes = new Map<string, FakeNode>()
  seq = 0
  /** Raw bytes of every file uploaded under an ops-<device> folder (sealed bundles). */
  bundleUploads: Buffer[] = []
  /** Requests to hosts we do not stub (must stay empty). */
  unexpected: string[] = []

  private json(route: Route, body: unknown, status = 200): Promise<void> {
    return route.fulfill({ status, contentType: 'application/json', headers: CORS, body: JSON.stringify(body) })
  }

  async handle(route: Route): Promise<void> {
    const req = route.request()
    const url = new URL(req.url())
    if (url.hostname === 'localhost') return route.continue()
    if (url.hostname === 'accounts.google.com' && url.pathname === '/gsi/client') {
      return route.fulfill({ status: 200, contentType: 'text/javascript', body: FAKE_GIS })
    }
    if (url.hostname !== 'www.googleapis.com') {
      this.unexpected.push(req.url())
      return route.abort()
    }
    if (req.method() === 'OPTIONS') return route.fulfill({ status: 204, headers: CORS })
    if (req.headers()['authorization'] !== 'Bearer e2e-fake-token') return this.json(route, { error: 'no auth' }, 401)

    const files = '/drive/v3/files'
    const upload = '/upload/drive/v3/files'
    if (req.method() === 'GET' && url.pathname === files) {
      const q = url.searchParams.get('q') ?? ''
      const m = /^'((?:[^'\\]|\\.)*)' in parents(?: and name='((?:[^'\\]|\\.)*)')? and trashed=false$/.exec(q)
      if (!m) return this.json(route, { error: `bad q ${q}` }, 400)
      const unq = (s: string) => s.replace(/\\(.)/g, '$1')
      const parent = unq(m[1])
      const name = m[2] != null ? unq(m[2]) : null
      const hits = [...this.nodes.values()].filter((n) => n.parent === parent && (name == null || n.name === name))
      return this.json(route, { files: hits.map((n) => ({ id: n.id, name: n.name })) })
    }
    if (req.method() === 'POST' && url.pathname === files) {
      const meta = JSON.parse(req.postData() ?? '{}') as { name: string; parents: string[] }
      return this.json(route, { id: this.add(meta.name, meta.parents[0], true, Buffer.alloc(0)) })
    }
    if (req.method() === 'POST' && url.pathname === upload) {
      const { meta, media } = parseMultipart(req.headers()['content-type'] ?? '', req.postDataBuffer() ?? Buffer.alloc(0))
      const parent = this.nodes.get(meta.parents[0])
      if (parent?.name.startsWith('ops-')) this.bundleUploads.push(media)
      return this.json(route, { id: this.add(meta.name, meta.parents[0], false, media) })
    }
    const one = new RegExp(`^${files}/([^/]+)$`).exec(url.pathname)
    if (one) {
      const node = this.nodes.get(decodeURIComponent(one[1]))
      if (!node) return this.json(route, { error: 'not found' }, 404)
      if (req.method() === 'PATCH') {
        node.name = (JSON.parse(req.postData() ?? '{}') as { name: string }).name
        return this.json(route, { id: node.id })
      }
      if (req.method() === 'DELETE') {
        this.nodes.delete(node.id)
        return route.fulfill({ status: 204, headers: CORS })
      }
      if (req.method() === 'GET' && url.searchParams.get('alt') === 'media') {
        return route.fulfill({ status: 200, contentType: 'application/octet-stream', headers: CORS, body: node.bytes })
      }
    }
    this.unexpected.push(`${req.method()} ${req.url()}`)
    return this.json(route, { error: 'unhandled' }, 400)
  }

  private add(name: string, parent: string, folder: boolean, bytes: Buffer): string {
    this.seq += 1
    const id = `${folder ? 'fld' : 'file'}-${this.seq}`
    this.nodes.set(id, { id, name, parent, folder, bytes })
    return id
  }
}

const CORS = {
  'access-control-allow-origin': '*',
  'access-control-allow-headers': 'authorization, content-type',
  'access-control-allow-methods': 'GET, POST, PATCH, DELETE, OPTIONS',
}

// A stand-in for Google Identity Services: the token model the app uses
// (initTokenClient → requestAccessToken → callback) with a fixed fake token.
const FAKE_GIS = `window.google = { accounts: { oauth2: { initTokenClient: function (c) {
  return { requestAccessToken: function () { setTimeout(function () {
    c.callback({ access_token: 'e2e-fake-token', expires_in: 3600 }) }, 0) } } } } } }`

/** Split GoogleDrive.upload's multipart/related body into its metadata + media. */
function parseMultipart(contentType: string, body: Buffer): { meta: { name: string; parents: string[] }; media: Buffer } {
  const boundary = /boundary=([^;]+)/.exec(contentType)?.[1]
  if (!boundary) throw new Error(`no multipart boundary in ${contentType}`)
  const sep = Buffer.from(`--${boundary}\r\n`)
  const first = body.indexOf(sep)
  const second = body.indexOf(sep, first + sep.length)
  const metaStart = body.indexOf('\r\n\r\n', first) + 4
  const meta = JSON.parse(body.subarray(metaStart, second - 2).toString('utf8'))
  const mediaStart = body.indexOf('\r\n\r\n', second) + 4
  const mediaEnd = body.lastIndexOf(Buffer.from(`\r\n--${boundary}--`))
  return { meta, media: body.subarray(mediaStart, mediaEnd) }
}

/** Open every uploaded bundle (device-sealed v2, c2s key) in the page with the same
 *  vidya-wasm module the app uses, and return the op ids each one carried. */
async function uploadedOpIds(page: Page, google: FakeGoogle, seed: E2ESeed): Promise<string[][]> {
  const bundles = google.bundleUploads.map((b) => b.toString('base64'))
  return page.evaluate(
    async ({ bundles, sessionKey, deviceId }) => {
      const modUrl = new URL('/wasm/vidya_wasm.js', location.origin).href
      const mod: Record<string, (...a: unknown[]) => unknown> & { default: () => Promise<unknown> } =
        await import(/* @vite-ignore */ modUrl)
      await mod.default()
      const keys = mod.derive_direction_keys(sessionKey) as Uint8Array
      const c2s = keys.slice(0, 32)
      const aad = mod.device_bundle_aad(deviceId)
      return bundles.map((b64) => {
        const sealed = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0))
        const plain = mod.seal_open(c2s, aad, sealed) as Uint8Array
        return (JSON.parse(new TextDecoder().decode(plain)) as { op_id: string }[]).map((o) => o.op_id)
      })
    },
    { bundles, sessionKey: seed.sessionKey, deviceId: seed.deviceId },
  )
}

async function unlockWithPin(page: Page): Promise<void> {
  const pin = page.locator('#pin')
  await expect(pin).toBeVisible()
  await pin.fill(PIN)
  await page.getByRole('button', { name: 'Unlock' }).click()
  await expect(pin).toBeHidden()
}

// Playwright's WebKit does not expose Blob request bodies to route handlers, and
// GoogleDrive.upload sends its multipart body as a Blob. This init script (test-side,
// not app code) hands the SAME bytes to fetch as a Uint8Array instead, so the fake
// Drive can read what was uploaded. Only Drive upload requests are touched.
const BLOB_BODY_SHIM = `(() => {
  const orig = window.fetch.bind(window)
  window.fetch = async (input, init) => {
    const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url
    if (url.startsWith('https://www.googleapis.com/upload/') && init && init.body instanceof Blob) {
      init = { ...init, body: new Uint8Array(await init.body.arrayBuffer()) }
    }
    return orig(input, init)
  }
})()`

// The service worker would sit between the page and our route stubs; the SW itself
// is covered by pwa.spec.ts.
test.use({ serviceWorkers: 'block' })

test('dismissing a refused change removes only it, survives reload, and never re-sends it', async ({ page }) => {
  const google = new FakeGoogle()
  await page.addInitScript(BLOB_BODY_SHIM)
  await page.route('**/*', (route) => google.handle(route))
  const pageErrors: string[] = []
  page.on('pageerror', (e) => pageErrors.push(String(e)))

  // 1. Seed a joined teacher (fresh origin storage per test context).
  await page.goto('/')
  await page.waitForFunction(() => !!window.__vidyaE2E)
  const seed = await page.evaluate((pin) => window.__vidyaE2E!.seedJoinedTeacher({ pin }), PIN)
  expect(await page.evaluate(() => window.__vidyaE2E!.outbox())).toEqual({
    pending: [seed.pendingOpId],
    refused: [{ id: seed.refusedOpId, status: 'rejected', reason: 'FORBIDDEN' }],
  })

  // 2. Relaunch: the joined app is at the PIN gate; unlock through the real screen.
  await page.reload()
  await unlockWithPin(page)

  // The foreground Drive loop starts on unlock and pushes the pending change — and
  // only it (the refused one was "uploaded" 2 h ago, past the re-upload window).
  await expect.poll(() => google.bundleUploads.length, { timeout: 20_000 }).toBeGreaterThan(0)
  expect((await uploadedOpIds(page, google, seed)).flat()).toEqual([seed.pendingOpId])

  // 3. The refused change is listed on About with its reason; dismiss it.
  await page.evaluate(() => (location.hash = '/teacher/about'))
  const card = page.getByTestId('sync-problems')
  await expect(card).toContainText('Changes not accepted')
  await expect(card).toContainText('Homework / note')
  const dismiss = card.getByRole('button', { name: 'Dismiss: Homework / note' })
  await expect(dismiss).toHaveCount(1)
  await dismiss.click()
  await expect(card).toContainText('All your changes were accepted.')
  await expect(card.getByRole('button', { name: /^Dismiss/ })).toHaveCount(0)

  // 4. Reload (locks again) and re-unlock: the dismissal persisted.
  await page.reload()
  await unlockWithPin(page)
  await page.evaluate(() => (location.hash = '/teacher/about'))
  await expect(page.getByTestId('sync-problems')).toContainText('All your changes were accepted.')
  await expect(page.getByTestId('sync-problems').getByRole('button', { name: /^Dismiss/ })).toHaveCount(0)

  // The pending change was NOT dismissed: still in the outbox, not refused, and the
  // Sync screen still counts it as waiting to send.
  expect(await page.evaluate(() => window.__vidyaE2E!.outbox())).toEqual({ pending: [seed.pendingOpId], refused: [] })
  await page.evaluate(() => (location.hash = '/teacher/sync'))
  await expect(page.getByText('Waiting to send')).toBeVisible()
  await expect(page.getByText('1 change(s)')).toBeVisible()

  // 5. "Sync now" runs one more full Drive pass (push → acks → down) after the
  // dismissal; it must finish, and no bundle ever uploaded may carry the refused op.
  await page.getByRole('button', { name: 'Sync now' }).click()
  await expect(page.getByRole('status')).toHaveText('Sync finished.', { timeout: 30_000 })
  const sent = (await uploadedOpIds(page, google, seed)).flat()
  expect(sent).not.toContain(seed.refusedOpId)
  expect(sent).toContain(seed.pendingOpId)
  expect(google.unexpected, 'requests to unstubbed hosts / endpoints').toEqual([])
  expect(pageErrors).toEqual([])
})
