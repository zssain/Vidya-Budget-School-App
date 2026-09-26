// Photo compression for bill/attachment uploads (P15 Step 2, §10.3).
//
// Desktop: a webview canvas resizes the image to a long edge ≤ 1600 px and
// re-encodes it as JPEG quality 0.7, then base64-encodes the bytes for
// `save_attachment`. On Android the in-repo `vidya-android` plugin does the same
// natively (Bitmap → JPEG q70); that native path is a documented follow-up (no
// Android toolchain here). Both keep a bill photo comfortably under the 2 MB cap.

const MAX_EDGE = 1600
const QUALITY = 0.7

/** Compress an image File to a base64 JPEG (long edge ≤ 1600 px, q70). */
export async function compressImageToJpegBase64(file: File): Promise<string> {
  const bitmap = await createImageBitmap(file)
  const scale = Math.min(1, MAX_EDGE / Math.max(bitmap.width, bitmap.height))
  const w = Math.max(1, Math.round(bitmap.width * scale))
  const h = Math.max(1, Math.round(bitmap.height * scale))
  const canvas = document.createElement('canvas')
  canvas.width = w
  canvas.height = h
  const ctx = canvas.getContext('2d')
  if (!ctx) throw new Error('no 2d context')
  ctx.drawImage(bitmap, 0, 0, w, h)
  bitmap.close?.()
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/jpeg', QUALITY))
  if (!blob) throw new Error('compression failed')
  const bytes = new Uint8Array(await blob.arrayBuffer())
  let binary = ''
  for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i])
  return btoa(binary)
}
