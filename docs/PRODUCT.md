# PRODUCT.md — what Vidya does and for whom

## Customers
Small and budget private schools in India (Nursery to class X or XII, 100 to 2,000 students), usually with one office computer, unreliable internet, staff using low-cost Android phones, and English or Hindi as the working language.

## Business model
- One-time payment per school. Full payment before an activation code is issued. No trials, no expiry, no subscriptions.
- One office computer (Windows or Mac) is activated per school. Phones are approved by the principal.
- License limits: maximum active logins (default 40) and maximum approved phones (default 5), set in the activation code.

## People and roles
| Role | Who | Typical device | What they do |
|---|---|---|---|
| Principal | Head of school or owner. Exactly one. Created during setup. | Office computer, own phone | Everything: setup, staff logins, settings, reports, backups, approving phones, correcting records |
| Accountant | Office clerk or accounts staff | Office computer or phone | Admissions, student records, fee collection, receipts, day book |
| Teacher | Class and subject teachers | Own Android phone | Attendance and marks for assigned sections, report cards |

Exact permissions are in `PERMISSIONS.md`.

## Main jobs the product must do well
1. Collect fees and print a receipt in under 30 seconds, with correct balances.
2. Take attendance for a section in under 1 minute on a phone.
3. Enter marks and print report cards for a whole class.
4. Know who owes how much at any moment.
5. Produce the numbers asked for in UDISE+ forms and inspections.
6. Never lose data: encrypted backups on the computer, external or pen drives and a school-network folder.
7. Work with no internet at all.

## Out of scope for version 1
Parent app, online payments, SMS gateway, timetable, library, transport routes, payroll, iPhone app, multi-branch schools, web version.

## Glossary
| Term | Meaning |
|---|---|
| Session | Academic year, usually April to March, written like 2026-27 |
| Class / section | Class is the grade (Nursery, LKG, UKG, I to XII). Section is the division (A, B). Stored as class + section; shown as "V-A" |
| Roll number | Student's number within a section, 1 upward |
| Admission number | School-wide permanent number, never reused. Shown as ADM/0001 |
| RTE | Right to Education Act, Section 12(1)(c): free seats for disadvantaged children. RTE students pay no fee |
| UDISE+ / UDISE code | Government school database; each school has an 11-digit UDISE code |
| APAAR ID | Government student ID ("One Nation One Student ID") |
| Aadhaar | Government identity number. Vidya stores only whether it was collected, never the number |
| TC | Transfer certificate, issued when a student leaves |
| Term | Fee instalment period; a session has 1, 2, 3, 4 or 12 terms |
| Concession | Fee reduction for a student for the session, principal only |
| Day book | List and totals of all money received on a date |
| Category | General, OBC, SC, ST (used in government reports) |
| Board | State Board, CBSE, ICSE, etc. |
| Unit test, half yearly, annual | Common exam names |
| AB | Absent in an exam |

## Language and tone
Plain, short, friendly English and simple everyday Hindi. Staff may have little computer experience. Every error says what went wrong and how to fix it.
