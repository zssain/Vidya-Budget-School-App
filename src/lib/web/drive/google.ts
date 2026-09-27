// Google Drive REST v3 over fetch (Phase 19, Step 4). Browser-only, CORS-supported
// (spike Q3). Every request carries the GIS bearer token; a 401 refreshes the token
// once, a 403 quota surfaces DRIVE_QUOTA for the Home "Needs attention" copy. Scope
// is `drive.file`, so the app only ever sees files it created (the school's sync
// account exchange tree) — never the user's other Drive files.

import type { DriveApi, DriveFile, UploadOpts } from './api'
import { clearToken, getAccessToken } from './auth'

const FILES = 'https://www.googleapis.com/drive/v3/files'
const UPLOAD = 'https://www.googleapis.com/upload/drive/v3/files'
const FOLDER_MIME = 'application/vnd.google-apps.folder'

function qEscape(s: string): string {
  return s.replace(/\\/g, '\\\\').replace(/'/g, "\\'")
}

async function authed(input: string, init: RequestInit = {}, retry = true): Promise<Response> {
  const token = await getAccessToken()
  const headers = new Headers(init.headers)
  headers.set('Authorization', `Bearer ${token}`)
  const res = await fetch(input, { ...init, headers })
  if (res.status === 401 && retry) {
    clearToken()
    return authed(input, init, false)
  }
  if (res.status === 403) throw new Error('DRIVE_QUOTA')
  if (!res.ok) throw new Error(`DRIVE_HTTP_${res.status}`)
  return res
}

export class GoogleDrive implements DriveApi {
  root(): string {
    return 'root'
  }

  async list(folderId: string): Promise<DriveFile[]> {
    const q = encodeURIComponent(`'${qEscape(folderId)}' in parents and trashed=false`)
    const url = `${FILES}?q=${q}&fields=files(id,name,appProperties)&pageSize=1000`
    const res = await authed(url)
    const json = (await res.json()) as { files?: { id: string; name: string; appProperties?: Record<string, string> }[] }
    return (json.files ?? []).map((f) => ({ id: f.id, name: f.name, properties: f.appProperties }))
  }

  async findChild(folderId: string, name: string): Promise<DriveFile | null> {
    const q = encodeURIComponent(`'${qEscape(folderId)}' in parents and name='${qEscape(name)}' and trashed=false`)
    const res = await authed(`${FILES}?q=${q}&fields=files(id,name,appProperties)&pageSize=1`)
    const json = (await res.json()) as { files?: { id: string; name: string; appProperties?: Record<string, string> }[] }
    const f = json.files?.[0]
    return f ? { id: f.id, name: f.name, properties: f.appProperties } : null
  }

  async ensureFolder(parentId: string, name: string): Promise<string> {
    const existing = await this.findChild(parentId, name)
    if (existing) return existing.id
    const res = await authed(`${FILES}?fields=id`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name, mimeType: FOLDER_MIME, parents: [parentId] }),
    })
    return ((await res.json()) as { id: string }).id
  }

  async upload(parentId: string, name: string, bytes: Uint8Array, opts?: UploadOpts): Promise<string> {
    const boundary = `vidya${Math.random().toString(36).slice(2)}`
    const metadata = {
      name,
      parents: [parentId],
      ...(opts?.properties ? { appProperties: opts.properties } : {}),
    }
    const head = `--${boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n${JSON.stringify(metadata)}\r\n--${boundary}\r\nContent-Type: ${opts?.mime ?? 'application/octet-stream'}\r\n\r\n`
    const tail = `\r\n--${boundary}--`
    const body = new Blob([head, bytes as BlobPart, tail])
    const res = await authed(`${UPLOAD}?uploadType=multipart&fields=id`, {
      method: 'POST',
      headers: { 'Content-Type': `multipart/related; boundary=${boundary}` },
      body,
    })
    return ((await res.json()) as { id: string }).id
  }

  async download(fileId: string): Promise<Uint8Array> {
    const res = await authed(`${FILES}/${encodeURIComponent(fileId)}?alt=media`)
    return new Uint8Array(await res.arrayBuffer())
  }

  async rename(fileId: string, name: string): Promise<void> {
    await authed(`${FILES}/${encodeURIComponent(fileId)}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name }),
    })
  }

  async remove(fileId: string): Promise<void> {
    await authed(`${FILES}/${encodeURIComponent(fileId)}`, { method: 'DELETE' })
  }
}
