// Homework/notes history for a class (Phase 20, C3) — a faithful TS port of
// list_homework_notes_logic + NOTE_SELECT + note_dto (src-tauri/src/commands/logic.rs)
// over synced records. Pure (records in → DTO out) → unit-tested. LEFT JOINs: a note
// keeps its row even when the class_subject / subject / author aren't present (→ null).

import type { AttachmentMeta, HomeworkNoteDto } from '../api'

// vidya_core::notes::DELETE_WINDOW_MS — author may delete their own note for 24 h.
export const DELETE_WINDOW_MS = 24 * 60 * 60 * 1000

export interface HomeworkNoteRec {
  id: string
  class_id: string
  class_subject_id: string | null
  kind: string
  text: string
  attachments_json?: string | null
  created_by: string | null
  created_at: string
}

function parseAttachments(json: string | null | undefined): AttachmentMeta[] {
  if (!json) return []
  try {
    const v = JSON.parse(json)
    return Array.isArray(v) ? (v as AttachmentMeta[]) : []
  } catch {
    return []
  }
}

export interface HomeworkNotesInput {
  classId: string
  staffId: string
  nowMs: number
  notes: HomeworkNoteRec[]
  classSubjects: { id: string; subject_id: string }[]
  subjects: { id: string; name: string }[]
  staff: { id: string; name: string }[]
}

export function buildHomeworkNotes(input: HomeworkNotesInput): HomeworkNoteDto[] {
  const csById = new Map(input.classSubjects.map((cs) => [cs.id, cs]))
  const subById = new Map(input.subjects.map((s) => [s.id, s]))
  const staffById = new Map(input.staff.map((s) => [s.id, s]))

  return input.notes
    .filter((h) => h.class_id === input.classId)
    .sort((a, b) => (a.created_at < b.created_at ? 1 : a.created_at > b.created_at ? -1 : 0)) // created_at DESC
    .map((h) => {
      const cs = h.class_subject_id ? csById.get(h.class_subject_id) : undefined
      const subject_name = cs ? (subById.get(cs.subject_id)?.name ?? null) : null
      const isAuthor = h.created_by != null && h.created_by === input.staffId
      const elapsed = input.nowMs - Date.parse(h.created_at)
      return {
        id: h.id,
        class_id: h.class_id,
        class_subject_id: h.class_subject_id ?? null,
        subject_name,
        kind: h.kind,
        text: h.text,
        attachments: parseAttachments(h.attachments_json),
        created_by: h.created_by ?? null,
        created_by_name: h.created_by ? (staffById.get(h.created_by)?.name ?? null) : null,
        created_at: h.created_at,
        // vidya_core::notes::can_delete_own(is_author, elapsed_ms)
        can_delete: isAuthor && elapsed >= 0 && elapsed <= DELETE_WINDOW_MS,
      }
    })
}
