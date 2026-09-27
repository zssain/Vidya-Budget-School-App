// Minimal static file server for the PWA e2e tests (Phase 19, Step 9). No deps —
// node:http + node:fs only (docs §13). Serves the built app (web-pwa/dist) at `/`
// and the raw wasm-pack module (web-pwa/wasm) at `/wasm/`, so a WebKit test can both
// (a) load the real app (with its Content-Security-Policy) and (b) import the WASM
// module directly to check the rules run in Apple's engine. Unknown extensionless
// paths fall back to index.html (the app is a hash-router SPA).
//
//   node scripts/serve-static.mjs [--port 5280]

import { createServer } from 'node:http'
import { readFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import { extname, normalize } from 'node:path'

const portArg = process.argv.indexOf('--port')
const PORT = portArg > -1 ? Number(process.argv[portArg + 1]) : 5280
const DIST = fileURLToPath(new URL('../web-pwa/dist/', import.meta.url))
const WASM = fileURLToPath(new URL('../web-pwa/wasm/', import.meta.url))

const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.wasm': 'application/wasm',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.webmanifest': 'application/manifest+json; charset=utf-8',
  '.png': 'image/png',
  '.svg': 'image/svg+xml',
  '.woff2': 'font/woff2',
  '.ico': 'image/x-icon',
}

// Resolve a URL path to a file under DIST or WASM. Prevents `..` traversal by
// normalising and rejecting any path that escapes the served roots.
function resolve(urlPath) {
  const clean = normalize(decodeURIComponent(urlPath.split('?')[0])).replace(/^(\.\.[/\\])+/, '')
  if (clean.startsWith('/wasm/')) return { root: WASM, rel: clean.slice('/wasm/'.length) || 'index.html' }
  return { root: DIST, rel: clean === '/' ? 'index.html' : clean.replace(/^\//, '') }
}

const server = createServer(async (req, res) => {
  const { root, rel } = resolve(req.url ?? '/')
  const send = async (file, code = 200) => {
    const body = await readFile(root + file)
    res.writeHead(code, { 'content-type': TYPES[extname(file)] ?? 'application/octet-stream' })
    res.end(body)
  }
  try {
    await send(rel)
  } catch {
    // SPA fallback for extensionless navigations (e.g. /join); real missing assets 404.
    if (root === DIST && !extname(rel)) {
      try {
        await send('index.html')
        return
      } catch {
        /* fall through to 404 */
      }
    }
    res.writeHead(404, { 'content-type': 'text/plain' })
    res.end('404')
  }
})

server.listen(PORT, () => console.log(`serve-static: http://localhost:${PORT} (dist + /wasm)`))
