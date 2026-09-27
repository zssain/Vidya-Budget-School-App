// In-memory DriveApi for tests (Phase 19). Mirrors the Rust FakeDrive: a single
// account's folder tree, so the harness can exercise push/ack/pull and the join
// exchange without touching Google. Pure TS (a Map), so it also runs under vitest.

import type { DriveApi, DriveFile, UploadOpts } from './api'

interface Node {
  id: string
  name: string
  parent: string
  isFolder: boolean
  bytes: Uint8Array
  properties?: Record<string, string>
}

export class FakeDrive implements DriveApi {
  private nodes = new Map<string, Node>()
  private seq = 0

  root(): string {
    return 'root'
  }

  private newId(kind: string): string {
    this.seq += 1
    return `${kind}-${this.seq}`
  }

  async list(folderId: string): Promise<DriveFile[]> {
    return [...this.nodes.values()]
      .filter((n) => n.parent === folderId)
      .map((n) => ({ id: n.id, name: n.name, properties: n.properties }))
  }

  async findChild(folderId: string, name: string): Promise<DriveFile | null> {
    const n = [...this.nodes.values()].find((x) => x.parent === folderId && x.name === name)
    return n ? { id: n.id, name: n.name, properties: n.properties } : null
  }

  async ensureFolder(parentId: string, name: string): Promise<string> {
    const existing = await this.findChild(parentId, name)
    if (existing) return existing.id
    const id = this.newId('folder')
    this.nodes.set(id, { id, name, parent: parentId, isFolder: true, bytes: new Uint8Array() })
    return id
  }

  async upload(parentId: string, name: string, bytes: Uint8Array, opts?: UploadOpts): Promise<string> {
    const id = this.newId('file')
    this.nodes.set(id, { id, name, parent: parentId, isFolder: false, bytes, properties: opts?.properties })
    return id
  }

  async download(fileId: string): Promise<Uint8Array> {
    const n = this.nodes.get(fileId)
    if (!n) throw new Error(`FakeDrive: no file ${fileId}`)
    return n.bytes
  }

  async rename(fileId: string, name: string): Promise<void> {
    const n = this.nodes.get(fileId)
    if (!n) throw new Error(`FakeDrive: no file ${fileId}`)
    n.name = name
  }

  async remove(fileId: string): Promise<void> {
    this.nodes.delete(fileId)
  }
}
