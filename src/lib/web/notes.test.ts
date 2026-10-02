import { describe, expect, it } from 'vitest'
import { buildHomeworkNotes, DELETE_WINDOW_MS, type HomeworkNotesInput } from './notes'

// Mirrors list_homework_notes_logic + note_dto: a class's notes newest-first, subject +
// author names via LEFT JOIN (missing refs → null), attachments parsed, and can_delete
// = author && within the 24 h window.
const NOW = Date.UTC(2026, 9, 3, 12, 0, 0)

function base(): HomeworkNotesInput {
  return {
    classId: 'c5',
    staffId: 'stf-1',
    nowMs: NOW,
    notes: [
      { id: 'n1', class_id: 'c5', class_subject_id: 'cs-m', kind: 'homework', text: 'Pg 12', attachments_json: '[{"name":"a.pdf","size":10,"mime":"application/pdf"}]', created_by: 'stf-1', created_at: '2026-10-03T09:00:00.000Z' },
      { id: 'n2', class_id: 'c5', class_subject_id: null, kind: 'note', text: 'Bring colors', attachments_json: null, created_by: 'stf-2', created_at: '2026-10-01T09:00:00.000Z' },
      { id: 'n3', class_id: 'c6', class_subject_id: 'cs-m', kind: 'note', text: 'other class', attachments_json: null, created_by: 'stf-1', created_at: '2026-10-03T10:00:00.000Z' },
    ],
    classSubjects: [{ id: 'cs-m', subject_id: 'sub-m' }],
    subjects: [{ id: 'sub-m', name: 'Maths' }],
    staff: [
      { id: 'stf-1', name: 'Meena' },
      { id: 'stf-2', name: 'Ravi' },
    ],
  }
}

describe('buildHomeworkNotes', () => {
  it('returns the class notes newest-first with joined names + attachments', () => {
    const d = buildHomeworkNotes(base())
    expect(d.map((n) => n.id)).toEqual(['n1', 'n2']) // c6 excluded; created_at DESC
    expect(d[0]).toMatchObject({ subject_name: 'Maths', created_by_name: 'Meena', can_delete: true })
    expect(d[0].attachments).toHaveLength(1)
    // n2: no class_subject → null subject; different author; authored >24h ago.
    expect(d[1]).toMatchObject({ subject_name: null, created_by_name: 'Ravi', can_delete: false })
  })

  it('lets the author delete only inside the 24h window', () => {
    const mk = (createdAt: string, by: string) => ({
      ...base(),
      notes: [{ id: 'x', class_id: 'c5', class_subject_id: null, kind: 'note', text: 't', created_by: by, created_at: createdAt }],
    })
    const justInside = new Date(NOW - DELETE_WINDOW_MS).toISOString()
    const justOutside = new Date(NOW - DELETE_WINDOW_MS - 1000).toISOString()
    expect(buildHomeworkNotes(mk(justInside, 'stf-1'))[0].can_delete).toBe(true)
    expect(buildHomeworkNotes(mk(justOutside, 'stf-1'))[0].can_delete).toBe(false)
    expect(buildHomeworkNotes(mk(justInside, 'stf-2'))[0].can_delete).toBe(false) // not the author
  })
})
