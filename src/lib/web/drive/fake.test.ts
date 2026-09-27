import { describe, expect, it } from 'vitest'
import { FakeDrive } from './fake'

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
})
