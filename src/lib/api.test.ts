// Consistency test (prompts/P03 Step 5): src/lib/api.ts must expose exactly one
// wrapper per command in the shared commands.json. The Rust side asserts the
// registered commands match the same JSON, so all three stay in sync.

import { describe, expect, it } from 'vitest'
import * as api from './api'
import commandsJson from './commands.json'

const expected = new Set<string>([...commandsJson.commands, ...commandsJson.debugOnly])

// Every function exported from api.ts is a command wrapper (types are erased).
const wrappers = new Set(
  Object.entries(api)
    .filter(([, v]) => typeof v === 'function')
    .map(([k]) => k)
)

describe('api.ts ↔ commands.json', () => {
  it('has a wrapper for every command', () => {
    const missing = [...expected].filter((c) => !wrappers.has(c))
    expect(missing, `api.ts is missing wrappers: ${missing.join(', ')}`).toEqual([])
  })

  it('has no wrapper without a command', () => {
    const extra = [...wrappers].filter((w) => !expected.has(w))
    expect(extra, `api.ts has unexpected wrappers: ${extra.join(', ')}`).toEqual([])
  })
})
