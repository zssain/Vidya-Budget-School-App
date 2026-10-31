import { describe, expect, it } from 'vitest'
import type { ExamDto, MarksSheetDto } from './api'
import { cellsFromSheet, entriesFromCells, groupStudents, marksOptions, remarkOptions } from './teacherPhone'
import teacherHome from './i18n/strings/teacherHome'
import teacherPhone from './i18n/strings/teacherPhone'

const sub = (id: string, cs: string, cls: string, name: string, status = 'not_started') => ({
  id, class_subject_id: cs, class_id: cls, class_display: cls.toUpperCase(), subject_name: name, max_marks: 25, status,
})
const exams: ExamDto[] = [
  { id: 'x2', name: 'Half-yearly', term_name: null, starts_on: '2026-10-01', ends_on: null, subjects: [sub('es3', 'cs-5a-eng', 'c5a', 'English')] },
  { id: 'x1', name: 'Unit test 1', term_name: null, starts_on: '2026-07-01', ends_on: null, subjects: [
    sub('es1', 'cs-5a-eng', 'c5a', 'English', 'submitted'),
    sub('es2', 'cs-5a-maths', 'c5a', 'Maths'),
    sub('es4', 'cs-7b-eng', 'c7b', 'English'),
  ] },
]

describe('marks options', () => {
  it('only the class subjects the teacher teaches, newest exam first', () => {
    expect(marksOptions(exams, ['cs-5a-eng']).map((o) => [o.exam_id, o.subject.id])).toEqual([['x2', 'es3'], ['x1', 'es1']])
    expect(marksOptions(exams, [])).toEqual([])
  })
})

describe('marks grid', () => {
  const sheet: MarksSheetDto = {
    exam_subject_id: 'es1', class_id: 'c5a', class_display: 'V-A', subject_name: 'English', max_marks: 25, status: 'draft',
    rows: [
      { student_id: 's1', name: 'Asha', roll_no: 1, marks: 20, absent: false },
      { student_id: 's2', name: 'Bilal', roll_no: 2, marks: null, absent: true },
      { student_id: 's3', name: 'Chitra', roll_no: 3, marks: null, absent: false },
    ],
  }
  it('round-trips the sheet; blanks are not sent, absent carries no marks', () => {
    const cells = cellsFromSheet(sheet)
    expect(cells.map((c) => c.value)).toEqual(['20', '', ''])
    expect(entriesFromCells(cells, 25)).toEqual({
      entries: [
        { student_id: 's1', marks: 20, absent: false },
        { student_id: 's2', marks: null, absent: true },
      ],
      invalid: [],
    })
  })
  it('flags values outside 0..max', () => {
    const cells = cellsFromSheet(sheet).map((c) => (c.student_id === 's3' ? { ...c, value: '26' } : c))
    expect(entriesFromCells(cells, 25).invalid).toEqual(['s3'])
    const bad = cellsFromSheet(sheet).map((c) => (c.student_id === 's1' ? { ...c, value: '1.5' } : c))
    expect(entriesFromCells(bad, 25).invalid).toEqual(['s1'])
  })
})

describe('remark options', () => {
  it('exam × the classes I am class teacher of, where the exam covers the class', () => {
    expect(remarkOptions(exams, ['c5a'], { c5a: 'V-A' })).toEqual([
      { exam_id: 'x2', exam_name: 'Half-yearly', class_id: 'c5a', class_display: 'V-A' },
      { exam_id: 'x1', exam_name: 'Unit test 1', class_id: 'c5a', class_display: 'V-A' },
    ])
    expect(remarkOptions(exams, ['c7b'], {}).map((o) => o.exam_id)).toEqual(['x1'])
    expect(remarkOptions(exams, [], {})).toEqual([])
  })
})

describe('students grouping', () => {
  const rows = [
    { id: 's1', name: 'Bilal', admission_no: 'A-2', provisional_no: null, class_display: 'V-A', roll_no: 2 },
    { id: 's2', name: 'Asha', admission_no: 'A-1', provisional_no: null, class_display: 'V-A', roll_no: 1 },
    { id: 's3', name: 'Dev', admission_no: null, provisional_no: null, class_display: null, roll_no: null },
    { id: 's4', name: 'Chitra', admission_no: null, provisional_no: null, class_display: 'II-A', roll_no: 4 },
  ]
  it('groups by class (no class last), by roll inside', () => {
    expect(groupStudents(rows, '').map((g) => [g.class_display, g.students.map((s) => s.id)])).toEqual([
      ['II-A', ['s4']],
      ['V-A', ['s2', 's1']],
      [null, ['s3']],
    ])
  })
  it('filters by name or admission number', () => {
    expect(groupStudents(rows, 'bil').flatMap((g) => g.students.map((s) => s.id))).toEqual(['s1'])
    expect(groupStudents(rows, 'a-1').flatMap((g) => g.students.map((s) => s.id))).toEqual(['s2'])
  })
})

describe('teacher phone strings', () => {
  it('Telugu covers every key of the new modules', () => {
    for (const b of [teacherHome, teacherPhone]) {
      expect(Object.keys(b.te ?? {}).sort()).toEqual(Object.keys(b.en).sort())
    }
  })
})
