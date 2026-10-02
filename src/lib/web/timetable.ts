// The teacher's timetable (Phase 20, C3) — a faithful TS port of the Rust
// my_timetable_logic + SLOT_SELECT (src-tauri/src/commands/logic.rs) over the records
// Drive sync populates locally. Pure (records in → DTO out) → unit-tested; the backend
// handler is a thin store-read shell. The Rust query INNER-JOINs class_subject, subject
// and class, so a slot whose refs haven't synced is dropped — matched here.

import type { PeriodDto, TeacherTimetableDto, TimetableSlotDto } from '../api'

export interface TimetableSlotRec {
  id: string
  class_id: string
  weekday: number
  period_no: number
  class_subject_id: string
  teacher_id: string
  effective_to?: string | null
}
export interface ClassRec {
  id: string
  display?: string | null
}
export interface ClassSubjectRec {
  id: string
  subject_id: string
}
export interface SubjectRec {
  id: string
  name: string
}
export interface PeriodRec {
  id: string
  no: number
  starts_at: string
  ends_at: string
  session_id: string
}

export interface TeacherTimetableInput {
  staffId: string
  staffName: string
  sessionId: string | null
  slots: TimetableSlotRec[]
  classes: ClassRec[]
  classSubjects: ClassSubjectRec[]
  subjects: SubjectRec[]
  periods: PeriodRec[]
}

export function buildTeacherTimetable(input: TeacherTimetableInput): TeacherTimetableDto {
  const classById = new Map(input.classes.map((c) => [c.id, c]))
  const csById = new Map(input.classSubjects.map((cs) => [cs.id, cs]))
  const subById = new Map(input.subjects.map((s) => [s.id, s]))

  const slots: TimetableSlotDto[] = input.slots
    .filter((ts) => ts.teacher_id === input.staffId && ts.effective_to == null)
    .map((ts) => ({ ts, cs: csById.get(ts.class_subject_id), cls: classById.get(ts.class_id) }))
    // INNER JOIN: drop a slot whose class / class_subject / subject isn't present.
    .filter((j): j is { ts: TimetableSlotRec; cs: ClassSubjectRec; cls: ClassRec } =>
      j.cs != null && j.cls != null && subById.has(j.cs.subject_id),
    )
    .map(({ ts, cs, cls }) => ({
      id: ts.id,
      class_id: ts.class_id,
      class_display: cls.display ?? null,
      weekday: ts.weekday,
      period_no: ts.period_no,
      class_subject_id: ts.class_subject_id,
      subject_name: subById.get(cs.subject_id)!.name,
      teacher_id: ts.teacher_id,
      teacher_name: input.staffName,
    }))
    .sort((a, b) => a.weekday - b.weekday || a.period_no - b.period_no)

  // Periods belong to the current session; none synced / no session → empty.
  const periods: PeriodDto[] =
    input.sessionId == null
      ? []
      : input.periods
          .filter((p) => p.session_id === input.sessionId)
          .sort((a, b) => a.no - b.no)
          .map((p) => ({ id: p.id, no: p.no, starts_at: p.starts_at, ends_at: p.ends_at }))

  return { periods, slots }
}
