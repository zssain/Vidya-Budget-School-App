// Pure helpers for the teacher phone screens behind the Teacher Home tiles (repair
// T1): which exam subjects a teacher may enter, which classes they write remarks
// for, the marks-entry grid ↔ MarkEntryInput, and the students list grouping.
// Unit-tested (teacherPhone.test.ts).

import type { ExamDto, ExamSubjectDto, MarkEntryInput, MarksSheetDto, StudentDto } from './api'

export interface MarksOption {
  exam_id: string
  exam_name: string
  subject: ExamSubjectDto
}

/** The exam subjects the teacher may enter: those whose class subject they teach
 *  (vidya_core::marks::can_edit — a class teacher who does not teach the subject is
 *  refused, so they are not offered). Newest exam first (list_exams order). */
export function marksOptions(exams: ExamDto[], classSubjectIds: string[]): MarksOption[] {
  const mine = new Set(classSubjectIds)
  return exams.flatMap((e) => e.subjects.filter((s) => mine.has(s.class_subject_id)).map((s) => ({ exam_id: e.id, exam_name: e.name, subject: s })))
}

/** One row of the marks grid (text so a half-typed number survives re-render). */
export interface MarkCell {
  student_id: string
  name: string
  roll_no: number | null
  value: string
  absent: boolean
}

export function cellsFromSheet(sheet: MarksSheetDto): MarkCell[] {
  return sheet.rows.map((r) => ({ student_id: r.student_id, name: r.name, roll_no: r.roll_no, value: r.marks == null ? '' : String(r.marks), absent: r.absent }))
}

/** Grid → entries for save/submit. Blank rows are left out (not entered yet); an
 *  absent row carries no marks. Returns the student ids whose value is not a whole
 *  number in 0..=max as `invalid` (nothing is sent then). */
export function entriesFromCells(cells: MarkCell[], max: number): { entries: MarkEntryInput[]; invalid: string[] } {
  const entries: MarkEntryInput[] = []
  const invalid: string[] = []
  for (const c of cells) {
    if (c.absent) {
      entries.push({ student_id: c.student_id, marks: null, absent: true })
      continue
    }
    const v = c.value.trim()
    if (v === '') continue
    const n = Number(v)
    if (!/^\d+$/.test(v) || !Number.isInteger(n) || n < 0 || n > max) {
      invalid.push(c.student_id)
      continue
    }
    entries.push({ student_id: c.student_id, marks: n, absent: false })
  }
  return { entries, invalid }
}

export interface RemarkOption {
  exam_id: string
  exam_name: string
  class_id: string
  class_display: string
}

/** Report-card remarks are the class teacher's (permissions EnterReportRemark): one
 *  option per exam × class they are class teacher of, where the exam covers that
 *  class (it has a subject there). Class names come from the dashboard. */
export function remarkOptions(exams: ExamDto[], classTeacherOf: string[], classNames: Record<string, string>): RemarkOption[] {
  const out: RemarkOption[] = []
  for (const e of exams) {
    for (const cid of classTeacherOf) {
      const hit = e.subjects.find((s) => s.class_id === cid)
      if (!hit) continue
      out.push({ exam_id: e.id, exam_name: e.name, class_id: cid, class_display: classNames[cid] ?? hit.class_display ?? cid })
    }
  }
  return out
}

export interface StudentGroup {
  class_display: string | null
  students: StudentDto[]
}

/** Students grouped by class (classes in first-seen order of a name-sorted list
 *  → alphabetical by class label; "no class" last), each by roll then name, with
 *  an optional case-insensitive name/admission-no filter. */
export function groupStudents(rows: StudentDto[], query: string): StudentGroup[] {
  const q = query.trim().toLowerCase()
  const hit = (s: StudentDto) => !q || s.name.toLowerCase().includes(q) || (s.admission_no ?? '').toLowerCase().includes(q)
  const groups = new Map<string, StudentDto[]>()
  for (const s of rows.filter(hit)) {
    const k = s.class_display ?? ''
    const g = groups.get(k)
    if (g) g.push(s)
    else groups.set(k, [s])
  }
  return [...groups.entries()]
    .sort(([a], [b]) => (a === '' ? 1 : b === '' ? -1 : a.localeCompare(b, undefined, { numeric: true })))
    .map(([k, students]) => ({
      class_display: k || null,
      students: students.sort((a, b) => (a.roll_no ?? 1e9) - (b.roll_no ?? 1e9) || a.name.localeCompare(b.name)),
    }))
}
