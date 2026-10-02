// Dev-only Q-B test harness (docs/phase-notes/drive-file-spike.md). It reuses the REAL
// P19 web OAuth (the Web client + drive.file scope) so the result reflects exactly what
// the iPhone PWA would see. It lists every file this Web client can access; if files the
// DESKTOP client created (the *.vbak backups) show up, Q-B = YES and §18's shared Drive
// exchange works across OAuth clients. NOT part of the production build — it is only
// served by `npm run dev:web` at http://localhost:5273/drive-test.html (add that origin
// to the Web OAuth client's Authorized JavaScript origins first).

import { connectDrive, getAccessToken, driveConfigured } from '@/lib/web/drive/auth'

const out = document.getElementById('out') as HTMLPreElement
const log = (s = '') => {
  out.textContent += s + '\n'
}

interface DriveFile {
  name: string
  mimeType: string
}

document.getElementById('run')!.addEventListener('click', async () => {
  out.textContent = ''
  if (!driveConfigured()) {
    log('✗ No Web client id configured. Put VITE_GOOGLE_CLIENT_ID_WEB in web-pwa/.env.local and restart dev:web.')
    return
  }
  try {
    log('Signing in to Google (Web client, drive.file)…')
    await connectDrive() // interactive: pick the SAME account used for the desktop backup
    const token = await getAccessToken()
    log('✓ Signed in. Listing files this Web client can access…')
    log()

    const params = new URLSearchParams({
      q: 'trashed = false',
      fields: 'files(id,name,mimeType,size)',
      spaces: 'drive',
      pageSize: '200',
    })
    const resp = await fetch(`https://www.googleapis.com/drive/v3/files?${params}`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    if (!resp.ok) {
      log(`✗ Drive list failed: HTTP ${resp.status} ${await resp.text()}`)
      return
    }
    const data = (await resp.json()) as { files?: DriveFile[] }
    const files = data.files ?? []
    const vbak = files.filter((f) => f.name.endsWith('.vbak'))

    log(`This Web client can see ${files.length} file(s):`)
    for (const f of files) log(`  • ${f.name}  [${f.mimeType}]`)
    log()
    log('──────── VERDICT ────────')
    if (vbak.length > 0) {
      log(`✓ Q-B = YES — ${vbak.length} desktop-created .vbak file(s) are visible to the Web client.`)
      log('  The shared Google Drive exchange works across OAuth clients → build the')
      log('  Drive sync client + iPhone sync on the §18 design.')
    } else {
      log('✗ Q-B = NO (assuming you already ran a desktop backup as THIS account).')
      log('  A Web client cannot read files the Desktop client created → the iPhone')
      log('  cannot use the Drive exchange; fall back to the relay (see drive-file-spike.md).')
      log('  (If you have NOT yet run a desktop backup as this account, do that first and retry.)')
    }
  } catch (e) {
    log('✗ Error: ' + (e as Error).message)
  }
})
