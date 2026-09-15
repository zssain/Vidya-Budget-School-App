# Vidya starter kit — read this first

This kit turns Claude Code into a careful builder for Vidya. It contains the rules the agent must follow, the complete specification, and 44 small, detailed build prompts. You run them one at a time.

## What is in the kit

| Path | What it is | Who reads it |
|---|---|---|
| `AGENTS.md` | Rules for any AI coding agent: sources of truth, never guess APIs, verify everything, never weaken security, end-of-session summary | The agent, every session |
| `CLAUDE.md` | Loads `AGENTS.md` for Claude Code, plus Claude-specific habits | Claude Code, automatically |
| `src/`, `src-tauri/`, `crates/`, `provider/` | Only `AGENTS.md` and `CLAUDE.md` files with rules for that part of the code | The agent, when working there |
| `docs/PRODUCT.md` … `docs/TESTING_STRATEGY.md` | The specification: product, architecture, decisions, allowed libraries, database schema, permissions, commands, sync, license codes, backups, UI, platforms, testing | The agent; you when curious |
| `docs/prompts/P1.1.md` … `P10.6.md` | The 44 build prompts | The agent, through `/run` |
| `docs/PROGRESS.md` | Status of every prompt, checks for you, session log | You and the agent |
| `docs/KNOWN_ISSUES.md` | Anything unfinished or risky | You and the agent |
| `.claude/commands/` | Shortcuts: `/run`, `/verify`, `/finish`, `/status`, `/fix` | You type them in Claude Code |
| `.claude/agents/` | Helper agents: `spec-checker`, `security-reviewer`, `dependency-verifier` | Claude Code uses them |
| `scripts/doctor.sh` | Checks your MacBook has the tools installed | You |
| `reference/VidyaSchoolApp_step1.html` | The working prototype | The agent; you for comparing |
| `VIDYA_PROMPTS_BOOK.md` | All 44 prompts in one file, for reading and printing | You |

## How the kit stops the agent from guessing

1. **One source of truth.** The documents define every table, command, permission and file format. The agent must stop and ask when something is unclear or two documents disagree.
2. **Allowed libraries only.** `docs/DEPENDENCIES.md` lists every library. The agent adds them with `cargo add` or `npm install` (never versions from memory) and records the installed version.
3. **Verify before use.** Before using any library function, config setting or command option, the agent must find it in the installed version's source or docs, and say how it checked. The `dependency-verifier` helper does this.
4. **Automatic drift checks.** Tests fail if the code differs from the documents: the permission table, the database schema file, and the list of commands in Rust, JavaScript and `docs/API.md`.
5. **Small steps with proof.** Each prompt has numbered tasks with checks. `npm run verify` must pass before a prompt is done. The `spec-checker` helper compares the result with the prompt.
6. **Honest endings.** Every session ends with a fixed summary: done, not done, what was verified and how, and what you must check by hand on the MacBook, in the Windows VM and on the phone.
7. **Spikes before risky choices.** Where the right answer is uncertain (for example Hindi in PDFs), a prompt tests options and shows evidence before anything is built.

## Getting started

1. On your MacBook, open Terminal.
2. Create a private GitHub repository called `vidya`, then:
   ```
   mkdir -p ~/code && cd ~/code
   git clone https://github.com/<your-username>/vidya.git
   ```
3. Unzip this kit and copy **everything, including the hidden `.claude` folder**, into `~/code/vidya`. In Finder press Cmd+Shift+. to see hidden files, or use Terminal:
   ```
   cp -R ~/Downloads/vidya-starter-kit/. ~/code/vidya/
   ```
4. Check your tools:
   ```
   cd ~/code/vidya
   ./scripts/doctor.sh
   ```
   Install whatever it marks ✗ under "Core tools". Android tools can wait until P8.1.
5. Save the kit in git:
   ```
   git add -A && git commit -m "Starter kit" && git push
   ```
6. Start Claude Code in the folder:
   ```
   claude
   ```
7. Type `/run P1.1`.

## Your routine for every prompt

1. Start a **fresh** Claude Code session (type `/clear` or restart `claude`).
2. Read the prompt's "Before you start (developer)" section, if it has one, and do those steps first.
3. Type `/run <prompt id>`, for example `/run P2.3`.
4. For prompts marked "Size: large", Claude shows a plan first. Read it and reply "approved" or say what to change.
5. Answer any questions it asks. It is designed to stop and ask rather than guess.
6. When it finishes, read the summary. Do the "Done when (developer)" checks and the new items in `docs/PROGRESS.md`.
7. If something is wrong, type `/fix <what happened in plain words>`, for example `/fix receipt numbers restart at 0001 after reopening the app`.
8. When everything passes, commit:
   ```
   git add -A && git commit -m "P2.3: encrypted database" && git push
   ```
9. Check that GitHub Actions is green (it builds and tests Windows for you).
10. Type `/status` any time to see where you are.

## The phases

| Phase | Prompts | Result |
|---|---|---|
| 1 Foundation | P1.1–P1.3 | Project skeleton, prototype screens in the desktop app, Windows builds from GitHub |
| 2 Core and database | P2.1–P2.7 | Business rules, permissions, encrypted database, sign-in against real data |
| 3 School features | P3.1–P3.7 | Staff logins, students, fees, attendance, marks, settings, reports, Excel |
| 4 Licensing and setup | P4.1–P4.5 | Activation codes, Provider Tool, setup wizard, Excel import |
| 5 Documents | P5.1–P5.2 | Hindi printing proof, all printed documents |
| 6 Backups and year end | P6.1–P6.3 | Local, pen drive and Google Drive backups, new academic session |
| 7 Local network and sync | P7.1–P7.5 | Office computer as server, phone approval, sync engine |
| 8 Android | P8.1–P8.5 | Phone app with offline work and sync |
| 9 Language | P9.1 | Full Hindi, reviewed by a person |
| 10 Release | P10.1–P10.6 | Signed installers, security review, testing, Play Store |

## What you will need, and when

| When | What |
|---|---|
| P1.1 | GitHub account with a private repository; Claude Code |
| P1.3 | App icon, 1024×1024 PNG (a temporary one is made if missing) |
| P1.3 onwards | Windows 11 virtual machine on the MacBook (Parallels or UTM), network set to bridged |
| P4.3 | FileVault switched on; a pen drive; a long passphrase written on paper and stored safely |
| P4.4 | Your business name and support phone number for the recovery sheet |
| P5.2 | Photos of real receipts, report cards, attendance registers and a transfer certificate |
| P6.2 | Google Cloud project with the Drive API and a Desktop OAuth client |
| P7.1 onwards | Home Wi-Fi without guest mode or client isolation |
| P8.1 | Android Studio; an Android phone with USB debugging |
| P9.1 | A Hindi-speaking reviewer (teacher or school clerk) |
| P10.1 | A real Windows PC for final testing; a Windows code signing option |
| P10.2 | Apple Developer Program membership |
| P10.3 | A safe second place to keep the Android upload keystore |
| P10.4 | A lawyer to review the privacy notice and EULA (India DPDP Act) |
| P10.5 | A pilot school for two weeks |
| P10.6 | Play Console organisation account (D-U-N-S number); a website for the privacy policy |

Real fee structures (terms, monthly fees, late fees, sibling discounts) and your final license limits help from P3.3 onwards. Tell the agent when you have them.

## Rules for you

- Never paste passwords, the Provider passphrase, signing keys, keystore passwords or Google client secrets into Claude Code. Prompts tell you where to put them.
- Do not skip prompts or run two at once. Each depends on the ones before it.
- Do not let the agent continue if `npm run verify` fails.
- If the agent asks for a decision, take your time. Decisions are recorded in `docs/DECISIONS.md`.
- Keep `docs/PROGRESS.md` honest. Tick checks only after you have done them.
