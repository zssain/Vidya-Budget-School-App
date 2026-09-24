# Vidya Budget School — Principal's Guide

This guide is for you, the Principal. It explains, in plain words, everything you
look after: activating the school, keeping the recovery key safe, inviting your
staff, setting up Google Drive, approving requests, handling conflicts and review
flags, backups, moving to a new PC, dealing with a lost phone, starting a new
session, the offline access period, and — honestly — what the audit record can and
cannot prove.

You do not need to be technical to use Vidya. If a screen ever says something needs
your attention, it will also tell you exactly what to do next. Vidya never pretends
something worked when it did not — if it says "Confirmed", "Backed up" or "Printed",
it really happened.

A few words used throughout:

- **School server** — the one main PC where Vidya is installed and where the
  official school record lives. Keep it on during school hours.
- **Device** — any phone or PC your staff use to reach the school server.
- **Confirmed** — a change that the school server has accepted and recorded.
- **Request** — a change to locked data (a submitted attendance sheet, submitted
  marks, or someone else's record) that needs your approval.
- **Session** — one school year, April to March, shown like "2026–27".

---

## 1. Activating the school (entering the licence code)

When you buy Vidya you receive an **activation code** that looks like
`VIDYA-XXXX-XXXX-XXXX`. You enter it once, on the main PC that will become the
school server.

1. Install Vidya on the main school PC (see `docs/INSTALL.md`).
2. Open Vidya. On the Welcome screen, choose **Set up a new school**.
3. Type your activation code and your school's name, then continue.
4. Vidya needs the internet **just for this one step**. If it cannot reach the
   activation service, you'll see "Activation needs internet once — please try
   again." Connect and try again.

Good to know:

- The licence is a **one-time purchase and never expires**. There is no monthly
  fee, no trial and no renewal.
- Each code works for **one school on one PC**. If you enter it on a second machine
  you'll see "This code has already been used by another school." (Moving your
  school to a new PC later is different — see **Section 10**.)
- After activation, Vidya keeps working even with no internet for a long time. It
  only quietly re-checks the licence about once a month when it happens to be
  online.

---

## 2. Your recovery key — keep it safe

Right after activation, during setup, Vidya shows you a **recovery key** once. It
is 30 characters in six groups of five, for example:

```
K7M2Q  9XR4T  BW8HN  3PYC6  VD5FK  Q2M9J
```

Vidya asks you to type two of the groups back to confirm you wrote it down.

**Why it matters.** Your school's data is encrypted. The recovery key is the master
key that lets you:

- **Move the school to a new PC** if the old one dies or is replaced.
- **Restore your data from a backup**.

Without it, nobody — not you, not your staff, not the company — can open a backup or
move the school. There is no "forgot my key" button. This is deliberate: it is what
keeps your students' and parents' data private.

**Where to store it.**

- Write it on paper and lock it somewhere safe (a school safe or a locked drawer).
- Keep a **second copy** in a different safe place, in case the first is lost.
- Do **not** keep it only on the school PC, and do not email or WhatsApp it to
  yourself — that defeats the purpose.
- Consider giving a sealed copy to a trusted trustee or the school owner.

Vidya shows the recovery key **only once**, at setup. It is never stored on the PC
in a way anyone can read, so please capture it carefully at that moment.

---

## 3. Inviting your staff

Your accountant and teachers join by invitation. You create an invite for each
person from **Staff & access**.

1. Open **Staff & access** and add the staff member (name, role, mobile).
2. Choose **Invite**. Vidya gives you three ways to share the same invitation:
   - a **link** you can send them,
   - an **8-character code** they can type,
   - a **QR code** they can look at.
3. The staff member installs Vidya, chooses **Join a school**, and pastes the link
   or types the code.

Good to know:

- An invitation is **single-use** and expires after **72 hours**. If it lapses,
  just create a new one.
- When someone joins, their device automatically gets its own **receipt and
  admission number series** so numbers never clash, even offline.
- **Roles decide what people can do.** Accountants handle admissions and fees.
  Teachers take attendance and enter marks for their own classes. Teachers never
  see fee data. You see everything and approve corrections.
- You can **suspend** a staff member (they can't sign in or sync, but their data is
  kept) or **remove** them. Only you can change these.

---

## 4. Setting up Google Drive (optional but recommended)

Google Drive gives your school two things: a safe off-site copy of your **backups**,
and a way for devices to **exchange changes** when the school PC happens to be off.
It is set up from the school server PC, using **your** Google account.

1. Open **Settings → Google Drive** on the school server.
2. Sign in with the school's Google account (your browser opens for this).
3. Vidya creates a private `Vidya` folder in your Drive and organises it for you.

How Vidya keeps it private and safe:

- **Backups are never shared** — only your Google account can see them.
- Each staff member can write only to **their own** exchange folder. A teacher can
  never delete a backup or read another class's data.
- **Everything is encrypted before it leaves the device.** Google cannot read any
  of your school's data.

If your Drive is full, the connection is lost, or the folder is removed, Vidya shows
a clear "Needs attention" note on Home with the exact fix. Not every staff member
needs a Google account — those without one simply don't use the Drive exchange, and
Vidya says so.

---

## 5. Approvals — your requests inbox

Some data is **locked** so it can't be changed casually: a submitted attendance
sheet, submitted marks for a subject, and edits to records the person doesn't own.
When staff need such a change, they send you a **request**. You'll see them in
**Approvals**.

Each request shows you plainly **what it wants to change** — the value before and
the value after — and the reason given. For each one you can:

- **Approve** — the change is applied and recorded.
- **Reject** — nothing changes.
- **Return** — send it back with a note asking for more detail.

Request types you may see: attendance correction, marks correction, student-details
edit, payment reversal, an access change, or a device replacement. Only you can
decide them. A teacher cannot approve their own request.

---

## 6. Conflicts — when two devices disagree

Vidya **never silently overwrites** anyone's work. If two devices change the **same
field of the same record** from the same starting point — for example, a student's
address edited on two phones — Vidya keeps the current value, records a **conflict**,
and tells you: "Riya Verma · address changed on two phones."

You resolve it from **Conflict review** by choosing the correct value. A few things
worth knowing:

- If two people change **different** fields of the same record, both changes are
  kept — that's not a conflict.
- **Attendance** conflicts are compared student by student; only the students marked
  differently are flagged.
- **Money never causes a conflict.** A real payment is always kept (see Section 7).

---

## 7. Review flags — money that needs a look

Payments are special: **money actually received is never rejected or lost.** But
some situations deserve your eye, so Vidya raises a **review flag** and leaves the
receipt standing until you decide. You'll find these on Home and in your inbox.

- **Overpayment** — a payment came in for more than was due (for example, two
  devices collected before they'd synced). The receipt stands; the extra is held as
  **advance credit** for that student, and you're flagged to review it.
- **Possible duplicate** — the same student paid the same amount with the same
  UPI/cheque reference, or the same amount twice within ten minutes on different
  devices. Both payments stay until you decide.
- **Revoked author** — a change arrived from a device or staff member whose access
  had been taken away. It is **not** applied automatically; it waits for you.

To correct a genuine mistake, you request a **reversal** — payments are never edited
or deleted, only reversed with an approved reason. This keeps the money trail
honest.

---

## 8. Backups — daily, verified, and copied

Vidya backs up your school automatically. You don't have to remember to do it.

- A backup runs **every day at 6:00 AM** on the school server (and again when you
  quit, if none ran that day).
- Each backup is **encrypted**, then **verified** — Vidya reopens it, checks it is
  whole, counts the records and checks the audit record — before it counts as done.
- If you've set up Google Drive, a copy is **uploaded and checked** there too.
- Vidya keeps the last **30 daily** backups and **12 monthly** ones, in each place.

Home shows the honest status, for example: "Verified today at 6:02 AM · this PC and
school Drive." If a backup ever fails, Home shows a **Needs attention** note instead
— never a false "done".

**Keep a copy on a USB drive.** Once in a while, copy the newest backup file from
the school server's `Vidya\backups` folder onto a USB stick and keep it off-site.
It is your safety net if the PC and the Drive are ever both unavailable. Remember:
the file is encrypted, so you'll need your **recovery key** (Section 2) to open it.

---

## 9. Starting a new session (rollover and carry-forward)

At the start of a new school year you run the **new session** step. Vidya does the
heavy lifting in one careful action:

- Creates the **new session** (for example "2027–28") and its terms, and marks the
  old session **read-only** so its records can't be changed by accident.
- **Promotes** each student to the next class (Nursery → LKG → … → XII), keeping
  their section. You can mark individual students to **repeat** or as **left**.
- **Carries forward** any unpaid balance a student still owed as a single
  "Previous balance" due in the new year's first term (the old dues are kept, just
  linked).
- Generates the new session's term fees.

Nothing is lost and the old year stays intact and viewable. This step is yours
alone to run.

---

## 10. Moving to a new PC (restore)

If the school PC dies, is stolen, or you simply want a better machine, you move the
school by **restoring from a backup** onto the new PC. You'll need your **recovery
key**.

1. Install Vidya on the new PC.
2. On the Welcome screen choose **Recover**.
3. Pick the backup — a file (for example, from your USB copy) or one from your
   Google Drive after signing in.
4. Enter your **recovery key** to unlock it.
5. Vidya shows a summary — school name, backup date, number of students, payments,
   last receipt number, and that the audit record checks out — for you to confirm.
6. Vidya moves the licence to this PC, sets fresh security keys, and makes this the
   new school server.

After a restore, **every staff device must join again** (this is a safety measure so
an old, lost machine can never keep acting as your server). Home will show "Devices
need to join again" and prompt you to **invite all staff again**. Your staff keep any
unsent work on their phones and it syncs up once they've re-joined. The old PC, if it
ever comes back online, will show "This computer is no longer the school server" and
can no longer act as one.

---

## 11. A lost or stolen phone (revoke the device)

If a staff phone or PC is lost or stolen, remove its access from **Sync & devices**:

1. Open **Sync & devices**.
2. Find the device and choose **Revoke**.

What this does — and its honest limit:

- Revocation takes effect the **next time that device tries to reach your school**
  (over Wi-Fi, the internet, or Google Drive). From that moment it is refused and
  can no longer sync.
- Vidya **cannot remotely wipe** a phone that stays switched off and offline. No app
  can, and Vidya will never pretend otherwise. The important protection is that the
  device is **encrypted and PIN-locked**: without the person's PIN, the data on it
  can't be opened, and once revoked it can never sync again or pull new data.

To give that staff member a working device, invite them again (Section 3) or approve
a **device replacement** request.

---

## 12. The offline access period (30 days)

Vidya is built to work **offline**. Every device keeps working with no network, and
nothing is ever lost — work is saved on the device and sent later.

There is one sensible limit: each device has a **30-day offline access period**,
counted from the last time it reached your school server. Within those 30 days it
works normally offline. After 30 days without any contact, the app still opens and
still keeps any unsent work, but it shows **"Connect to your school to continue"** and
hides school data until the device reaches the school server again. This makes sure a
device that has drifted far out of touch checks back in before showing potentially
stale information. As soon as it reconnects, the period resets and everything is
available again.

---

## 13. The honest truth about the audit record

Vidya keeps an **audit record**: an append-only, hash-chained log of every important
change — who did what, when, and the before/after values. Each entry is locked to the
one before it, so the record is **tamper-evident**: if any past entry were altered or
removed, the chain no longer matches. Vidya checks the chain **every day and before
every backup**, and stores the chain's fingerprint with each backup and in
**Settings → About**. Not even you, the Principal, can edit the log — the database
itself refuses to change or delete audit, payment or reversal records.

Here is the honest part, in plain words:

- The audit record is very good at **catching accidental or casual tampering**, and
  at proving that your day-to-day records are consistent and unchanged.
- It is **not** an absolute, unbreakable proof against a determined expert. A person
  who has **full control of the PC and all the keys** could, in principle, rebuild the
  whole chain from scratch to hide a change.
- What makes that **detectable** is exactly why the backups and Drive copies matter:
  because the chain's fingerprint is stored with **every backup** (on this PC, on
  Google Drive, and on your USB copy), a rebuilt chain would **no longer match** those
  earlier saved fingerprints. Someone would have to alter every copy everywhere, at
  the same time, to get away with it.

So keep your backups in more than one place (PC, Drive, USB) — that is what turns
"tamper-evident" into "very hard to tamper with unnoticed". Vidya does not claim to be
magically unbreakable; it claims to be honest, and to make wrongdoing show up.

---

## Where to go when something needs attention

Whenever Vidya needs you, it says so plainly on **Home** under **Needs attention**,
and tells you the exact step to fix it — a failed backup, a full Google Drive, a
disconnected account, a pending approval, a conflict, or a review flag. Vidya never
hides a problem behind a false "everything's fine".

For anything this guide doesn't cover, contact [Company name] support at
[support contact].
