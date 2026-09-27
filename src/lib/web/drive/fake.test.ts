import { describe, expect, it } from 'vitest'
import { FakeDrive } from './fake'
import { ACKS, EXCHANGE, JOINS, opsFolder } from './api'

describe('FakeDrive', () => {
  it('creates folders idempotently and round-trips files', async () => {
    const d = new FakeDrive()
    const vidya = await d.ensureFolder(d.root(), 'Vidya')
    // ensureFolder is idempotent (same id on a second call).
    expect(await d.ensureFolder(d.root(), 'Vidya')).toBe(vidya)
    const exch = await d.ensureFolder(vidya, 'exchange')

    const bytes = new Uint8Array([1, 2, 3, 4])
    const id = await d.upload(exch, 'x-admin-v1.vop', bytes, { properties: { vidya_audience: 'admin' } })
    expect(await d.download(id)).toEqual(bytes)

    const listed = await d.list(exch)
    expect(listed.map((f) => f.name)).toEqual(['x-admin-v1.vop'])
    expect(listed[0].properties?.vidya_audience).toBe('admin')

    // findChild by name; rename; then it's found under the new name only.
    expect((await d.findChild(exch, 'x-admin-v1.vop'))?.id).toBe(id)
    await d.rename(id, 'y-admin-v1.vop')
    expect(await d.findChild(exch, 'x-admin-v1.vop')).toBeNull()
    expect((await d.findChild(exch, 'y-admin-v1.vop'))?.id).toBe(id)

    await d.remove(id)
    expect(await d.list(exch)).toEqual([])
  })

  // Model the real exchange layout (drive/sync.ts, drive/api.ts) at the I/O level:
  // Vidya/<school>/exchange/{ops-<device>, acks, joins}, HLC-named audience .vop
  // bundles. This locks the contract the browser sync/join code depends on, so the
  // FakeDrive stays a faithful stand-in for the real DriveApi.
  it('supports the exchange layout the sync route relies on', async () => {
    const d = new FakeDrive()
    const vidya = await d.ensureFolder(d.root(), 'Vidya')
    const school = await d.ensureFolder(vidya, 'Sunrise (sch_1)')
    const exch = await d.ensureFolder(school, EXCHANGE)

    // This device's outbox + the standard exchange subfolders.
    const myOps = await d.ensureFolder(exch, opsFolder('devA'))
    await d.ensureFolder(exch, ACKS)
    const joins = await d.ensureFolder(exch, JOINS)

    // Two sealed op bundles, HLC-named so a lexical sort is chronological, tagged
    // with their audience + key version in appProperties (as pushOps writes them).
    const enc = new TextEncoder()
    await d.upload(myOps, '0000000000001-000-devA-admin-v1.vop', enc.encode('a'), {
      properties: { vidya_audience: 'admin', vidya_keyver: '1' },
    })
    await d.upload(myOps, '0000000000002-000-devA-class:5-v1.vop', enc.encode('b'), {
      properties: { vidya_audience: 'class:5', vidya_keyver: '1' },
    })

    const files = await d.list(myOps)
    // Lexical order over HLC-prefixed names == chronological order.
    const names = files.map((f) => f.name).sort()
    expect(names[0]).toContain('0000000000001-')
    expect(names[1]).toContain('0000000000002-')

    // A device pulls only the audiences it holds a key for (here: admin).
    const held = new Set(['admin'])
    const pulled = files.filter((f) => held.has(f.properties?.vidya_audience ?? ''))
    expect(pulled).toHaveLength(1)
    expect(pulled[0].properties?.vidya_audience).toBe('admin')

    // Join request/response round-trip through exchange/joins (join-without-LAN, §18).
    const reqId = 'req-123'
    await d.upload(joins, `${reqId}.vjoin`, enc.encode('sealed-request'), {
      properties: { vidya_kind: 'join_request' },
    })
    // The school PC answers with a sealed response beside it.
    await d.upload(joins, `${reqId}.vjoinresp`, enc.encode('sealed-response'), {
      properties: { vidya_kind: 'join_response' },
    })
    const resp = await d.findChild(joins, `${reqId}.vjoinresp`)
    expect(resp).not.toBeNull()
    expect(new TextDecoder().decode(await d.download(resp!.id))).toBe('sealed-response')
  })
})
