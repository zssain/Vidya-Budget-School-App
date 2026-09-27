// The browser-side Google Drive interface (Phase 19, Step 4). Mirrors the Rust
// `DriveApi` trait (src-tauri/src/sync/drive), so the PWA speaks the same exchange
// protocol as the app. Two implementations: `GoogleDrive` (real Drive REST v3 over
// fetch + a GIS access token) and `FakeDrive` (in-memory, for the harness tests).
// Screens/sync call this interface, never a concrete client.

export interface DriveFile {
  id: string
  name: string
  /** Drive file `appProperties` — the PWA stores the bundle audience + key version
   *  here so a reader can pick the right key without downloading (§11). */
  properties?: Record<string, string>
}

export interface UploadOpts {
  mime?: string
  properties?: Record<string, string>
}

export interface DriveApi {
  /** The id to treat as the Drive root when resolving the Vidya folder tree. */
  root(): string
  /** Files directly under `folderId` (id + name + properties). */
  list(folderId: string): Promise<DriveFile[]>
  /** A child of `folderId` with exactly `name`, or null. */
  findChild(folderId: string, name: string): Promise<DriveFile | null>
  /** Create a folder `name` under `parentId` if absent; returns its id. */
  ensureFolder(parentId: string, name: string): Promise<string>
  /** Upload bytes as `name` under `parentId` (multipart metadata+media). Returns id. */
  upload(parentId: string, name: string, bytes: Uint8Array, opts?: UploadOpts): Promise<string>
  /** Download a file's raw bytes (`alt=media`). */
  download(fileId: string): Promise<Uint8Array>
  /** Rename a file — used for temp-name-then-rename atomic bundle uploads (§11). */
  rename(fileId: string, name: string): Promise<void>
  /** Delete a file (archiving processed bundles / acks). */
  remove(fileId: string): Promise<void>
}

/** The Vidya exchange folder names (§6.2 / §11). */
export const NOTES_FOLDER = 'notes'
export const EXCHANGE = 'exchange'
export const ACKS = 'acks'
export const JOINS = 'joins'
export const EPOCH_FILE = 'epoch.json'
export const opsFolder = (deviceId: string): string => `ops-${deviceId}`
